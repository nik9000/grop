use std::{error::Error, fmt};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DecodeError(pub &'static str);

impl Error for DecodeError {}

impl fmt::Display for DecodeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self, f)
  }
}

pub type DecodeResult = std::result::Result<u64, DecodeError>;

pub trait Write {
  type Finish;

  fn write(&mut self, i: u64);

  fn finish(self) -> Self::Finish;
}

impl Write for Vec<u64> {
  type Finish = Vec<u64>;

  fn write(&mut self, i: u64) {
    self.push(i);
  }

  fn finish(self) -> Self::Finish {
    self
  }
}
