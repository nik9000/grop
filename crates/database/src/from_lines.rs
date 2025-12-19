use std::io;
use tracing::{Level, event, span};

use super::*;

impl DatabaseBuilder {
  pub fn from_lines<R: io::BufRead + io::Seek>(
    buf: &mut R,
    lines_per_chunk: u32,
    bytes_per_chunk: u32,
  ) -> io::Result<DatabaseBuilder> {
    let span = span!(Level::TRACE, "from_lines");
    let _guard = span.enter();

    let mut builder = DatabaseBuilder::new();
    let mut line = String::new();

    let mut chunk = 0;
    let mut line_count = 0; // NOCOMMIT this should be u64, right?
    let mut chunk_start = 0;
    let mut stream_position = 0;
    event!(Level::DEBUG, "building chunk {}", chunk);
    loop {
      let line_len = buf.read_line(&mut line)?;
      if line_len == 0 {
        break;
      }
      stream_position += line_len as u64;

      builder.add_line(chunk, line.bytes());

      line_count += 1;
      if line_count % lines_per_chunk == 0
        || (stream_position - chunk_start) as u32 > bytes_per_chunk
      {
        chunk += 1;
        chunk_start = stream_position;
        event!(Level::DEBUG, "building chunk {}", chunk);
        builder.add_chunk_end(
          stream_position as u32, // NOCOMMIT better way of handling big files
          line_count,
        );
      }
      line.clear();
    }
    if chunk_start != stream_position {
      builder.add_chunk_end(
        stream_position as u32, // NOCOMMIT better way of handling big files
        line_count,
      );
    }
    Ok(builder)
  }

  fn add_line(&mut self, chunk: u64, mut bytes: impl Iterator<Item = u8>) {
    let Some(mut first) = bytes.next() else {
      return;
    };
    let Some(mut second) = bytes.next() else {
      return;
    };
    loop {
      let Some(next) = bytes.next() else {
        return;
      };

      self.add_trigram([first, second, next], chunk);
      first = second;
      second = next;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::File;
  use std::path::PathBuf;
  use std::u32;

  fn macbeth_path() -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("..");
    d.push("..");
    d.push("testdata");
    d.push("macbeth.txt");
    d
  }

  #[test]
  fn _16_lines_per_chunk() {
    let mut macbeth = io::BufReader::new(File::open(macbeth_path()).unwrap());
    let db = DatabaseBuilder::from_lines(&mut macbeth, 16, u32::MAX).unwrap();
    let mut bytes = vec![];
    db.write(&mut bytes).unwrap();

    // Actual text is 116138
    assert_eq!(139060, bytes.len());
    let db = DatabaseRef::from(&bytes).unwrap();
    assert_eq!(Some(vec![78, 184]), db.chunks_containing_as_vec("END"));
    assert_eq!(None, db.chunks_containing_as_vec("NOP"));
    assert_eq!(185, db.chunk_count());

    assert_eq!(107404, db.chunk_end_offset(169));
    assert_eq!(108040, db.chunk_end_offset(170));
    assert_eq!(116138, db.chunk_end_offset(db.chunk_count() - 1));
    assert_eq!(2720, db.chunk_end_line_count(169));
    assert_eq!(2736, db.chunk_end_line_count(170));
    assert_eq!(2956, db.chunk_end_line_count(db.chunk_count() - 1));
  }

  #[test]
  fn _16kb_per_chunk() {
    let mut macbeth = io::BufReader::new(File::open(macbeth_path()).unwrap());
    let db = DatabaseBuilder::from_lines(&mut macbeth, u32::MAX, 16 * 1024).unwrap();
    let mut bytes = vec![];
    db.write(&mut bytes).unwrap();

    // Actual text is 116138
    assert_eq!(84426, bytes.len());
    let db = DatabaseRef::from(&bytes).unwrap();
    assert_eq!(Some(vec![2, 7]), db.chunks_containing_as_vec("END"));
    assert_eq!(None, db.chunks_containing_as_vec("NOP"));
    assert_eq!(8, db.chunk_count());

    assert_eq!(49234, db.chunk_end_offset(2));
    assert_eq!(116138, db.chunk_end_offset(7));
    assert_eq!(1269, db.chunk_end_line_count(2));
    assert_eq!(2956, db.chunk_end_line_count(7));
  }
}
