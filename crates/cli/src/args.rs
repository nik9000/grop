use clap::*;
use clap_verbosity_flag::*;

/// Search for PATTERN in FILE.
#[derive(Parser, Debug)]
#[command(name = "grop")]
pub(crate) struct Args {
  #[command(flatten)]
  pub(crate) verbosity: Verbosity<InfoLevel>,

  #[command(subcommand)]
  pub(crate) command: Command,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
  /// Search for PATTERN in FILE.
  Run {
    /// Pattern to search for.
    pattern: String,
    /// File to search in.
    file: String,
  },
  /// Build the database for FILE then print some information about it.
  Db {
    /// File who's database to build.
    file: String,
  },
  /// List candidate chunks for PATTERN in FILE.
  Query {
    /// Pattern to search for.
    pattern: String,
    /// File to search in.
    file: String,
  },
}
