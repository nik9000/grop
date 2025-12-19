/// A list of [ChunkList]s.
use crate::{ChunkList, ChunkListBuilder, ChunkListRef};
use std::{fmt, io};

/// An "inventory" of [ChunkList]s.
pub struct ChunkInventoryBuilder {
  builders: Vec<ChunkListBuilder>,
}

impl ChunkInventoryBuilder {
  pub fn get(&mut self, index: usize) -> &mut ChunkListBuilder {
    &mut self.builders[index]
  }

  pub fn next(&mut self) -> usize {
    let i = self.builders.len();
    self.builders.push(ChunkList::builder());
    i
  }

  pub fn build(self) -> ChunkInventory {
    ChunkInventory {
      chunks: self
        .builders
        .into_iter()
        .map(ChunkListBuilder::build)
        .collect(),
    }
  }
}

/// List of [ChunkList]s.
///
/// Serialized as:
/// ```text
/// number_of_entries as u32
/// end_of_first as u32
/// end_of_second as u32
/// ...
/// end_of_n as u32
/// list_0
/// list_1
/// ...
/// list_n
/// ```
pub struct ChunkInventory {
  chunks: Vec<ChunkList>,
}

impl ChunkInventory {
  pub fn builder() -> ChunkInventoryBuilder {
    ChunkInventoryBuilder { builders: vec![] }
  }

  pub fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    let count = self.chunks.len() as u32;
    w.write_all(&count.to_be_bytes())?;
    let mut end = 0;
    for c in self.chunks.iter() {
      end += c.len() as u32;
      w.write_all(&end.to_be_bytes())?;
    }
    for c in self.chunks.iter() {
      c.write(w)?;
    }
    Ok(())
  }
}

pub struct ChunkInventoryRef<'a> {
  offsets: &'a [u8],
  lists: &'a [u8],
}

impl<'a> ChunkInventoryRef<'a> {
  pub fn from(bytes: &'a [u8]) -> (Self, usize) {
    // NOCOMMIT return error if bad shape
    let starts_start = size_of::<u32>();
    let len = u32::from_be_bytes(bytes[0..starts_start].try_into().unwrap()) as usize;
    let starts_end = starts_start + len * size_of::<u32>();
    let offsets = &bytes[starts_start..starts_end];

    let lists_end = starts_end + Self::read_offset(offsets, len - 1);
    let lists: &[u8] = &bytes[starts_end..lists_end];
    (ChunkInventoryRef { offsets, lists }, lists_end)
  }

  pub fn len(&self) -> usize {
    self.offsets.len() / size_of::<u32>()
  }

  /// Number of bytes used by the inventory.
  pub fn size_of(&self) -> usize {
    self.offsets.len() + self.lists.len()
  }

  pub fn get(&self, index: usize) -> ChunkListRef<'a> {
    let start = if index == 0 {
      0
    } else {
      Self::read_offset(self.offsets, index - 1)
    };
    let end: usize = Self::read_offset(self.offsets, index);
    ChunkListRef::from(&self.lists[start..end])
  }

  #[inline]
  fn read_offset(offsets: &[u8], index: usize) -> usize {
    let start = index * size_of::<u32>();
    let end = start + size_of::<u32>();
    u32::from_be_bytes(offsets[start..end].try_into().unwrap()) as usize
  }
}

impl<'a> fmt::Debug for ChunkInventoryRef<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut l = f.debug_list();
    for i in 0..self.len() {
      l.entry(&self.get(i));
    }
    l.finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_inventory() -> ChunkInventory {
    let mut b = ChunkInventory::builder();
    assert_eq!(0, b.next());
    b.get(0).add(0);
    assert_eq!(1, b.next());
    b.get(1).add(1);
    assert_eq!(2, b.next());
    b.get(2).add(0);
    b.get(2).add(2);

    b.get(0).add(3);
    b.get(1).add(3);
    b.get(2).add(3);

    b.get(1).add(4);

    b.build()
  }

  #[test]
  fn build() {
    let i = test_inventory();
    assert_eq!(
      vec![0, 3],
      i.chunks[0]
        .as_ref()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    );
    assert_eq!(
      vec![1, 3, 4],
      i.chunks[1]
        .as_ref()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    );
    assert_eq!(
      vec![0, 2, 3],
      i.chunks[2]
        .as_ref()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    );
  }

  #[test]
  fn read_write() {
    let mut bytes = vec![];
    let orig = test_inventory();
    orig.write(&mut bytes).unwrap();
    assert_eq!(
      bytes.len(),
      4           // length
      + 3 * 4     // offsets
      + 2 + 3 + 3 // lists
    );

    let read = ChunkInventoryRef::from(&bytes[..]).0;
    assert_eq!(3, read.len());
    assert_eq!(
      vec![0, 2, 3],
      read
        .get(2)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    );
  }

  #[test]
  fn debug() {
    let mut bytes = vec![];
    let orig = test_inventory();
    orig.write(&mut bytes).unwrap();
    let read = ChunkInventoryRef::from(&bytes[..]).0;
    assert_eq!(
      r#"[
    [
        0,
        3,
    ],
    [
        1,
        3,
        4,
    ],
    [
        0,
        2,
        3,
    ],
]"#,
      format!("{:#?}", read)
    );
  }
}
