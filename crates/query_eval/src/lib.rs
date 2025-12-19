mod and;
mod heap;
mod leaf;
mod match_all;
mod r#or;

type Result<V> = std::result::Result<V, int::DecodeError>;

#[derive(Debug)]
#[non_exhaustive]
pub enum QueryEval<L: Iterator<Item = int::DecodeResult>> {
  /// Match all chunks.
  MatchAll(match_all::MatchAll),
  /// Match no docs.
  MatchNone,
  /// Match chunks from an [Iterator].
  Leaf(leaf::Leaf<L>),
  /// Match the `OR` of many queries.
  Or(or::Or<L>),
  /// Match the `AND` of many queries.
  And(and::And<L>),
}

impl<L: Iterator<Item = int::DecodeResult>> QueryEval<L> {
  /// Match all chunks up to `max`.
  pub fn new_match_all(max: u64) -> Self {
    Self::MatchAll(match_all::MatchAll::new(max))
  }

  pub fn new_leaf(iter: L) -> Self {
    leaf::Leaf::new(iter)
  }

  /// Match the `OR` of many queries.
  pub fn new_or(children: impl IntoIterator<Item = QueryEval<L>>) -> Self {
    or::Or::new(children)
  }

  /// Match the `AND` of many queries.
  pub fn new_and(children: impl IntoIterator<Item = QueryEval<L>>) -> Self {
    and::And::new(children)
  }

  pub fn advance(&mut self) -> Result<bool> {
    match self {
      Self::MatchAll(m) => m.advance(),
      Self::MatchNone => Ok(false),
      Self::Leaf(l) => l.advance(),
      Self::Or(or) => or.advance(),
      Self::And(and) => and.advance(),
    }
  }

  pub fn current(&self) -> u64 {
    match self {
      Self::MatchAll(m) => m.current(),
      Self::MatchNone => panic!("no values"),
      Self::Leaf(l) => l.current(),
      Self::Or(or) => or.current(),
      Self::And(and) => and.current(),
    }
  }
}

#[cfg(test)]
pub(crate) fn leaf_from_numbers(
  l: impl IntoIterator<Item = u64>,
) -> QueryEval<std::vec::IntoIter<Result<u64>>> {
  let l = l.into_iter().map(|v| Ok(v)).collect::<Vec<_>>();
  QueryEval::new_leaf(l.into_iter())
}

#[cfg(test)]
pub(crate) fn assert_contents<L: Iterator<Item = int::DecodeResult> + std::fmt::Debug>(
  q: &mut QueryEval<L>,
  expected: impl IntoIterator<Item = u64>,
) {
  let mut result = vec![];
  while q.advance().unwrap() {
    result.push(q.current());
  }
  assert_eq!(expected.into_iter().collect::<Vec<_>>(), result);
}
