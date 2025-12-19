mod testlib;

use testlib::*;

#[test]
fn bad_regex() {
  run().args(["test[", "test"]).assert().failure().stderr(
    r#"regex parse error:
    test[
        ^
error: unclosed character class
"#,
  );
}

#[test]
fn missing_file() {
  run()
    .args(["test", "notfound"])
    .assert()
    .failure()
    .stderr(format!(
      "file not found: {}/notfound\n",
      env!("CARGO_MANIFEST_DIR")
    ));
}

#[test]
fn tomorrow() {
  run()
    .arg("Tomorrow, and tomorrow, and tomorrow")
    .arg(macbeth())
    .assert()
    .success()
    .stdout("2726:    Tomorrow, and tomorrow, and tomorrow\n");
}

#[test]
fn outs() {
  run()
    .arg("Out,.+(?:spot|candle)")
    .arg(macbeth())
    .assert()
    .success()
    .stdout(
      r#"2466:  LADY MACBETH. Out, damned spot! Out, I say! One- two -why then
2730:    The way to dusty death. Out, out, brief candle!
"#,
    );
}
