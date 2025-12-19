use super::{QueryEval, Result};
use crate::heap::Heap;

#[derive(Debug)]
pub struct Or<L: Iterator<Item = int::DecodeResult>> {
  children: Heap<L>,
}

impl<L: Iterator<Item = int::DecodeResult>> Or<L> {
  pub(crate) fn new(children: impl IntoIterator<Item = QueryEval<L>>) -> QueryEval<L> {
    let children = Heap::new(children);
    QueryEval::Or(Or { children })
  }

  pub(crate) fn advance(&mut self) -> Result<bool> {
    if !self.children.heapified() {
      return self.children.advance();
    }
    loop {
      let prev = self.children.current();
      if !self.children.advance()? {
        return Ok(false);
      }
      if self.children.current() != prev {
        return Ok(true);
      }
    }
  }

  pub(crate) fn current(&self) -> u64 {
    self.children.current()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::*;

  pub(crate) fn or_from_numbers(
    numbers: impl IntoIterator<Item = impl IntoIterator<Item = u64>>,
  ) -> QueryEval<std::vec::IntoIter<Result<u64>>> {
    Or::new(numbers.into_iter().map(|n| leaf_from_numbers(n)))
  }

  #[test]
  fn one() {
    assert_contents(&mut or_from_numbers([[1, 2]]), [1, 2]);
  }

  #[test]
  fn two() {
    assert_contents(&mut or_from_numbers([vec![1, 2], vec![3]]), [1, 2, 3]);
  }

  #[test]
  fn two_dupes() {
    assert_contents(&mut or_from_numbers([[1, 2], [2, 3]]), [1, 2, 3]);
  }

  #[test]
  fn three() {
    assert_contents(
      &mut or_from_numbers([vec![1, 2], vec![3], vec![4, 5]]),
      [1, 2, 3, 4, 5],
    );
  }

  #[test]
  fn three_dupes() {
    assert_contents(&mut or_from_numbers([[1, 2], [2, 3], [1, 3]]), [1, 2, 3]);
  }
}
