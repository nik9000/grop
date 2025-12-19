use super::Result;

#[derive(Debug)]
pub struct MatchAll {
  prev: u64,
  max: u64,
}

impl MatchAll {
  pub(crate) fn new(max: u64) -> MatchAll {
    MatchAll { prev: 0, max }
  }

  pub(crate) fn advance(&mut self) -> Result<bool> {
    if self.prev > self.max {
      Ok(false)
    } else {
      self.prev += 1;
      Ok(true)
    }
  }

  pub(crate) fn current(&self) -> u64 {
    self.prev - 1
  }
}
