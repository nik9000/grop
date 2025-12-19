use super::{QueryEval, Result};

#[derive(Debug)]
pub struct Leaf<L: Iterator<Item = int::DecodeResult>> {
  current: Option<u64>,
  iter: L,
}

impl<L: Iterator<Item = int::DecodeResult>> Leaf<L> {
  pub(crate) fn new(iter: L) -> QueryEval<L> {
    QueryEval::Leaf(Leaf {
      current: None,
      iter,
    })
  }

  pub(crate) fn advance(&mut self) -> Result<bool> {
    let Some(current) = self.iter.next() else {
      return Ok(false);
    };
    self.current = Some(current?);
    Ok(true)
  }

  pub(crate) fn current(&self) -> u64 {
    self.current.expect("not yet advanced")
  }
}
