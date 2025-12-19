#![allow(dead_code)]

use assert_cmd::{Command, cargo_bin};
use std::path::PathBuf;

pub fn macbeth() -> PathBuf {
  let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  d.push("..");
  d.push("..");
  d.push("testdata");
  d.push("macbeth.txt");
  d
}

pub fn cmd() -> Command {
  let mut cmd = Command::new(cargo_bin!());
  cmd.env("RUST_BACKTRACE", "1");
  cmd
}

pub fn run() -> Command {
  let mut cmd = cmd();
  cmd.arg("run");
  cmd
}

pub fn db() -> Command {
  let mut cmd = cmd();
  cmd.arg("db");
  cmd
}
