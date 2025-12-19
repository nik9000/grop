use std::{cmp, cmp::Ordering, collections::BTreeSet, fmt};

#[non_exhaustive]
pub enum Query<'a, M: Meta> {
  MatchAll,
  MatchNone,
  /// All chunks containing a trigram. After rewrite this *must* return at least one chunk.
  Trigram(&'a [u8; 3], M),
  Or(Vec<Query<'a, M>>),
  And(Vec<Query<'a, M>>),
}

pub trait Meta {
  fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<'a, M: Meta> Query<'a, M> {
  pub fn or(children: impl IntoIterator<Item = Self>) -> Self {
    let mut collected = BTreeSet::new();
    for q in children {
      match q {
        Self::MatchAll => return Self::MatchAll,
        Self::MatchNone => {}
        Self::And(_) | Self::Or(_) | Self::Trigram(_, _) => {
          collected.insert(q);
        }
      }
    }
    if collected.is_empty() {
      return Self::MatchNone;
    }
    if collected.len() == 1 {
      return collected.pop_first().unwrap();
    }
    Query::Or(collected.into_iter().collect::<Vec<_>>())
  }

  pub fn and(children: impl IntoIterator<Item = Self>) -> Self {
    let mut collected = BTreeSet::new();
    for q in children {
      match q {
        Self::MatchAll => {}
        Self::MatchNone => return Self::MatchNone,
        Self::And(_) | Self::Or(_) | Self::Trigram(_, _) => {
          collected.insert(q);
        }
      }
    }
    if collected.is_empty() {
      return Self::MatchAll;
    }
    if collected.len() == 1 {
      return collected.pop_first().unwrap();
    }
    Query::And(collected.into_iter().collect::<Vec<_>>())
  }
}

impl Query<'static, ()> {
  pub fn str(s: &'static str) -> Self {
    let bytes = s.as_bytes();
    let bytes: &[u8; 3] = bytes.try_into().unwrap();
    Query::Trigram(bytes, ())
  }

  pub fn or_str(children: impl IntoIterator<Item = &'static str>) -> Self {
    Query::Or(children.into_iter().map(Query::str).collect())
  }

  pub fn and_str<'a>(children: impl IntoIterator<Item = &'static str>) -> Self {
    Query::And(children.into_iter().map(Query::str).collect())
  }
}

impl Meta for () {
  fn fmt_debug(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
    Ok(())
  }
}

impl<'a, M: Meta> fmt::Debug for Query<'a, M> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::MatchAll => f.debug_struct("MatchAll").finish(),
      Self::MatchNone => f.debug_struct("MatchNone").finish(),
      Self::Trigram(trigram, meta) => Self::fmt_trigram(f, trigram, meta),
      Self::Or(subs) => Self::fmt_subs(f, "Or", subs),
      Self::And(subs) => Self::fmt_subs(f, "And", subs),
    }
  }
}

impl<'a, M: Meta> Query<'a, M> {
  /// Format the trigram as ascii text if it'd be visible
  fn fmt_trigram(f: &mut fmt::Formatter<'_>, trigram: &[u8; 3], meta: &M) -> fmt::Result {
    if trigram.iter().all(|c| b' ' < *c && *c <= b'~') {
      write!(
        f,
        "{}{}{}",
        trigram[0] as char, trigram[1] as char, trigram[2] as char
      )?;
    } else {
      write!(f, "{:x}{:x}{:x}", trigram[0], trigram[1], trigram[2])?;
    }
    meta.fmt_debug(f)
  }

  fn fmt_subs(f: &mut fmt::Formatter<'_>, name: &str, subs: &[Self]) -> fmt::Result {
    if subs.into_iter().all(Query::fmts_short) {
      Self::fmt_compact_subs(f, name, subs)
    } else {
      f.write_str(name)?;
      f.debug_list().entries(subs.iter()).finish()
    }
  }

  fn fmts_short(&self) -> bool {
    matches!(self, Self::MatchAll | Self::MatchNone | Self::Trigram(_, _))
  }

  fn fmt_compact_subs(f: &mut fmt::Formatter<'_>, name: &str, subs: &[Self]) -> fmt::Result {
    f.write_str(name)?;
    f.write_str("[")?;
    let mut first = true;
    for s in subs {
      if first {
        first = false;
      } else {
        f.write_str(", ")?;
      }
      fmt::Debug::fmt(s, f)?;
    }
    f.write_str("]")
  }
}

impl<'a, T: Meta, O: Meta> cmp::PartialEq<Query<'a, O>> for Query<'a, T> {
  fn eq(&self, other: &Query<'a, O>) -> bool {
    match (self, other) {
      (Self::MatchAll, Query::MatchAll) => true,
      (Self::MatchNone, Query::MatchNone) => true,
      (Self::Trigram(l, _), Query::Trigram(r, _)) => l == r,
      (Self::Or(l), Query::Or(r)) => l == r,
      (Self::And(l), Query::And(r)) => l == r,
      _ => false,
    }
  }
}

impl<'a, T: Meta> cmp::Eq for Query<'a, T> {}

impl<'a, T: Meta> cmp::PartialOrd for Query<'a, T> {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<'a, T: Meta> cmp::Ord for Query<'a, T> {
  fn cmp(&self, other: &Self) -> Ordering {
    match (self, other) {
      (Self::MatchAll, Self::MatchAll) => Ordering::Equal,
      (Self::MatchAll, _) => Ordering::Less,

      (Self::MatchNone, Self::MatchAll) => Ordering::Greater,
      (Self::MatchNone, Self::MatchNone) => Ordering::Equal,
      (Self::MatchNone, _) => Ordering::Less,

      (Self::Trigram(_, _), Self::MatchAll) => Ordering::Greater,
      (Self::Trigram(_, _), Self::MatchNone) => Ordering::Greater,
      (Self::Trigram(l, _), Self::Trigram(r, _)) => l.cmp(r),
      (Self::Trigram(_, _), _) => Ordering::Less,

      (Self::Or(_), Self::MatchAll) => Ordering::Greater,
      (Self::Or(_), Self::MatchNone) => Ordering::Greater,
      (Self::Or(_), Self::Trigram(_, _)) => Ordering::Greater,
      (Self::Or(l), Self::Or(r)) => l.cmp(r),
      (Self::Or(_), _) => Ordering::Less,

      (Self::And(_), Self::MatchAll) => Ordering::Greater,
      (Self::And(_), Self::MatchNone) => Ordering::Greater,
      (Self::And(_), Self::Trigram(_, _)) => Ordering::Greater,
      (Self::And(_), Self::Or(_)) => Ordering::Greater,
      (Self::And(l), Self::And(r)) => l.cmp(r),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use yare::parameterized;

  #[parameterized(
                   all_all = {                Query::MatchAll,                Query::MatchAll, Ordering::Equal   },
                  all_none = {                Query::MatchAll,               Query::MatchNone, Ordering::Less    },
                   all_asd = {                Query::MatchAll,              Query::str("asd"), Ordering::Less    },
                    all_or = {                Query::MatchAll,  Query::or_str(["asd", "def"]), Ordering::Less    },
                   all_and = {                Query::MatchAll, Query::and_str(["asd", "def"]), Ordering::Less    },

                  none_all = {               Query::MatchNone,                Query::MatchAll, Ordering::Greater },
                 none_none = {               Query::MatchNone,               Query::MatchNone, Ordering::Equal   },
              none_trigram = {               Query::MatchNone,              Query::str("asd"), Ordering::Less    },
                   none_or = {               Query::MatchNone,  Query::or_str(["asd", "def"]), Ordering::Less    },
                  none_and = {               Query::MatchNone, Query::and_str(["asd", "def"]), Ordering::Less    },

                   asd_all = {              Query::str("asd"),                Query::MatchAll, Ordering::Greater },
                  asd_none = {              Query::str("asd"),               Query::MatchNone, Ordering::Greater },
                   asd_asa = {              Query::str("asd"),              Query::str("asa"), Ordering::Greater },
                   asd_asd = {              Query::str("asd"),              Query::str("asd"), Ordering::Equal   },
                   asd_def = {              Query::str("asd"),              Query::str("def"), Ordering::Less    },
                    asd_or = {              Query::str("asd"),  Query::or_str(["asd", "def"]), Ordering::Less    },
                   asd_and = {              Query::str("asd"), Query::and_str(["asd", "def"]), Ordering::Less    },

            or_asd_def_all = {  Query::or_str(["asd", "def"]),                Query::MatchAll, Ordering::Greater },
           or_asd_def_none = {  Query::or_str(["asd", "def"]),               Query::MatchNone, Ordering::Greater },
            or_asd_def_asa = {  Query::or_str(["asd", "def"]),              Query::str("asa"), Ordering::Greater },
     or_asd_def_or_asa_def = {  Query::or_str(["asd", "def"]),  Query::or_str(["asa", "def"]), Ordering::Greater },
     or_asd_def_or_asd_def = {  Query::or_str(["asd", "def"]),  Query::or_str(["asd", "def"]), Ordering::Equal   },
     or_asd_def_or_asd_fud = {  Query::or_str(["asd", "def"]),  Query::or_str(["asd", "fud"]), Ordering::Less    },
            or_asd_def_and = {  Query::or_str(["asd", "def"]), Query::and_str(["asd", "def"]), Ordering::Less    },

           and_asd_def_all = { Query::and_str(["asd", "def"]),                Query::MatchAll, Ordering::Greater },
          and_asd_def_none = { Query::and_str(["asd", "def"]),               Query::MatchNone, Ordering::Greater },
           and_asd_def_asa = { Query::and_str(["asd", "def"]),              Query::str("asa"), Ordering::Greater },
            and_asd_def_or = { Query::and_str(["asd", "def"]),  Query::or_str(["asd", "def"]), Ordering::Greater },
    and_asd_def_or_asa_def = { Query::and_str(["asd", "def"]), Query::and_str(["asa", "def"]), Ordering::Greater },
    and_asd_def_or_asd_def = { Query::and_str(["asd", "def"]), Query::and_str(["asd", "def"]), Ordering::Equal   },
    and_asd_def_or_asd_fud = { Query::and_str(["asd", "def"]), Query::and_str(["asd", "fud"]), Ordering::Less    },
  )]
  fn cmp(lhs: Query<'static, ()>, rhs: Query<'static, ()>, expected: Ordering) {
    assert_eq!(lhs.cmp(&rhs), expected);
  }

  #[parameterized(
            all = {                         Query::MatchAll, "MatchAll" },
           none = {                        Query::MatchNone, "MatchNone" },
            asd = {                       Query::str("asd"), "asd" },
            a_d = {                       Query::str("a d"), "612064" },
        a_dot_d = {                       Query::str("a.d"), "a.d" },
         dot_ad = {                       Query::str(".ad"), ".ad" },
          ad_f3 = { Query::Trigram(&[b'a', b'd', 0xf3], ()), "6164f3" },
         fefefe = { Query::Trigram(&[0xfe, 0xfe, 0xfe], ()), "fefefe" },
     or_asd_def = {           Query::or_str(["asd", "def"]), "Or[asd, def]" },
    and_asd_def = {          Query::and_str(["asd", "def"]), "And[asd, def]" },
    or_cat_and_asd_def = {
      Query::or([Query::str("cat"), Query::and_str(["asd", "def"])]),
      r#"Or[
    cat,
    And[asd, def],
]"# 
    },
  )]
  fn debug(query: Query<'static, ()>, expected: &str) {
    assert_eq!(format!("{query:#?}"), expected);
  }
}
