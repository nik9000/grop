mod testlib;

use testlib::*;

#[test]
fn bad_regex() {
  query().args(["test[", "test"]).assert().failure().stderr(
    r#"regex parse error:
    test[
        ^
error: unclosed character class
"#,
  );
}

#[test]
fn missing_file() {
  query()
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
  query()
    .arg("Tomorrow, and tomorrow, and tomorrow")
    .arg(macbeth())
    .assert()
    .success()
    .stdout(r#"regex query And[20616e(1), 20746f(1), 2c2061(1), Tom(1), and(1), 642074(1), mor(1), 6e6420(1), omo(1), orr(1), ow,(1), row(1), rro(1), tom(1), 772c20(1)]
candidate chunk: 0/1  000000 -> 116138
matched: 1/1 (100.00%)
"#);
}

#[test]
fn outs() {
  query()
    .arg("Out,.+(?:spot|candle)")
    .arg(macbeth())
    .assert()
    .success()
    .stdout(
      r#"regex query And[
    Or[
        And[and(1), can(1), dle(1), ndl(1)],
        And[pot(1), spo(1)],
    ],
    And[Out(1), ut,(1)],
]
candidate chunk: 0/1  000000 -> 116138
matched: 1/1 (100.00%)
"#,
    );
}
