use super::{QueryEval, Result};

#[derive(Debug)]
pub(crate) struct Heap<L: Iterator<Item = int::DecodeResult>> {
  children: Vec<QueryEval<L>>,
  heapified: bool,
}

impl<L: Iterator<Item = int::DecodeResult>> Heap<L> {
  pub fn new(children: impl IntoIterator<Item = QueryEval<L>>) -> Heap<L> {
    let children = children.into_iter().collect::<Vec<_>>();
    Heap {
      children,
      heapified: false,
    }
  }

  /// Advance the top.
  pub(crate) fn advance(&mut self) -> Result<bool> {
    if !self.heapified {
      self.heapified = true;
      self.heapify()?;
      return Ok(!self.children.is_empty());
    }
    let Some(top) = self.children.get_mut(0) else {
      return Ok(false);
    };
    if top.advance()? == false {
      self.children.swap_remove(0);
      if self.children.is_empty() {
        return Ok(false);
      }
    }
    self.push_down(0);
    Ok(true)
  }

  pub(crate) fn current(&self) -> u64 {
    self.children[0].current()
  }

  pub(crate) fn len(&self) -> usize {
    self.children.len()
  }

  pub(crate) fn heapified(&self) -> bool {
    self.heapified
  }

  fn left_child(i: usize) -> usize {
    2 * i + 1
  }

  fn right_child(i: usize) -> usize {
    2 * i + 2
  }

  /// Correct the heap invariant of a parent and its children. This runs in {@code O(log n)} time.
  fn push_down(&mut self, idx: usize) {
    let mut curr_idx = idx;
    let left_idx = Self::left_child(idx);
    if left_idx < self.children.len()
      && self.children[left_idx].current() < self.children[curr_idx].current()
    {
      curr_idx = left_idx;
    }

    let right_idx = Self::right_child(idx);
    if right_idx < self.children.len()
      && self.children[right_idx].current() < self.children[curr_idx].current()
    {
      curr_idx = right_idx;
    }

    if curr_idx != idx {
      self.children.swap(curr_idx, idx);
      self.push_down(curr_idx);
    }
  }

  /// Heapify the buckets, assuming random order. Look up "Floyd's linear-time heap construction algorithm" for more.
  fn heapify(&mut self) -> Result<()> {
    for c in 0..self.children.len() {
      if !self.children[c].advance()? {
        self.children.swap_remove(0);
      }
    }
    if self.children.len() < 2 {
      return Ok(());
    }
    let max_parent = ((self.children.len() + 1) / 2 - 1).max(1);
    for parent in (0..=max_parent).rev() {
      self.push_down(parent);
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::leaf_from_numbers;
  use std::fmt;

  pub(crate) fn heap_from_numbers(
    numbers: impl IntoIterator<Item = impl IntoIterator<Item = u64>>,
  ) -> Heap<std::vec::IntoIter<Result<u64>>> {
    Heap::new(numbers.into_iter().map(|n| leaf_from_numbers(n)))
  }

  mod one {
    use super::*;

    #[test]
    fn one() {
      assert_contents(&mut heap_from_numbers([[1]]), [1]);
    }

    #[test]
    fn three() {
      assert_contents(&mut heap_from_numbers([[1, 2, 3]]), [1, 2, 3]);
    }
  }

  mod two {
    use super::*;

    #[test]
    fn one_each() {
      assert_contents(&mut heap_from_numbers([[1], [4]]), [1, 4]);
    }

    #[test]
    fn low_then_high() {
      assert_contents(
        &mut heap_from_numbers([[1, 2, 3], [4, 5, 6]]),
        [1, 2, 3, 4, 5, 6],
      );
    }

    #[test]
    fn high_then_low() {
      assert_contents(
        &mut heap_from_numbers([[4, 5, 6], [1, 2, 3]]),
        [1, 2, 3, 4, 5, 6],
      );
    }

    #[test]
    fn odds_and_evens() {
      assert_contents(
        &mut heap_from_numbers([[1, 3, 5], [2, 4, 6]]),
        [1, 2, 3, 4, 5, 6],
      );
    }

    #[test]
    fn big_and_little() {
      assert_contents(
        &mut heap_from_numbers([vec![2, 3, 4, 5, 6], vec![1]]),
        [1, 2, 3, 4, 5, 6],
      );
    }

    #[test]
    fn dups() {
      assert_contents(
        &mut heap_from_numbers([[1, 2, 3], [1, 2, 3]]),
        [1, 1, 2, 2, 3, 3],
      );
    }
  }

  mod three {
    use super::*;

    #[test]
    fn one_each() {
      assert_contents(&mut heap_from_numbers([[1], [4], [10]]), [1, 4, 10]);
    }

    #[test]
    fn low_then_high() {
      assert_contents(
        &mut heap_from_numbers([[1, 2], [3, 4], [5, 6]]),
        [1, 2, 3, 4, 5, 6],
      );
    }

    #[test]
    fn high_then_low() {
      assert_contents(
        &mut heap_from_numbers([[5, 6], [3, 4], [1, 2]]),
        [1, 2, 3, 4, 5, 6],
      );
    }

    #[test]
    fn big_and_little() {
      assert_contents(
        &mut heap_from_numbers([vec![3, 4, 5, 6], vec![1], vec![2]]),
        [1, 2, 3, 4, 5, 6],
      );
    }

    #[test]
    fn dups() {
      assert_contents(
        &mut heap_from_numbers([[1, 2], [1, 2], [1, 2]]),
        [1, 1, 1, 2, 2, 2],
      );
    }
  }

  mod six {
    use super::*;

    #[test]
    fn one_each() {
      assert_contents(
        &mut heap_from_numbers([[1], [2], [1], [3], [1], [2], [4]]),
        [1, 1, 1, 2, 2, 3, 4],
      );
    }
  }

  fn assert_contents<L: Iterator<Item = int::DecodeResult> + fmt::Debug>(
    h: &mut Heap<L>,
    expected: impl IntoIterator<Item = u64>,
  ) {
    let mut result = vec![];
    while h.advance().unwrap() {
      result.push(h.current());
    }
    assert_eq!(expected.into_iter().collect::<Vec<_>>(), result);
  }
}
