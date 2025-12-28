mod chunk_ends;
mod chunk_inventory;
mod chunk_list;
mod from_lines;
mod trigram_map;

use chunk_ends::*;
use chunk_inventory::*;
use chunk_list::*;
use int::DecodeError;
use std::{fmt, io};
use trigram_map::*;

pub use chunk_list::ChunkListRef;

#[derive(thiserror::Error, Debug)]
pub enum ReadError<'a> {
  #[error("wrong magic bytes: {0:x?}")]
  BadMagic(&'a [u8]),
  #[error("no version: {0:x?}")]
  NoVersion(&'a [u8]),
  #[error("version decode error: {0:?}")]
  VersionDecodeError(DecodeError),
  #[error("wrong version: expected {0} but was {1}")]
  WrongVersion(u64, u64),
  #[error("not enough bytes for chunk_ends: {0}")]
  InvalidChunkEnds(usize),
  #[error(transparent)]
  IO(#[from] io::Error),
}

pub struct DatabaseBuilder {
  map: InnerLayerBuilder<Box<InnerLayerBuilder<Box<LeafLayerBuilder>>>>,
  inventory: ChunkInventoryBuilder,
  chunk_end_offsets: ChunkEndsBuilder,
  chunk_end_line_counts: ChunkEndsBuilder,
}

impl DatabaseBuilder {
  pub fn new() -> DatabaseBuilder {
    DatabaseBuilder {
      map: three_layer_builder(),
      inventory: ChunkInventory::builder(),
      chunk_end_offsets: ChunkEndsBuilder::new(),
      chunk_end_line_counts: ChunkEndsBuilder::new(),
    }
  }

  pub fn add_trigram<'s>(&mut self, trigram: [u8; 3], chunk: u64) {
    self
      .map
      .get(trigram[0])
      .get(trigram[1])
      .get(&mut self.inventory, trigram[2])
      .add(chunk);
  }

  pub fn add_chunk_end(&mut self, end_offset: u32, end_line_count: u32) {
    self.chunk_end_offsets.add(end_offset);
    self.chunk_end_line_counts.add(end_line_count);
  }

  pub fn write<W: io::Write>(self, w: &mut W) -> io::Result<()> {
    w.write_all(&"grop".as_bytes())?;
    w.write(&[0])?;
    w.write_all(&(self.map.written_len() as u32).to_be_bytes())?;
    self.map.write(w)?;
    self.inventory.build().write(w)?;
    self.chunk_end_offsets.write(w)?;
    self.chunk_end_line_counts.write(w)?;
    Ok(())
  }
}

pub struct DatabaseRef<'a> {
  map: InnerLayerRef<'a, InnerLayerRef<'a, LeafLayerRef<'a>>>,
  inventory: ChunkInventoryRef<'a>,
  chunk_end_offsets: ChunkEndsRef<'a>,
  chunk_end_line_counts: ChunkEndsRef<'a>,
}

impl<'a> DatabaseRef<'a> {
  pub fn from(bytes: &'a [u8]) -> Result<Self, ReadError<'a>> {
    // NOCOMMIT return error if bad shape
    // NOCOMMIT chunk locations in original file
    // NOCOMMIT fail if partially written

    let magic_end = "grop".as_bytes().len();
    if &bytes[..magic_end] != "grop".as_bytes() {
      return Err(ReadError::BadMagic(&bytes[..magic_end]));
    }
    let version = int_variable::iter(bytes[magic_end..].iter().copied()).next();
    let Some(version) = version else {
      return Err(ReadError::NoVersion(&bytes[magic_end..]));
    };
    let version = version.map_err(ReadError::VersionDecodeError)?;
    if 0 != version {
      return Err(ReadError::WrongVersion(0, version));
    }

    let version_end = magic_end + 1;
    let map_len_end = version_end + size_of::<u32>();
    let map_len = u32::from_be_bytes(bytes[version_end..map_len_end].try_into().unwrap()) as usize;

    let map_end = map_len_end + map_len;
    let map = three_layer_ref_from_bytes(&bytes[map_len_end..map_end]);

    let bytes = &bytes[map_end..];
    let (inventory, inventory_end) = ChunkInventoryRef::from(bytes);
    let bytes = &bytes[inventory_end..];
    let (chunk_end_offsets, chunk_end_offsets_end) = ChunkEndsRef::from(bytes)?;
    let bytes = &bytes[chunk_end_offsets_end..];
    let (chunk_end_line_counts, chunk_end_line_counts_ends) = ChunkEndsRef::from(bytes)?;
    assert_eq!(chunk_end_line_counts_ends, bytes.len());

    Ok(DatabaseRef {
      map,
      inventory,
      chunk_end_offsets,
      chunk_end_line_counts,
    })
  }

  pub fn chunks_containing(&self, trigram: &[u8; 3]) -> Option<ChunkListRef<'a>> {
    let chunk_list = self.map.get(trigram[0])?.get(trigram[1])?.get(trigram[2])?;
    Some(self.inventory.get(chunk_list))
  }

  pub fn chunks_containing_by_ord(&self, id: usize) -> ChunkListRef<'a> {
    self.inventory.get(id)
  }

  pub fn chunk_end_offset(&self, chunk: u32) -> u32 {
    self.chunk_end_offsets.get(chunk)
  }

  pub fn chunk_end_line_count(&self, chunk: u32) -> u32 {
    self.chunk_end_line_counts.get(chunk)
  }

  /// Number of chunks indexed.
  pub fn chunk_count(&self) -> u32 {
    self.chunk_end_offsets.len() as u32
  }

  /// Number of trigrams in the file.
  pub fn trigram_count(&self) -> usize {
    self.inventory.len()
  }

  /// Number of consumed by the map.
  pub fn map_size(&self) -> usize {
    self.map.size_of()
  }

  /// Number of bytes used by the inventory.
  pub fn inventory_size(&self) -> usize {
    self.inventory.size_of()
  }

  /// Number of bytes used by the chunk ends.
  pub fn chunk_end_offsets_size(&self) -> usize {
    self.chunk_end_offsets.size_of()
  }

  /// Number of bytes used by the chunk lin counts.
  pub fn chunk_end_line_counts_size(&self) -> usize {
    self.chunk_end_line_counts.size_of()
  }
}

impl<'a> fmt::Debug for DatabaseRef<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DB")
      .field("map", &self.map)
      .field("inventory", &self.inventory)
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_db() -> DatabaseBuilder {
    let mut builder = DatabaseBuilder::new();
    builder.add_trigram("abc".as_bytes().try_into().unwrap(), 0);
    builder.add_trigram("bcd".as_bytes().try_into().unwrap(), 0);
    builder.add_trigram("cdj".as_bytes().try_into().unwrap(), 0);
    builder.add_chunk_end(99, 1);

    builder.add_trigram("abc".as_bytes().try_into().unwrap(), 1);
    builder.add_trigram("bcd".as_bytes().try_into().unwrap(), 1);
    builder.add_trigram("cdz".as_bytes().try_into().unwrap(), 1);
    builder.add_chunk_end(200, 3);

    builder
  }

  #[test]
  fn test() {
    let mut bytes = vec![];
    test_db().write(&mut bytes).unwrap();

    assert_eq!(125, bytes.len());
    let db = DatabaseRef::from(&bytes).unwrap();
    assert_eq!(None, db.chunks_containing_as_vec("aaa"));
    assert_eq!(Some(vec![0, 1]), db.chunks_containing_as_vec("abc"));
    assert_eq!(Some(vec![0, 1]), db.chunks_containing_as_vec("bcd"));
    assert_eq!(Some(vec![0]), db.chunks_containing_as_vec("cdj"));
    assert_eq!(Some(vec![1]), db.chunks_containing_as_vec("cdz"));

    assert_eq!(99, db.chunk_end_offset(0));
    assert_eq!(200, db.chunk_end_offset(1));

    assert_eq!(1, db.chunk_end_line_count(0));
    assert_eq!(3, db.chunk_end_line_count(1));
  }

  impl<'a> DatabaseRef<'a> {
    pub fn chunks_containing_as_vec(&self, trigram: &'static str) -> Option<Vec<u64>> {
      Some(
        self
          .chunks_containing(trigram.as_bytes().try_into().unwrap())?
          .into_iter()
          .collect::<Result<Vec<_>, _>>()
          .unwrap(),
      )
    }
  }

  #[test]
  fn debug() {
    let mut bytes = vec![];
    test_db().write(&mut bytes).unwrap();
    let db = DatabaseRef::from(&bytes).unwrap();
    assert_eq!(
      r#"DB {
    map: {
        "abc": 0,
        "bcd": 1,
        "cdj": 2,
        "cdz": 3,
    },
    inventory: [
        [
            0,
            1,
        ],
        [
            0,
            1,
        ],
        [
            0,
        ],
        [
            1,
        ],
    ],
}"#,
      format!("{:#?}", db)
    );
  }
}
