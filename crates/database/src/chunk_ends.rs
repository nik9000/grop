use std::io;

use crate::ReadError;

pub(crate) struct ChunkEndsBuilder { // NOCOMMIT rename me - it's just a u32 per chunk
  ends: Vec<u32>,
}

impl ChunkEndsBuilder {
  pub fn new() -> Self {
    ChunkEndsBuilder { ends: vec![] }
  }

  pub fn add(&mut self, end: u32) {
    self.ends.push(end);
  }

  pub fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    assert!(
      self.ends.len() <= u32::MAX as usize,
      "no more than u32 chunks allowed"
    );
    w.write_all(&(self.ends.len() as u32).to_be_bytes())?;
    for e in self.ends.iter() {
      w.write_all(&e.to_be_bytes())?;
    }
    Ok(())
  }
}

pub(crate) struct ChunkEndsRef<'a> {
  ends: &'a [u8],
}

impl<'a> ChunkEndsRef<'a> {
  pub(crate) fn from(bytes: &'a [u8]) -> Result<(Self, usize), ReadError<'a>> {
    // TODO byte alignment
    if bytes.len() < size_of::<u32>() {
      return Err(ReadError::InvalidChunkEnds(bytes.len()));
    }

    let len = u32::from_be_bytes(bytes[..size_of::<u32>()].try_into().unwrap()) as usize;
    let start = size_of::<u32>();
    let end = start + len * size_of::<u32>();
    if bytes.len() < end {
      return Err(ReadError::InvalidChunkEnds(bytes.len()));
    }
    Ok((
      ChunkEndsRef {
        ends: &bytes[start..end],
      },
      end,
    ))
  }

  pub(crate) fn get(&self, chunk: u32) -> u32 {
    let start = chunk as usize * size_of::<u32>();
    let end = start + size_of::<u32>();
    u32::from_be_bytes(self.ends[start..end].try_into().unwrap())
  }

  pub(crate) fn len(&self) -> usize {
    self.ends.len() / size_of::<u32>()
  }

  /// Number of bytes used by the ends ref.
  pub(crate) fn size_of(&self) -> usize {
    self.ends.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test() {
    let mut b = ChunkEndsBuilder::new();
    b.add(12);
    b.add(99);
    b.add(1024);
    let mut bytes = vec![];
    b.write(&mut bytes).unwrap();

    assert_eq!(16, bytes.len());
    let ends = ChunkEndsRef::from(&bytes).unwrap();
    assert_eq!(bytes.len(), ends.1);
    let ends = ends.0;
    assert_eq!(12, ends.get(0));
    assert_eq!(99, ends.get(1));
    assert_eq!(1024, ends.get(2));
  }
}
