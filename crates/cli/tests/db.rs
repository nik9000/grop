mod testlib;

use testlib::*;

#[test]
fn missing_file() {
  db().arg("notfound").assert().failure().stderr(format!(
    "file not found: {}/notfound\n",
    env!("CARGO_MANIFEST_DIR")
  ));
}

#[test]
fn macbeth() {
  db().arg(testlib::macbeth()).assert().success().stdout(
    r#"                 chunks: 1
               trigrams: 6129
              file size: 113.42 KiB
                db size:  65.94 KiB (58.14% of file)
      trigrams map size:  35.98 KiB (54.56% of db)
trigrams inventory size:  29.93 KiB (45.39% of db)
        chunk ends size:        4 B (00.01% of db)
 chunk line counts size:        4 B (00.01% of db)
"#,
  );
}
