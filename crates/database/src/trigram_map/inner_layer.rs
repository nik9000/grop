use crate::{LeafLayerBuilder, LeafLayerRef};
use std::marker::PhantomData;
use std::{fmt, io};

pub fn three_layer_builder() -> InnerLayerBuilder<Box<InnerLayerBuilder<Box<LeafLayerBuilder>>>> {
  InnerLayerBuilder::new()
}

pub fn three_layer_ref_from_bytes<'a>(
  bytes: &'a [u8],
) -> InnerLayerRef<'a, InnerLayerRef<'a, LeafLayerRef<'a>>> {
  InnerLayerRef::from_bytes(bytes)
}

pub trait NextLayerBuilder {
  fn new() -> Self; // NOCOMMIT should be default?

  fn written_len(&self) -> usize;

  fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()>;
}

impl<N: NextLayerBuilder> NextLayerBuilder for Box<N> {
  fn new() -> Self {
    Box::new(N::new())
  }

  fn written_len(&self) -> usize {
    self.as_ref().written_len()
  }

  fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    self.as_ref().write(w)
  }
}

pub struct InnerLayerBuilder<N> {
  n: [Option<N>; 256],
}

impl<N: NextLayerBuilder> InnerLayerBuilder<N> {
  #[must_use]
  pub fn get(&mut self, c: u8) -> &mut N {
    self.n[c as usize].get_or_insert_with(N::new)
  }
}

impl<N: NextLayerBuilder> NextLayerBuilder for InnerLayerBuilder<N> {
  fn new() -> Self {
    InnerLayerBuilder {
      n: [const { None }; 256],
    }
  }

  fn written_len(&self) -> usize {
    let mut len = size_of::<u32>();
    for n in self.n.iter() {
      if let Some(n) = n {
        len += 1 + size_of::<u32>() + n.written_len();
      }
    }
    len
  }

  fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    let len = self.n.iter().filter(|o| o.is_some()).count();

    w.write_all(&(len as u32).to_be_bytes())?;
    for k in 0..256 {
      if self.n[k].is_some() {
        w.write_all(&[k as u8])?;
      }
    }
    let mut end = 0;
    for k in 0..256 {
      if let Some(n) = &self.n[k] {
        end += n.written_len() as u32;
        w.write_all(&end.to_be_bytes())?;
      }
    }
    for k in 0..256 {
      if let Some(n) = &self.n[k] {
        n.write(w)?;
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
/// end_offset_of_value_0 as u32
/// end_offset_of_value_1 as u32
/// ...
/// end_offset_of_value_n as u32
/// value_0
/// value_1
/// ...
/// value_n
/// ```
pub struct InnerLayerRef<'a, N> {
  keys: &'a [u8],
  ends: &'a [u8],
  values: &'a [u8],
  phantom: PhantomData<N>,
}

pub trait NextLayerRef<'a> {
  /// Read from a perfectly sized slice.
  fn from_bytes(bytes: &'a [u8]) -> Self;
}

impl<'a, N: NextLayerRef<'a>> NextLayerRef<'a> for InnerLayerRef<'a, N> {
  /// Read from a perfectly sized slice.
  fn from_bytes(bytes: &'a [u8]) -> Self {
    // NOCOMMIT return error if bad shape
    // NOCOMMIT can't we get length from offsets above?
    let len_end = size_of::<u32>();
    let len = u32::from_be_bytes(bytes[..len_end].try_into().unwrap()) as usize;

    let keys_end = len_end + len;
    let keys = &bytes[len_end..keys_end];

    let ends_end = keys_end + len * size_of::<u32>();
    let ends = &bytes[keys_end..ends_end];

    let values = &bytes[ends_end..];
    InnerLayerRef {
      keys,
      ends,
      values,
      phantom: PhantomData,
    }
  }
}

impl<'a, N: NextLayerRef<'a>> InnerLayerRef<'a, N> {
  #[allow(unused)]
  pub fn len(&self) -> usize {
    self.keys.len()
  }

  /// Number of bytes in this layer
  pub fn size_of(&self) -> usize {
    self.keys.len() + self.ends.len() + self.values.len()
  }

  pub fn iter(&self) -> impl Iterator<Item = (u8, N)> {
    (0..(self.len())).map(|i| (self.keys[i], self.read_value(i)))
  }

  pub fn get(&self, k: u8) -> Option<N> {
    self
      .keys
      .binary_search(&k)
      .ok()
      .map(|index| self.read_value(index))
  }

  fn read_value(&self, index: usize) -> N {
    let start = if index == 0 {
      0
    } else {
      self.read_end(index - 1)
    };
    // NOCOMMIT the last end can be [start..]
    let end = self.read_end(index);
    N::from_bytes(&self.values[start..end])
  }

  fn read_end(&self, index: usize) -> usize {
    let start = index * size_of::<u32>();
    let end = start + size_of::<u32>();
    u32::from_be_bytes(self.ends[start..end].try_into().unwrap()) as usize
  }
}

impl<'a> fmt::Debug for InnerLayerRef<'a, InnerLayerRef<'a, LeafLayerRef<'a>>> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut m = f.debug_map();
    for (k0, v) in self.iter() {
      for (k1, v) in v.iter() {
        for (k2, v) in v.iter() {
          m.key(&format!("{}{}{}", k0 as char, k1 as char, k2 as char))
            .value(&v);
        }
      }
    }
    m.finish()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ChunkInventory, ChunkInventoryRef};

  fn test_layer() -> (
    InnerLayerBuilder<Box<InnerLayerBuilder<Box<LeafLayerBuilder>>>>,
    ChunkInventory,
  ) {
    let mut b = three_layer_builder();
    let mut inventory = ChunkInventory::builder();

    b.get(b'a').get(b'a').get(&mut inventory, b'a').add(0);
    b.get(b'a').get(b'a').get(&mut inventory, b'b').add(0);
    b.get(b'a').get(b'a').get(&mut inventory, b'c').add(0);
    b.get(b'a').get(b'b').get(&mut inventory, b'a').add(0);
    b.get(b'a').get(b'z').get(&mut inventory, b'z').add(0);
    b.get(b'c').get(b'z').get(&mut inventory, b'z').add(0);

    b.get(b'a').get(b'a').get(&mut inventory, b'a').add(1);
    b.get(b'a').get(b'a').get(&mut inventory, b'b').add(1);
    b.get(b'a').get(b'z').get(&mut inventory, b'z').add(1);
    (b, inventory.build())
  }

  #[test]
  fn size() {
    let mut map_bytes = vec![];
    test_layer().0.write(&mut map_bytes).unwrap();
    // assert_eq!(72, map_bytes.len());
  }

  #[test]
  fn iter() {
    let mut bytes = vec![];
    test_layer().0.write(&mut bytes).unwrap();
    let map = three_layer_ref_from_bytes(&bytes);
    let mut all = vec![];
    for (k0, v) in map.iter() {
      for (k1, v) in v.iter() {
        for (k2, v) in v.iter() {
          all.push((format!("{}{}{}", k0 as char, k1 as char, k2 as char), v));
        }
      }
    }
    assert_eq!(
      vec![
        ("aaa".to_string(), 0),
        ("aab".to_string(), 1),
        ("aac".to_string(), 2),
        ("aba".to_string(), 3),
        ("azz".to_string(), 4),
        ("czz".to_string(), 5),
      ],
      all
    );
  }

  #[test]
  fn get() {
    let mut bytes = vec![];
    test_layer().0.write(&mut bytes).unwrap();
    let map = three_layer_ref_from_bytes(&bytes);
    assert_eq!(Some(0), map.get(b'a').unwrap().get(b'a').unwrap().get(b'a'));
    assert_eq!(Some(2), map.get(b'a').unwrap().get(b'a').unwrap().get(b'c'));
    assert!(map.get(b'a').unwrap().get(b'f').is_none());
    assert!(map.get(b'z').is_none());
  }

  #[test]
  fn debug() {
    let mut bytes = vec![];
    test_layer().0.write(&mut bytes).unwrap();
    let map = three_layer_ref_from_bytes(&bytes);
    assert_eq!(
      r#"{
    "aaa": 0,
    "aab": 1,
    "aac": 2,
    "aba": 3,
    "azz": 4,
    "czz": 5,
}"#,
      format!("{:#?}", map)
    );
  }

  #[test]
  fn inventory() {
    let mut bytes = vec![];
    test_layer().1.write(&mut bytes).unwrap();
    let i = ChunkInventoryRef::from(&bytes).0;
    assert_eq!(i.len(), 6);
    assert_eq!(
      vec![0, 1],
      i.get(0).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![0, 1],
      i.get(1).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![0],
      i.get(2).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![0],
      i.get(3).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![0, 1],
      i.get(4).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
    assert_eq!(
      vec![0],
      i.get(5).into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    );
  }
}
