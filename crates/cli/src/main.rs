mod args;
mod db;
mod query;
mod run;
mod target_file;

use clap::Parser;
use std::{io, path::PathBuf, process};

use args::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error(transparent)]
  RegexSyntax(#[from] regex_syntax::Error),
  #[error("file not found: {0}")]
  NotFound(PathBuf),
  #[error(transparent)]
  IO(#[from] io::Error),
  #[error("no valid home directory")]
  NoHome,
  #[error("database read error: {0}")]
  DatabaseReadError(String),
  #[error(transparent)]
  DatabaseDecodeError(#[from] int::DecodeError),
}

fn main() {
  let args = args::Args::parse();
  tracing_subscriber::fmt()
    .with_max_level(args.verbosity)
    .init();

  let r = match args.command {
    Command::Run(Full { pattern, file, db }) => run::run(pattern, file, db),
    Command::Db { file, db } => db::run(file, db),
    Command::Query(Full { pattern, file, db }) => query::run(pattern, file, db),
  };
  if let Err(e) = r {
    eprintln!("{}", e);
    process::exit(1);
  }
}
