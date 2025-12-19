mod query;

pub use query::{Meta, Query};
use regex_syntax::hir::{Hir, HirKind, Literal, Repetition};
use tracing::{Level, event, span};

/// Returns a [Query] that iterates all `chunks` that *can* the regex.
pub fn trigrams<'a>(regex: &'a Hir) -> Query<'a, ()> {
  // TODO the regex crate take a ton of care not to recur. It's safer on untrusted regexes.
  let span = span!(Level::TRACE, "trigrams");
  let _guard = span.enter();
  match regex.kind() {
    HirKind::Empty => Query::MatchAll,
    HirKind::Literal(lit) => from_literal(lit),
    HirKind::Class(_) => Query::MatchAll,
    HirKind::Look(_) => {
      event!(Level::WARN, "unsupported {:?}", regex.kind());
      Query::MatchAll
    }
    HirKind::Repetition(r) => from_repetition(r),
    HirKind::Capture(c) => trigrams(&c.sub),
    HirKind::Concat(subs) => from_concat(&subs[..]),
    HirKind::Alternation(subs) => from_alternation(&subs[..]),
  }
}

fn from_literal<'a>(lit: &'a Literal) -> Query<'a, ()> {
  let span = span!(Level::TRACE, "from_literal");
  let _guard = span.enter();
  let lit = &lit.0[..];
  if lit.len() < 3 {
    return Query::MatchAll;
  }
  let end = if lit.len() >= 3 { lit.len() - 2 } else { 0 };
  Query::and((0..end).map(|from| {
    let slice = &lit[from..(from + 3)];
    Query::Trigram(slice.try_into().unwrap(), ())
  }))
}

fn from_repetition<'a>(r: &'a Repetition) -> Query<'a, ()> {
  let span = span!(Level::TRACE, "from_repetition");
  let _guard = span.enter();
  if r.max == Some(0) {
    return Query::MatchNone;
  }
  if r.min == 0 {
    return Query::MatchAll;
  }
  trigrams(&r.sub)
}

fn from_concat<'a>(subs: &'a [Hir]) -> Query<'a, ()> {
  let span = span!(Level::TRACE, "from_concat");
  let _guard = span.enter();
  Query::and(subs.iter().map(|a| trigrams(a)))
}

fn from_alternation<'a>(subs: &'a [Hir]) -> Query<'a, ()> {
  let span = span!(Level::TRACE, "from_alternation");
  let _guard = span.enter();
  Query::or(subs.iter().map(|a| trigrams(a)))
}

#[cfg(test)]
mod tests {
  use super::*;
  use arbtest::arbtest;
  use regex_syntax::{ast::Ast, hir::translate::Translator, parse};
  use std::sync::LazyLock;
  use tracing::event;
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

  #[parameterized(
                    empty = {             "", Query::MatchAll                },
                        a = {            "a", Query::MatchAll                },
                      asd = {          "asd", Query::str("asd")              },
                     asdf = {         "asdf", Query::and_str(["asd", "sdf"]) },
    small_character_class = {          "[a]", Query::MatchAll                },
      big_character_class = {        "[a-z]", Query::MatchAll                },
               asd_or_def = {      "asd|def", Query::or_str(["asd", "def"])  },
      asd_or_empty_string = {         "asd|", Query::MatchAll                },
             asd_dot_star = {        "asd.+", Query::str("asd")              },
             dot_star_asd = {        ".+asd", Query::str("asd")              },
               uncaptured = {      "(?:asd)", Query::str("asd")              },
         uncaptured_short = {       "(?:as)", Query::MatchAll                },
                 captured = {        "(asd)", Query::str("asd")              },
           captured_short = {         "(as)", Query::MatchAll                },
                  in_plus = {       "(asd)+", Query::str("asd")              },
                  in_star = {       "(asd)*", Query::MatchAll                },
              in_optional = {       "(asd)?", Query::MatchAll                },
                  in_0_10 = { "(asd){0, 10}", Query::MatchAll                },
                  in_1_10 = { "(asd){1, 10}", Query::str("asd")              },
                  in_2_10 = { "(asd){1, 10}", Query::str("asd")              },
                   concat = {   "(asd)(asd)", Query::str("asd")              },
  )]
  fn examples(regex: &str, expected: Query<'static, ()>) {
    *TRACING;
    event!(Level::INFO, "regex `{}`", regex);
    let regex = parse(regex).unwrap();
    assert_eq!(expected, trigrams(&regex));
  }

  #[test]
  fn no_crashing() {
    *TRACING;
    arbtest(|u| {
      let regex_ast: Ast = u.arbitrary()?;
      let regex_str = regex_ast.to_string();
      event!(Level::INFO, "regex `{}`", regex_str);
      let Ok(regex) = Translator::new().translate(&regex_str, &regex_ast) else {
        // arbitrary regex is invalid
        event!(Level::INFO, "unsupported");
        // panic!();
        return Ok(());
      };
      let actual = trigrams(&regex);
      event!(Level::INFO, "trigrams `{:?}`", actual);
      Ok(())
    });
  }
}
