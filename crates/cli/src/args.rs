use bytesize::ByteSize;
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
  Run(#[command(flatten)] Full),
  /// Build the database for FILE then print some information about it.
  Db {
    /// File who's database to build.
    file: String,
    #[command(flatten)]
    db: Db,
  },
  /// List candidate chunks for PATTERN in FILE.
  Query(#[command(flatten)] Full),
}

#[derive(Args, Debug)]
pub(crate) struct Full {
  /// Pattern to search for.
  pub(crate) pattern: String,
  /// File to search in.
  pub(crate) file: String,

  #[command(flatten)]
  pub(crate) db: Db,
}

#[derive(Args, Debug)]
#[group(skip)]
pub(crate) struct Db {
  /// Chunk target byte size.
  #[arg(long, short = 'b', default_value = "128KiB")]
  pub(crate) chunk_bytes: ByteSize,
  /// Chunk target line count.
  #[arg(long, short = 'l', default_value = "1000000")]
  pub(crate) chunk_lines: u32,
}
