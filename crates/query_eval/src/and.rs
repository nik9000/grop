use super::{QueryEval, Result};
use crate::heap::Heap;

#[derive(Debug)]
pub struct And<L: Iterator<Item = int::DecodeResult>> {
  children: Heap<L>,
  required_count: usize,
}

impl<L: Iterator<Item = int::DecodeResult>> And<L> {
  pub(crate) fn new(children: impl IntoIterator<Item = QueryEval<L>>) -> QueryEval<L> {
    let children = Heap::new(children);
    let required_count = children.len();
    QueryEval::And(And {
      children,
      required_count,
    })
  }

  pub(crate) fn advance(&mut self) -> Result<bool> {
    if self.required_count != self.children.len() {
      return Ok(false);
    }
    if !self.children.advance()? {
      return Ok(false);
    }
    self.advance_to_first()
  }

  fn advance_to_first(&mut self) -> Result<bool> {
    'next_candidate: loop {
      let candidate = self.children.current();
      let mut remaining = self.required_count - 1;
      while remaining > 0 {
        if self.children.advance()? == false {
          return Ok(false);
        }
        if self.children.current() > candidate {
          continue 'next_candidate;
        }
        remaining -= 1;
      }
      return Ok(true);
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

  pub(crate) fn and_from_numbers(
    numbers: impl IntoIterator<Item = impl IntoIterator<Item = u64>>,
  ) -> QueryEval<std::vec::IntoIter<Result<u64>>> {
    And::new(numbers.into_iter().map(|n| leaf_from_numbers(n)))
  }

  #[test]
  fn one() {
    assert_contents(&mut and_from_numbers([[1, 2]]), [1, 2]);
  }

  #[test]
  fn two_no_dupes() {
    assert_contents(&mut and_from_numbers([vec![1, 2], vec![3]]), []);
  }

  #[test]
  fn two_one_dupe() {
    assert_contents(&mut and_from_numbers([[1, 2], [2, 3]]), [2]);
  }

  #[test]
  fn two_two_dupe() {
    assert_contents(&mut and_from_numbers([[1, 2, 3], [2, 3, 4]]), [2, 3]);
  }

  #[test]
  fn three_dupes() {
    assert_contents(
      &mut and_from_numbers([vec![1, 2, 3, 4, 5], vec![3, 4, 5, 6], vec![3, 4, 5]]),
      [3, 4, 5],
    );
  }

  #[test]
  fn three_no_dupes() {
    assert_contents(
      &mut and_from_numbers([vec![1, 2], vec![2, 3], vec![1, 3]]),
      [],
    );
  }

  #[test]
  fn four_dupes() {
    assert_contents(
      &mut and_from_numbers([
        vec![1, 2, 3, 4, 5],
        vec![3, 4, 5, 6],
        vec![3, 4, 5],
        vec![3, 4],
      ]),
      [3, 4],
    );
  }
}
