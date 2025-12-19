mod testlib;

use testlib::*;

#[test]
fn main() {
  cmd().arg("--help").assert().success().stdout(
    r#"Search for PATTERN in FILE

Usage: grop [OPTIONS] <COMMAND>

Commands:
  run    Search for PATTERN in FILE
  db     Build the database for FILE then print some information about it
  query  List candidate chunks for PATTERN in FILE
  help   Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help
"#,
  );
}

#[test]
fn run() {
  testlib::run().arg("--help").assert().success().stdout(
    r#"Search for PATTERN in FILE

Usage: grop run [OPTIONS] <PATTERN> <FILE>

Arguments:
  <PATTERN>  Pattern to search for
  <FILE>     File to search in

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help
"#,
  );
}

#[test]
fn db() {
  testlib::db().arg("--help").assert().success().stdout(
    r#"Build the database for FILE then print some information about it

Usage: grop db [OPTIONS] <FILE>

Arguments:
  <FILE>  File who's database to build

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help
"#,
  );
}

#[test]
fn query() {
  testlib::query().arg("--help").assert().success().stdout(
    r#"List candidate chunks for PATTERN in FILE

Usage: grop query [OPTIONS] <PATTERN> <FILE>

Arguments:
  <PATTERN>  Pattern to search for
  <FILE>     File to search in

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help
"#,
  );
}
