/// Always ascending encoding for ints.
use int::DecodeResult;

pub fn iter<I: Iterator<Item = DecodeResult>>(iter: I) -> impl Iterator<Item = DecodeResult> {
  Iter { prev: None, iter }
}

struct Iter<I: Iterator<Item = DecodeResult>> {
  prev: Option<u64>,
  iter: I,
}

impl<I: Iterator<Item = DecodeResult>> Iterator for Iter<I> {
  type Item = DecodeResult;

  fn next(&mut self) -> Option<Self::Item> {
    let next = match self.iter.next()? {
      Ok(n) => n,
      Err(e) => return Some(Err(e)),
    };
    let r = match self.prev {
      None => next,
      Some(prev) => prev + next + 1,
    };
    self.prev = Some(r);
    Some(Ok(r))
  }
}

pub fn write<W: int::Write>(w: W) -> Write<W> {
  Write { prev: None, w }
}

pub struct Write<W: int::Write> {
  prev: Option<u64>,
  w: W,
}

impl<W: int::Write> int::Write for Write<W> {
  type Finish = W::Finish;

  fn write(&mut self, i: u64) {
    match self.prev {
      None => self.w.write(i),
      Some(prev) => {
        if i <= prev {
          panic!("invalid input {i} <= {prev}");
        }
        self.w.write(i - prev - 1)
      }
    };
    self.prev = Some(i);
  }

  fn finish(self) -> Self::Finish {
    self.w.finish()
  }
}

pub fn consume_dupes<W: int::Write>(w: W) -> ConsumeDupes<W> {
  let w: Write<W> = write(w);
  ConsumeDupes { w }
}

pub struct ConsumeDupes<W: int::Write> {
  w: Write<W>,
}

impl<W: int::Write> int::Write for ConsumeDupes<W> {
  type Finish = W::Finish;

  fn write(&mut self, i: u64) {
    if Some(i) != self.w.prev {
      self.w.write(i);
    }
  }

  fn finish(self) -> Self::Finish {
    self.w.finish()
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
      _1_2 = { &[1, 2], &[1, 0] },
      _1_3 = { &[1, 3], &[1, 1] },
    )]
  fn examples(values: &[u64], encoded: &[u64]) {
    // Check iter
    let mut iter = crate::iter(encoded.iter().map(|v| Ok(*v)));
    for v in values {
      assert_eq!(iter.next(), Some(Ok(*v)));
    }
    assert_eq!(iter.next(), None);

    // Check writer
    let mut write = crate::write(vec![]);
    for v in values {
      write.write(*v);
    }
    assert_eq!(write.finish(), encoded);
  }

  #[parameterized(
    _1 = { &[1], &[1], &[1] },
    _2 = { &[2], &[2], &[2] },
    _1_2_2 = { &[1, 2, 2], &[1, 0], &[1, 2] },
    _1_3_4_4 = { &[1, 3, 4, 4], &[1, 1, 0], &[1, 3, 4] },
  )]
  fn consume_dupes(input: &[u64], encoded: &[u64], decoded: &[u64]) {
    let mut write = crate::consume_dupes(vec![]);
    for v in input {
      write.write(*v);
    }
    assert_eq!(write.finish(), encoded);

    let mut iter = crate::iter(encoded.iter().map(|v| Ok(*v)));
    for v in decoded {
      assert_eq!(iter.next(), Some(Ok(*v)));
    }
    assert_eq!(iter.next(), None);
  }

  #[test]
  fn read_write() {
    arbtest(|u| {
      let mut values: Vec<u64> = u.arbitrary()?;
      values.sort();
      let mut write = crate::write(vec![]);
      for v in values.iter() {
        write.write(*v);
      }
      let buf = write.finish();
      let iter = crate::iter(buf.iter().map(|v| Ok(*v)));
      let collected: Vec<u64> = iter.map(|r| r.unwrap()).collect();
      assert_eq!(values, collected);
      Ok(())
    });
  }
}
