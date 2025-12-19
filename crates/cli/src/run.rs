use super::Error;
use database::DatabaseRef;
use std::io::{Read, Seek};
use std::{fs::File, io};
use tracing::{Level, event, span};
use trigrams_from_regex::{Query, trigrams};

use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::{Searcher, SearcherBuilder};

pub(crate) fn run(pattern: String, file: String) -> Result<(), Error> {
  let span = span!(Level::TRACE, "run");
  let _guard = span.enter();

  let directories = directories::BaseDirs::new().ok_or(Error::NoHome)?;
  let regex = regex_syntax::parse(&pattern)?;

  let (path, mut file) = crate::target_file::open(file)?;

  let trigrams = trigrams(&regex);
  if matches!(trigrams, Query::MatchNone) {
    event!(Level::DEBUG, "regex matches nothing");
    return Ok(());
  }
  if matches!(trigrams, Query::MatchAll) {
    event!(Level::DEBUG, "regex matches all trigrams");
    return match_all(&file, &pattern);
  }
  event!(Level::DEBUG, "regex {trigrams:#?}");

  let db = crate::db::make(&directories, &path, &file)?;
  // NOCOMMIT if the file has an error, rebuild it.
  let db = DatabaseRef::from(&db[..]).map_err(|e| Error::DatabaseReadError(format!("{}", e)))?;
  let trigrams = database_queries::rewrite(&db, trigrams);
  if matches!(trigrams, Query::MatchNone) {
    event!(Level::DEBUG, "rewritten regex matches nothing");
    return Ok(());
  }
  if matches!(trigrams, Query::MatchAll) {
    event!(Level::DEBUG, "rewritten regex matches all trigrams");
    return match_all(&file, &pattern);
  }
  event!(Level::DEBUG, "rewritten regex {trigrams:#?}");

  let mut trigrams = database_queries::eval(db.chunk_count() as u64 - 1, trigrams);
  let mut searcher = searcher();
  let matcher = matcher(&pattern);
  let mut buffer = vec![];
  while trigrams.advance()? {
    let current = trigrams.current() as u32;
    event!(
      Level::TRACE,
      "candidate chunk: {}/{}",
      current,
      db.chunk_count()
    );

    let start = if current == 0 {
      0
    } else {
      db.chunk_end_offset(current - 1)
    };
    let end = db.chunk_end_offset(current);
    file.seek(io::SeekFrom::Start(start as u64))?;
    let len = (end - start) as usize;
    if buffer.len() < len {
      buffer.resize(len, 0);
    }
    file.read_exact(&mut buffer[..len])?;
    let mut chunk_first_line = Option::None;
    searcher.search_slice(
      &matcher,
      &buffer[..len],
      UTF8(|lnum, line| {
        let chunk_first_line = *chunk_first_line.get_or_insert_with(|| {
          if current == 0 {
            0
          } else {
            db.chunk_end_offset(current)
          }
        });
        let lnum = chunk_first_line + lnum as u32;
        let line = line.trim_end_matches(|c| c == '\r' || c == '\n');
        println!("{lnum}:{line}");
        Ok(true)
      }),
    )?;
  }
  Ok(())
}

fn match_all(file: &File, pattern: &str) -> Result<(), Error> {
  searcher().search_reader(
    &matcher(pattern),
    file,
    UTF8(|lnum, line| {
      let line = line.trim_end_matches(|c| c == '\r' || c == '\n');
      print!("{lnum}:{line}");
      Ok(true)
    }),
  )?;
  Ok(())
}

fn searcher() -> Searcher {
  SearcherBuilder::new().build()
}

fn matcher(pattern: &str) -> RegexMatcher {
  RegexMatcher::new_line_matcher(&pattern)
    .expect("Unexpected error parsing regex. Should have failed when parsing hir.")
}
