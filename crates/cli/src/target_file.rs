use super::Error;
use std::io::{ErrorKind};
use std::path::PathBuf;
use std::{fs::File, path};
use tracing::{Level, span};

pub(crate) fn open(file: String) -> Result<(PathBuf, File), Error> {
  let span = span!(Level::TRACE, "open");
  let _guard = span.enter();

  let path = path::absolute(file)?;
  let file = File::open(&path).map_err(|e| {
    if e.kind() == ErrorKind::NotFound {
      Error::NotFound(path.clone())
    } else {
      Error::IO(e)
    }
  })?;
  Ok((path, file))
}