use md5::{Digest, Md5};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::fs::{canonicalize, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

const BUFFER_SIZE: usize = 4_194_304; // 4 MiB
const MAX_FILE_SIZE: u64 = 4_294_967_296; // 4 GiB

const MD5SUM: &'static str = "/usr/bin/md5sum";
const SHASUM: &'static str = "/usr/bin/shasum";
const SHA256SUM: &'static str = "/usr/bin/sha256sum";

fn process<D: Digest + Default, R: Read>(reader: &mut R, digest: &mut String) {
    let mut sh = D::default();
    let mut buffer = [0u8; BUFFER_SIZE];
    loop {
        let n = match reader.read(&mut buffer) {
            Ok(n) => n,
            Err(_) => return,
        };
        sh.update(&buffer[..n]);
        if n == 0 || n < BUFFER_SIZE {
            break;
        }
    }
    for byte in &sh.finalize() {
        digest.push_str(format!("{:02x}", byte).as_str());
    }
}

fn systime2unix(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum HashAlgorithm {
    Unknown,
    Md5,
    Sha1,
    Sha256,
}

impl HashAlgorithm {
    pub fn from_str(s: &str) -> HashAlgorithm {
        match s {
            "md5" => HashAlgorithm::Md5,
            "sha1" => HashAlgorithm::Sha1,
            "sha256" => HashAlgorithm::Sha256,
            _ => HashAlgorithm::Unknown,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "md5",
            HashAlgorithm::Sha1 => "sha1",
            HashAlgorithm::Sha256 => "sha256",
            _ => "unknown",
        }
    }

    pub fn external_command(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => MD5SUM,
            HashAlgorithm::Sha1 => SHASUM,
            HashAlgorithm::Sha256 => SHA256SUM,
            HashAlgorithm::Unknown => "",
        }
    }
}

impl fmt::Debug for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HashAlgorithm::{}", self.to_str())
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum HashMethod {
    Unknown,
    Native,
    External,
}

impl HashMethod {
    pub fn from_str(s: &str) -> HashMethod {
        match s {
            "native" => HashMethod::Native,
            "external" => HashMethod::External,
            _ => HashMethod::Unknown,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            HashMethod::Native => "native",
            HashMethod::External => "external",
            _ => "unknown",
        }
    }
}

impl fmt::Debug for HashMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HashMethod::{}", self.to_str())
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Hash {
    algo: HashAlgorithm,
    method: HashMethod,
}

impl Hash {
    pub fn new(algo: &str, method: &str) -> Hash {
        Hash {
            algo: HashAlgorithm::from_str(&algo),
            method: HashMethod::from_str(&method),
        }
    }

    fn external_sum(&self, path: &str, digest: &mut String) {
        let command = self.algo.external_command();
        let output = Command::new(command)
            .arg(path)
            .output()
            .expect("hash command failed");
        digest.push_str(
            &String::from_utf8(output.stdout)
                .unwrap()
                .split_whitespace()
                .nth(0)
                .unwrap()
                .to_string(),
        );
    }

    fn native_sum(&self, file: &mut std::fs::File, digest: &mut String) {
        match self.algo {
            HashAlgorithm::Md5 => process::<Md5, _>(file, digest),
            HashAlgorithm::Sha1 => process::<Sha1, _>(file, digest),
            HashAlgorithm::Sha256 => process::<Sha256, _>(file, digest),
            HashAlgorithm::Unknown => {}
        };
    }

    pub fn process(&self, file: &mut std::fs::File, path: &str, digest: &mut String) {
        match self.method {
            HashMethod::Native => self.native_sum(file, digest),
            HashMethod::External => self.external_sum(path, digest),
            HashMethod::Unknown => {}
        }
    }
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct IndexedFile {
    path: String,
    sum: String,
    size: u64,
    modified: SystemTime,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Duplicate {
    sum: String,
    num: u64,
    list: Vec<IndexedFile>,
}

pub struct SqliteIndex {
    db_conn: Connection,
}

impl SqliteIndex {
    pub fn new(path: String) -> SqliteIndex {
        let conn = Connection::open(&path).expect("error while opening the database");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                      path            TEXT PRIMARY KEY,
                      sum             TEXT NOT NULL,
                      size            INTEGER,
                      modified        INTEGER
                      )",
            [],
        )
        .expect("`create table files` request failed");
        SqliteIndex { db_conn: conn }
    }

    fn _index_file(&mut self, path: PathBuf, hash: Hash, force: bool) -> Result<(), ()> {
        let path_str = canonicalize(path.as_path())
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let metadata = fs::metadata(&path).unwrap();
        let mut file = File::open(path).unwrap();
        let mut digest = String::from("");

        let modified = match self.list(Some(&path_str.clone())).unwrap().first() {
            Some(f) => systime2unix(f.modified) != systime2unix(metadata.modified().unwrap()),
            None => true,
        };
        if modified || force {
            println!("Indexing {:?} ...", path_str);
            if metadata.len() < MAX_FILE_SIZE {
                hash.process(&mut file, path_str.clone().as_str(), &mut digest)
            }
            let f = IndexedFile {
                path: path_str,
                sum: digest,
                size: metadata.len(),
                modified: metadata.modified().unwrap(),
            };
            self.db_conn
                .execute(
                    "REPLACE INTO files(path, sum, size, modified) VALUES (?1, ?2, ?3, ?4)",
                    params![f.path, f.sum, f.size, systime2unix(f.modified)],
                )
                .expect("`replace into files` request failed");
        }
        Ok(())
    }

    pub fn index(
        &mut self,
        path: PathBuf,
        hash: Hash,
        recursive: bool,
        force: bool,
    ) -> Result<(), ()> {
        let abspath = canonicalize(path.as_path()).unwrap();
        let metadata = fs::metadata(&abspath).unwrap();
        if metadata.is_file() {
            self._index_file(abspath, hash, force)?;
        } else {
            // build a hashmap to flag the files in DB to be deleted
            let mut files_status = HashMap::new();
            for file in self
                .list(Some(&format!("{}/%", abspath.to_string_lossy())))
                .unwrap()
            {
                let filepath = file;
                if Path::new(&filepath.path.clone()).parent().unwrap() == abspath {
                    files_status.insert(filepath.path, true);
                }
            }

            for entry in fs::read_dir(abspath).unwrap() {
                let entry = entry.unwrap();
                let entry_path = entry.path();
                let metadata = fs::metadata(&entry_path).unwrap();

                if metadata.is_dir() && !recursive {
                    continue;
                }
                // mark seen files as non-deleted
                match files_status.get_mut(entry_path.to_str().unwrap()) {
                    Some(deleted) => *deleted = false,
                    None => {} // first seen, do nothing
                };
                self.index(entry_path, hash, recursive, force)?;
            }

            // remove from DB deleted files
            for (file, deleted) in files_status.iter() {
                if *deleted {
                    self.delete(file);
                }
            }
        }

        Ok(())
    }

    pub fn delete(&mut self, path: &str) {
        let path_to_delete = path;
        self.db_conn
            .execute(
                "DELETE FROM files WHERE path like (?)",
                params!(path_to_delete),
            )
            .expect("`delete from files` request failed");
    }

    pub fn list(&mut self, limit_path: Option<&str>) -> Result<Vec<IndexedFile>, ()> {
        let path = match limit_path {
            Some(p) => p,
            None => "%",
        };
        let mut stmt = self
            .db_conn
            .prepare("SELECT path, sum, size, modified FROM files WHERE path like (?)")
            .expect("`select [...] from files` prepare failed");
        let file_iter = stmt
            .query_map(params!(path), |row| {
                Ok(IndexedFile {
                    path: row.get(0)?,
                    sum: row.get(1)?,
                    size: row.get(2)?,
                    modified: SystemTime::UNIX_EPOCH
                        .checked_add(Duration::new(row.get(3)?, 0))
                        .unwrap(),
                })
            })
            .unwrap();
        Ok(file_iter.flatten().collect())
    }

    pub fn list_duplicates(&mut self) -> Result<Vec<Duplicate>, ()> {
        let mut sum_stmt = self
            .db_conn
            .prepare("SELECT sum, COUNT(*) c FROM files GROUP BY sum HAVING c > 1")
            .expect("`SELECT sum, COUNT(*) c FROM files GROUP BY sum HAVING c > 1` prepare failed");
        let duplicates_iter = sum_stmt
            .query_map([], |row| {
                Ok(Duplicate {
                    sum: row.get(0)?,
                    num: row.get(1)?,
                    list: vec![],
                })
            })
            .unwrap();

        let mut duplicates = vec![];

        for dup in duplicates_iter {
            let mut duplicate = dup.unwrap();
            let mut stmt = self
                .db_conn
                .prepare("SELECT path, sum, size, modified FROM files WHERE sum like (?)")
                .expect("`select [...] from files` prepare failed");
            let file_iter = stmt
                .query_map(params!(duplicate.sum), |row| {
                    Ok(IndexedFile {
                        path: row.get(0)?,
                        sum: row.get(1)?,
                        size: row.get(2)?,
                        modified: SystemTime::UNIX_EPOCH
                            .checked_add(Duration::new(row.get(3)?, 0))
                            .unwrap(),
                    })
                })
                .unwrap();

            for file in file_iter {
                duplicate.list.push(file.unwrap())
            }
            duplicates.push(duplicate)
        }

        Ok(duplicates)
    }
}
