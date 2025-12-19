use super::Error;
use database::DatabaseBuilder;
use directories::BaseDirs;
use memmap2::Mmap;
use std::{fs::{self, File}, io, os::unix::fs::MetadataExt, path::{self, PathBuf}};
use tracing::{Level, event, span};
use humansize::{format_size, BINARY};

use database::DatabaseRef;

const LINES_PER_CHUNK: u32 = 128 * 1024;
const BYTES_PER_CHUNK: u32 = 128 * 1024;

pub(crate) fn run(file: String) -> Result<(), Error> {
  let span = span!(Level::TRACE, "run");
  let _guard = span.enter();

  let directories = directories::BaseDirs::new().ok_or(Error::NoHome)?;
  let (path, file) = crate::target_file::open(file)?;
  let file_metadata = fs::metadata(&path)?;
  
  let db = crate::db::make(&directories, &path, &file)?;
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

pub(crate) fn make(directories: &BaseDirs, path: &PathBuf, file: &File) -> Result<Mmap, Error> {
  let span = span!(Level::TRACE, "make");
  let _guard = span.enter();

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

  let mut db_dir = db.clone();
  db_dir.pop();
  fs::create_dir_all(db_dir)?;

  if db.exists() == false {
    // TODO there's a race here. also with creation. we can fix it later
    event!(Level::DEBUG, "creating db at {}", db.to_string_lossy());
    let mut writer = File::create_new(&db)?;
    DatabaseBuilder::from_lines(
      &mut io::BufReader::new(file),
      LINES_PER_CHUNK,
      BYTES_PER_CHUNK,
    )?
    .write(&mut writer)?;
  }
  let read = File::open(&db)?;
  Ok(unsafe { memmap2::Mmap::map(&read) }?)
}
