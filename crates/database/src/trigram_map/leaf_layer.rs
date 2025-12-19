use std::io;

use crate::{ChunkInventoryBuilder, ChunkListBuilder, NextLayerBuilder, NextLayerRef};

const ENTRY_LEN: usize = size_of::<u8>() + size_of::<u32>();

pub struct LeafLayerBuilder {
  n: [Option<usize>; 256],
}

impl LeafLayerBuilder {
  pub fn new() -> LeafLayerBuilder {
    LeafLayerBuilder { n: [None; 256] }
  }

  #[must_use]
  pub fn get<'s, 'b>(
    &'s mut self,
    inventory: &'b mut ChunkInventoryBuilder,
    c: u8,
  ) -> &'b mut ChunkListBuilder {
    let i = self.n[c as usize].get_or_insert_with(|| inventory.next());
    inventory.get(*i)
  }
}

impl NextLayerBuilder for LeafLayerBuilder {
  fn new() -> Self {
    LeafLayerBuilder::new()
  }

  fn written_len(&self) -> usize {
    self
      .n
      .iter()
      .map(|n| if n.is_some() { ENTRY_LEN } else { 0 })
      .sum()
  }

  fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    for k in 0..256 {
      if self.n[k].is_some() {
        w.write_all(&[k as u8])?;
      }
    }
    for k in 0..256 {
      if let Some(n) = &self.n[k] {
        w.write_all(&(*n as u32).to_be_bytes())?;
      }
    }
    Ok(())
  }
}

/// A "leaf" layer of the tree.
///
/// Serialized as:
/// ```text
/// key_0 as u8
/// key_1 as u8
/// ...
/// key_2 as u8
/// value_0 as u32
/// value_1 as u32
/// ...
/// value_n as u32
/// ```
pub struct LeafLayerRef<'a> {
  keys: &'a [u8],
  values: &'a [u8],
}

impl<'a> NextLayerRef<'a> for LeafLayerRef<'a> {
  fn from_bytes(bytes: &'a [u8]) -> Self {
    // NOCOMMIT return error if bad shape
    let len = bytes.len() / ENTRY_LEN;
    let keys = &bytes[..len];
    let values = &bytes[len..];
    LeafLayerRef { keys, values }
  }
}

impl<'a> LeafLayerRef<'a> {
  #[allow(unused)]
  pub fn len(&self) -> usize {
    self.keys.len()
  }

  pub fn iter(&self) -> impl Iterator<Item = (u8, usize)> {
    (0..(self.len())).map(|i| (self.keys[i], self.read_value(i)))
  }

  pub fn get(&self, k: u8) -> Option<usize> {
    self
      .keys
      .binary_search(&k)
      .ok()
      .map(|index| self.read_value(index))
  }

  #[inline]
  fn read_value(&self, index: usize) -> usize {
    let start = index * size_of::<u32>();
    let end = start + size_of::<u32>();
    u32::from_be_bytes(self.values[start..end].try_into().unwrap()) as usize
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ChunkInventory, ChunkInventoryRef};

  #[test]
  fn test() {
    let mut inventory = ChunkInventory::builder();
    let mut b = LeafLayerBuilder::new();

    b.get(&mut inventory, b'a').add(0);
    b.get(&mut inventory, b'b').add(0);
    b.get(&mut inventory, b'a').add(1);
    b.get(&mut inventory, b'd').add(2);
    b.get(&mut inventory, b'c').add(1);
    b.get(&mut inventory, b'd').add(3);

    let mut leaf_bytes = vec![];
    b.write(&mut leaf_bytes).unwrap();
    assert_eq!(4 * (1 + 4), leaf_bytes.len());
    let leaf = LeafLayerRef::from_bytes(&leaf_bytes);
    assert_eq!(
      vec![(b'a', 0), (b'b', 1), (b'c', 3), (b'd', 2)],
      leaf.iter().collect::<Vec<_>>()
    );
    assert_eq!(Some(0), leaf.get(b'a'));
    assert_eq!(None, leaf.get(b'z'));

    let mut inventory_bytes = vec![];
    inventory.build().write(&mut inventory_bytes).unwrap();
    let i = ChunkInventoryRef::from(&inventory_bytes).0;
    assert_eq!(4, i.len());
    assert_eq!(
      vec![0, 1],
      i.get(0).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![0],
      i.get(1).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![2, 3],
      i.get(2).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![1],
      i.get(3).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
  }
}
