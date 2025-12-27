use super::{Error, Result, args};
use database::DatabaseBuilder;
use humansize::{BINARY, format_size};
use memmap2::Mmap;
use std::{
  fs::{self, File, OpenOptions},
  io,
  os::unix::fs::MetadataExt,
  path::{self, PathBuf},
  thread::sleep,
  time::Duration,
};
use tracing::{Level, event, span};

use database::DatabaseRef;

const MAX_SLEEPS: u32 = 50;

pub(crate) fn run(file: String, db_args: args::Db) -> Result<()> {
  let span = span!(Level::TRACE, "run");
  let _guard = span.enter();

  let (path, file) = crate::target_file::open(file)?;
  let file_metadata = fs::metadata(&path)?;

  let db = crate::db::open_or_build(&path, &file, db_args)?;
  let db_len = db.len();
  let db_len_percent = (db_len as f64) / (file_metadata.size() as f64) * 100.0;

  // NOCOMMIT if the file has an error, rebuild it.
  let db = DatabaseRef::from(&db[..]).map_err(|e| Error::DatabaseReadError(format!("{}", e)))?;

  let file_size = format_size(file_metadata.size(), BINARY);
  let db_size = format_size(db_len, BINARY);
  let chunk_count = db.chunk_count();
  let trigram_count = db.trigram_count();
  let trigr_map_size = format_size(db.map_size(), BINARY);
  let trigr_map_percent = (db.map_size() as f64) / (db_len as f64) * 100.0;
  let trigr_inv_size = format_size(db.inventory_size(), BINARY);
  let trigr_inv_percent = (db.inventory_size() as f64) / (db_len as f64) * 100.0;
  let chunk_end_size = format_size(db.chunk_end_offsets_size(), BINARY);
  let chunk_end_percent = (db.chunk_end_offsets_size() as f64) / (db_len as f64) * 100.0;
  let chunk_lin_size = format_size(db.chunk_end_line_counts_size(), BINARY);
  let chunk_lin_percent = (db.chunk_end_line_counts_size() as f64) / (db_len as f64) * 100.0;

  println!("                 chunks: {chunk_count}");
  println!("               trigrams: {trigram_count}");
  println!("              file size: {file_size}");
  println!("                db size: {db_size:>10} ({db_len_percent:05.2}% of file)");
  println!("      trigrams map size: {trigr_map_size:>10} ({trigr_map_percent:05.2}% of db)");
  println!("trigrams inventory size: {trigr_inv_size:>10} ({trigr_inv_percent:05.2}% of db)");
  println!("        chunk ends size: {chunk_end_size:>10} ({chunk_end_percent:05.2}% of db)");
  println!(" chunk line counts size: {chunk_lin_size:>10} ({chunk_lin_percent:05.2}% of db)");
  Ok(())
}

pub(crate) fn open_or_build(path: &PathBuf, file: &File, db_args: args::Db) -> Result<Mmap> {
  let span = span!(Level::TRACE, "make");
  let _guard = span.enter();

  let (db, lock) = db_and_lock_files(path)?;

  let mut db_dir = db.clone();
  db_dir.pop();
  fs::create_dir_all(db_dir)?;

  let mut sleeps = 0;
  loop {
    let db_exists = db.exists();
    let lock_exists = lock.exists();

    if lock_exists {
      backoff(&mut sleeps)?;
      continue;
    }
    if db_exists {
      event!(Level::DEBUG, "loading db at {}", db.to_string_lossy());
      let read = File::open(&db)?;
      return Ok(unsafe { memmap2::Mmap::map(&read) }?);
    }

    event!(Level::DEBUG, "taking lock");
    if !touch(&lock)? {
      backoff(&mut sleeps)?;
      continue;
    }

    let build_db = build(&db, file, &db_args);
    fs::remove_file(&lock)?;
    build_db?;
  }
}

fn db_and_lock_files(path: &PathBuf) -> Result<(PathBuf, PathBuf)> {
  let directories = directories::BaseDirs::new().ok_or(Error::NoHome)?;
  let mut db = directories.data_local_dir().to_path_buf();
  db.push("grop");
  db.push("db");
  let c = fs::canonicalize(path)?;
  let mut c = c.components();
  assert_eq!(Some(path::Component::RootDir), c.next());
  for c in c {
    assert!(
      matches!(c, path::Component::Normal(_)),
      "expected Normal but was {:?}",
      c
    );
    db.push(c);
  }

  let mut lock = db.clone();
  assert!(lock.add_extension("lock"));

  Ok((db, lock))
}

fn touch(path: &PathBuf) -> Result<bool> {
  match OpenOptions::new().create_new(true).write(true).open(path) {
    Ok(_) => Ok(true),
    Err(e) => match e.kind() {
      io::ErrorKind::AlreadyExists => Ok(false),
      _ => Err(Error::IO(e)),
    },
  }
}

fn build(db: &PathBuf, file: &File, db_args: &args::Db) -> Result<()> {
  event!(Level::DEBUG, "creating db at {}", db.to_string_lossy());
  let mut writer = File::create_new(&db)?;
  DatabaseBuilder::from_lines(
    &mut io::BufReader::new(file),
    db_args.chunk_lines,
    db_args.chunk_bytes.0 as u32,
  )?
  .write(&mut writer)?;
  Ok(())
}

fn backoff(sleeps: &mut u32) -> Result<()> {
  event!(Level::DEBUG, "another process is building the db");
  if *sleeps > MAX_SLEEPS {
    return Err(Error::OtherProcessBuilder);
  }
  *sleeps += 1;
  sleep(Duration::from_millis(100));
  Ok(())
}
