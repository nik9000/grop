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
  trigram_inventory_breakdown(&db);
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

fn trigram_inventory_breakdown(db: &DatabaseRef<'_>) {
  let total = db.trigram_count();
  let total_bytes = db.inventory_size();
  let mut in_1 = 0;
  let mut lt_001_pct = 0;
  let mut lt_005_pct = 0;
  let mut lt_010_pct = 0;
  let mut lt_020_pct = 0;
  let mut lt_050_pct = 0;
  let mut lt_080_pct = 0;
  let mut lt_100_pct = 0;
  let mut eq_100_pct = 0;
  let mut in_1_bytes = 1;
  let mut lt_001_pct_bytes = 0;
  let mut lt_005_pct_bytes = 0;
  let mut lt_010_pct_bytes = 0;
  let mut lt_020_pct_bytes = 0;
  let mut lt_050_pct_bytes = 0;
  let mut lt_080_pct_bytes = 0;
  let mut lt_100_pct_bytes = 0;
  let mut eq_100_pct_bytes = 0;
  for i in 0..total {
    let chunks = db.chunks_containing_by_ord(i);
    let bytes = chunks.byte_count();
    let count = chunks.into_iter().count();
    if count == 1 {
      in_1 += 1;
      in_1_bytes += bytes;
      continue;
    }
    let count = count as f64;
    let pct = count / db.chunk_count() as f64;
    if pct < 0.01 {
      lt_001_pct += 1;
      lt_001_pct_bytes += bytes;
    } else if pct < 0.05 {
      lt_005_pct += 1;
      lt_005_pct_bytes += bytes;
    } else if pct < 0.10 {
      lt_010_pct += 1;
      lt_010_pct_bytes += bytes;
    } else if pct < 0.20 {
      lt_020_pct += 1;
      lt_020_pct_bytes += bytes;
    } else if pct < 0.50 {
      lt_050_pct += 1;
      lt_050_pct_bytes += bytes;
    } else if pct < 0.80 {
      lt_080_pct += 1;
      lt_080_pct_bytes += bytes;
    } else if pct < 1.0 {
      lt_100_pct += 1;
      lt_100_pct_bytes += bytes;
    } else {
      eq_100_pct += 1;
      eq_100_pct_bytes += bytes;
    }
  }
  let in_1_pct = in_1 as f64 / total as f64 * 100.0;
  let lt_001_pct_pct = lt_001_pct as f64 / total as f64 * 100.0;
  let lt_005_pct_pct = lt_005_pct as f64 / total as f64 * 100.0;
  let lt_010_pct_pct = lt_010_pct as f64 / total as f64 * 100.0;
  let lt_020_pct_pct = lt_020_pct as f64 / total as f64 * 100.0;
  let lt_050_pct_pct = lt_050_pct as f64 / total as f64 * 100.0;
  let lt_080_pct_pct = lt_080_pct as f64 / total as f64 * 100.0;
  let lt_100_pct_pct = lt_100_pct as f64 / total as f64 * 100.0;
  let eq_100_pct_pct = eq_100_pct as f64 / total as f64 * 100.0;
  let in_1_bytes_pct = in_1_bytes as f64 / total_bytes as f64 * 100.0;
  let lt_001_pct_bytes_pct = lt_001_pct_bytes as f64 / total_bytes as f64 * 100.0;
  let lt_005_pct_bytes_pct = lt_005_pct_bytes as f64 / total_bytes as f64 * 100.0;
  let lt_010_pct_bytes_pct = lt_010_pct_bytes as f64 / total_bytes as f64 * 100.0;
  let lt_020_pct_bytes_pct = lt_020_pct_bytes as f64 / total_bytes as f64 * 100.0;
  let lt_050_pct_bytes_pct = lt_050_pct_bytes as f64 / total_bytes as f64 * 100.0;
  let lt_080_pct_bytes_pct = lt_080_pct_bytes as f64 / total_bytes as f64 * 100.0;
  let lt_100_pct_bytes_pct = lt_100_pct_bytes as f64 / total_bytes as f64 * 100.0;
  let eq_100_pct_bytes_pct = eq_100_pct_bytes as f64 / total_bytes as f64 * 100.0;

  println!(
    "    trigrams in 1 chunk: {in_1:>10} ({in_1_pct:05.2}% of inventory) ({in_1_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in <001%: {lt_001_pct:>10} ({lt_001_pct_pct:05.2}% of inventory) ({lt_001_pct_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in <005%: {lt_005_pct:>10} ({lt_005_pct_pct:05.2}% of inventory) ({lt_005_pct_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in <010%: {lt_010_pct:>10} ({lt_010_pct_pct:05.2}% of inventory) ({lt_010_pct_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in <020%: {lt_020_pct:>10} ({lt_020_pct_pct:05.2}% of inventory) ({lt_020_pct_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in <050%: {lt_050_pct:>10} ({lt_050_pct_pct:05.2}% of inventory) ({lt_050_pct_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in <080%: {lt_080_pct:>10} ({lt_080_pct_pct:05.2}% of inventory) ({lt_080_pct_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in <100%: {lt_100_pct:>10} ({lt_100_pct_pct:05.2}% of inventory) ({lt_100_pct_bytes_pct:05.2}% of inventory bytes)"
  );
  println!(
    "      trigrams in =100%: {eq_100_pct:>10} ({eq_100_pct_pct:05.2}% of inventory) ({eq_100_pct_bytes_pct:05.2}% of inventory bytes)"
  );
}
