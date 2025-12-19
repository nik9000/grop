use super::Error;
use database::DatabaseRef;
use database_queries::Meta;
use std::fs::File;
use tracing::{Level, event, span};
use trigrams_from_regex::{Query, trigrams};

pub(crate) fn run(pattern: String, file: String) -> Result<(), Error> {
  let span = span!(Level::TRACE, "run");
  let _guard = span.enter();

  query(
    pattern,
    file,
    Box::new(|| {
      println!("regex query matches no chunks");
      Ok(())
    }),
    Box::new(|_, _| {
      println!("regex query matches all chunks");
      Ok(())
    }),
    Box::new(|query, db, _, _| match_some(query, db)),
  )
}

pub(crate) fn query(
  pattern: String,
  file: String,
  match_none: Box<dyn FnOnce() -> Result<(), Error>>,
  match_all: Box<dyn FnOnce(&File, &str) -> Result<(), Error>>,
  match_some: Box<
    dyn FnOnce(Query<'_, Meta<'_>>, DatabaseRef<'_>, &File, &str) -> Result<(), Error>,
  >,
) -> Result<(), Error> {
  let directories = directories::BaseDirs::new().ok_or(Error::NoHome)?;
  let regex = regex_syntax::parse(&pattern)?;

  let (path, file) = crate::target_file::open(file)?;

  let trigrams = trigrams(&regex);
  if matches!(trigrams, Query::MatchNone) {
    println!("regex matches no chunks");
    return Ok(());
  }
  if matches!(trigrams, Query::MatchAll) {
    println!("regex matches all chunks");
    return Ok(());
  }
  event!(Level::DEBUG, "regex {trigrams:#?}");

  let db = crate::db::make(&directories, &path, &file)?;
  // NOCOMMIT if the file has an error, rebuild it.
  let db = DatabaseRef::from(&db[..]).map_err(|e| Error::DatabaseReadError(format!("{}", e)))?;
  let query = database_queries::rewrite(&db, trigrams);

  match query {
    Query::MatchNone => {
      event!(Level::DEBUG, "rewritten query matches nothing");
      match_none()
    }
    Query::MatchAll => {
      event!(Level::DEBUG, "rewritten query matches all trigrams");
      match_all(&file, &pattern)
    }
    _ => {
      event!(Level::DEBUG, "rewritten query {query:#?}");
      match_some(query, db, &file, &pattern)
    }
  }
}

fn match_some(query: Query<'_, Meta<'_>>, db: DatabaseRef<'_>) -> Result<(), Error> {
  println!("regex query {query:#?}");
  let chunk_count = db.chunk_count();
  let mut query = database_queries::eval(chunk_count as u64 - 1, query);
  let mut candidate_count = 0;
  let chunk_width = (chunk_count as f64).log10() as usize + 1;
  let offset_width = (db.chunk_end_offset(chunk_count - 1) as f64).log10() as usize + 1;

  while query.advance()? {
    let current = query.current() as u32;
    let start = if current == 0 {
      0
    } else {
      db.chunk_end_offset(current - 1)
    };
    let end = db.chunk_end_offset(current);

    let current = fixed_width(current, chunk_width);
    let start = fixed_width(start, offset_width);
    let end = fixed_width(end, offset_width);
    println!("candidate chunk: {current}/{chunk_count}  {start} -> {end}",);
    candidate_count += 1;
  }
  let candidate_pct = (candidate_count as f64) / (chunk_count as f64) * 100.0;
  println!("matched: {candidate_count}/{chunk_count} ({candidate_pct:05.2}%)");
  Ok(())
}

fn fixed_width(n: u32, width: usize) -> String {
  let mut n = format!("{n}");
  if n.len() < width {
    n.insert_str(0, &"0".repeat(width - n.len()));
  }
  n
}
