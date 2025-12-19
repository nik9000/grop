use database::{ChunkListRef, DatabaseRef};
use query_eval::QueryEval;
use std::fmt;
use tracing::{Level, event};
use trigrams_from_regex::Query;

pub struct Meta<'d>(ChunkListRef<'d>);

pub fn rewrite<'d, 'q>(db: &DatabaseRef<'d>, q: Query<'q, ()>) -> Query<'q, Meta<'d>> {
  match q {
    Query::MatchAll => Query::MatchAll,
    Query::MatchNone => Query::MatchNone,
    Query::Trigram(trigram, _) => {
      if let Some(chunks) = db.chunks_containing(trigram) {
        Query::Trigram(trigram, Meta(chunks))
      } else {
        Query::MatchNone
      }
    }
    Query::Or(sub) => Query::or(sub.into_iter().map(|s| rewrite(db, s))),
    Query::And(sub) => Query::and(sub.into_iter().map(|s| rewrite(db, s))),
    _ => {
      event!(Level::WARN, "unsupported `{:?}`", q);
      Query::MatchAll
    }
  }
}

impl<'d> trigrams_from_regex::Meta for Meta<'d> {
  fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({})", self.0.byte_count())
  }
}

pub fn eval<'d, 'q>(
  max_chunk: u64,
  q: Query<'q, Meta<'d>>,
) -> QueryEval<impl Iterator<Item = int::DecodeResult>> {
  // NOCOMMIT get the max chunk from the db
  match q {
    Query::MatchAll => QueryEval::new_match_all(max_chunk),
    Query::MatchNone => QueryEval::MatchNone,
    Query::Trigram(_, list) => QueryEval::new_leaf(list.0.into_iter()),
    Query::Or(sub) => QueryEval::new_or(sub.into_iter().map(|s| eval(max_chunk, s))),
    Query::And(sub) => QueryEval::new_and(sub.into_iter().map(|s| eval(max_chunk, s))),
    _ => {
      event!(Level::WARN, "unsupported `{:?}`", q);
      QueryEval::new_match_all(max_chunk)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use database::DatabaseBuilder;
  use regex_syntax::parse;
  use std::{fs::File, io, path::PathBuf, sync::LazyLock, u32};
  use tracing::{Level, event};
  use trigrams_from_regex::trigrams;
  use yare::parameterized;

  static TRACING: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt()
      .with_line_number(true)
      .with_file(true)
      .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
      .with_test_writer()
      .with_max_level(Level::TRACE)
      .init()
  });

  static MACBETH: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("..");
    d.push("..");
    d.push("testdata");
    d.push("macbeth.txt");
    let mut hamlet = io::BufReader::new(File::open(d).unwrap());
    let db = DatabaseBuilder::from_lines(&mut hamlet, 16, u32::MAX).unwrap();
    let mut bytes = vec![];
    db.write(&mut bytes).unwrap();
    bytes
  });

  #[parameterized(
                    empty = {        "",                Query::MatchAll, &[] },
                        a = {       "a",                Query::MatchAll, &[] },
                      asd = {     "asd",               Query::MatchNone, &[] },
                      tom = {     "tom",              Query::str("tom"), &[70, 76, 77, 101, 103, 120, 137, 153, 170] },
                     tomo = {    "tomo", Query::and_str(["omo", "tom"]), &[76, 77, 103, 170] },
    small_character_class = {     "[a]",                Query::MatchAll, &[] },
      big_character_class = {   "[a-z]",                Query::MatchAll, &[] },
               tom_or_def = { "tom|def",  Query::or_str(["def", "tom"]), &[17, 45, 70, 76, 77, 101, 103, 117, 120, 131, 137, 153, 170] },
      asd_or_empty_string = {    "asd|",                Query::MatchAll, &[] },
             asd_dot_star = {   "asd.+",               Query::MatchNone, &[] },
             dot_star_asd = {   ".+asd",               Query::MatchNone, &[] },
             tom_dot_star = {  "tomo.+", Query::and_str(["omo", "tom"]), &[76, 77, 103, 170] },
             dot_star_tom = {  ".+tomo", Query::and_str(["omo", "tom"]), &[76, 77, 103, 170] },
  )]
  fn examples(regex: &str, expected_query: Query<'static, ()>, expected_chunks: &[u64]) {
    *TRACING;
    event!(Level::INFO, "regex `{}`", regex);
    let regex = parse(regex).unwrap();
    let trigrams = trigrams(&regex);

    let db = DatabaseRef::from(&(*MACBETH)[..]).unwrap();
    let trigrams = rewrite(&db, trigrams);
    assert_eq!(expected_query, trigrams);

    let trigrams = super::trigrams(&regex);
    let trigrams = rewrite(&db, trigrams);
    let mut eval = eval(db.chunk_count() as u64 - 1, trigrams);

    if matches!(expected_query, Query::MatchAll) {
      return;
    }

    let mut collect = vec![];
    while eval.advance().unwrap() {
      collect.push(eval.current());
    }
    assert_eq!(expected_chunks, collect);
  }
}
