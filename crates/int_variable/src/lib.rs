/// Lucene style vint.
/// https://lucene.apache.org/core/10_0_0/core/org/apache/lucene/store/DataOutput.html#writeVInt(int)
use int::{DecodeError, DecodeResult};

pub fn iter<I: Iterator<Item = u8>>(iter: I) -> impl Iterator<Item = DecodeResult> {
  Iter { iter }
}

struct Iter<I: Iterator<Item = u8>> {
  iter: I,
}

impl<I: Iterator<Item = u8>> Iterator for Iter<I> {
  type Item = DecodeResult;

  fn next(&mut self) -> Option<Self::Item> {
    let first = self.iter.next()?;
    Some(self.begun_next(first))
  }
}

impl<I: Iterator<Item = u8>> Iter<I> {
  fn begun_next(&mut self, first: u8) -> DecodeResult {
    if first < 128 {
      return Ok(first as u64);
    }
    let mut v = first as u64 & 0x7f;
    let mut shift = 7;
    loop {
      let b = self.iter.next().ok_or(DecodeError("read partial vint"))?;
      v |= (b as u64 & 0x7f) << shift;
      if b < 128 {
        return Ok(v);
      }
      shift += 7;
      if shift >= 64 {
        return Err(DecodeError("read vint more bigger than 64"));
      }
    }
  }
}

pub fn write() -> Write {
  Write { w: vec![] }
}

pub struct Write {
  w: Vec<u8>,
}

impl int::Write for Write {
  type Finish = Vec<u8>;

  fn write(&mut self, mut i: u64) {
    while i > 127 {
      self.w.push(((i as u8) & 0x7f) | 0x80);
      i >>= 7;
    }
    self.w.push(i as u8)
  }

  fn finish(self) -> Self::Finish {
    self.w
  }
}

#[cfg(test)]
mod tests {
  use arbtest::arbtest;
  use int::Write as _;
  use yare::parameterized;

  #[parameterized(
      _1 = { &[1], &[1] },
      _2 = { &[2], &[2] },
      _127 = { &[127], &[127] },
      _4_5_6 = { &[4, 5, 6], &[4, 5, 6] },
      _128 = { &[128], &[0b10000000, 0b00000001] },
      _16383 = { &[16383], &[0b1111_1111, 0b0111_1111] },
      _128_1 = { &[128, 1], &[0b10000000, 0b00000001, 1] },
      _16384 = { &[16384], &[0b10000000, 0b10000000, 0b00000001] },
    )]
  fn examples(values: &[u64], encoded: &[u8]) {
    // Check iter
    let mut iter = crate::iter(encoded.iter().copied());
    for v in values {
      assert_eq!(iter.next(), Some(Ok(*v)));
    }
    assert_eq!(iter.next(), None);

    // Check writer
    let mut writer = crate::write();
    for v in values {
      writer.write(*v);
    }
    assert_eq!(writer.finish(), encoded);
  }

  #[test]
  fn read_write() {
    arbtest(|u| {
      let values: Vec<u64> = u.arbitrary()?;
      let mut write = crate::write();
      for v in values.iter() {
        write.write(*v);
      }
      let iter = crate::iter(write.finish().into_iter());
      let collected: Vec<u64> = iter.map(|r| r.unwrap()).collect();
      assert_eq!(values, collected);
      Ok(())
    });
  }
}
