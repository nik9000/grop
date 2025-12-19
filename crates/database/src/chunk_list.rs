use int::Write as _;
use std::{fmt, io};

/// A list of "chunks" which contain a trigram.
#[derive(Debug, PartialEq)]
pub struct ChunkList {
  bytes: Vec<u8>,
}

impl ChunkList {
  pub fn builder() -> ChunkListBuilder {
    ChunkListBuilder {
      w: int_always_ascending::consume_dupes(int_variable::write()),
    }
  }

  pub fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    w.write_all(&self.bytes)
  }

  pub fn len(&self) -> usize {
    self.bytes.len()
  }

  #[allow(unused)]
  pub fn as_ref(&self) -> ChunkListRef<'_> {
    ChunkListRef {
      bytes: &self.bytes[..],
    }
  }
}

pub struct ChunkListBuilder {
  w: int_always_ascending::ConsumeDupes<int_variable::Write>,
}

impl ChunkListBuilder {
  pub fn add(&mut self, i: u64) {
    self.w.write(i)
  }

  pub fn build(self) -> ChunkList {
    ChunkList {
      bytes: self.w.finish().into(),
    }
  }
}

#[derive(PartialEq)]
pub struct ChunkListRef<'a> {
  bytes: &'a [u8],
}

impl<'a> ChunkListRef<'a> {
  pub fn from(bytes: &'a [u8]) -> Self {
    ChunkListRef { bytes }
  }

  // TODO this should have a kind of count. So if it's ORed we can turn it into MATCH_ALL

  pub fn into_iter(self) -> impl Iterator<Item = int::DecodeResult> {
    int_always_ascending::iter(int_variable::iter(self.bytes.iter().copied()))
  }

  pub fn byte_count(&self) -> usize {
    self.bytes.len()
  }
}

impl<'a> fmt::Debug for ChunkListRef<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut l = f.debug_list();
    for item in int_always_ascending::iter(int_variable::iter(self.bytes.iter().copied())) {
      match item {
        Ok(v) => l.entry(&v),
        Err(e) => l.entry(&e),
      };
    }
    l.finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_list() -> ChunkList {
    let mut w = ChunkList::builder();
    w.add(1);
    w.add(2);
    w.add(12);
    w.build()
  }

  #[test]
  fn iter() {
    let chunk_list = test_list();

    // 1 for the first value
    // 0 because 2 is just one more than 1
    // 9 because 12 is 10 more than 2, and we encode delta - 1
    assert_eq!(vec![1, 0, 9], chunk_list.bytes);

    let iter = chunk_list.as_ref().into_iter();
    let collected: Vec<u64> = iter.map(|v| v.unwrap()).collect();
    assert_eq!(vec![1, 2, 12], collected);
  }

  #[test]
  fn write() {
    let orig = test_list();
    let mut bytes = vec![];
    orig.write(&mut bytes).unwrap();
    assert_eq!(vec![1, 0, 9], bytes.iter().copied().collect::<Vec<_>>());
    let reference = ChunkListRef::from(&bytes);
    assert_eq!(orig.as_ref(), reference);
  }
}
