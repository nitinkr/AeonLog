use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions};
use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use std::io::{BufReader, BufRead, Seek, SeekFrom, Write};

#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Commands {
    Set {
        key: String,
        value: String,
    },

    Get {
        key: String,
    },
    
    Rm {
        key: String,
    },
}

pub type Result<T, E = std::io::Error> = std::result::Result<T, E>; 

pub struct KvStore {
    // kvs: HashMap<String, String>,
    index : HashMap<String, u64>,
    file_path: String,
    file_handle: File, 
    is_stale: bool,
}

impl KvStore {
    const DEFAULT_PATH: &'static str = "wal.log";
    fn get_file_path() -> String {
        env::current_dir().unwrap().join(KvStore::DEFAULT_PATH).to_str().unwrap().to_string()
    }

    pub fn new() -> Self {
        let file_path = KvStore::get_file_path();
        KvStore {
            // kvs: HashMap::new(),
            index: HashMap::new(),
            file_path: file_path.clone(),
            file_handle: OpenOptions::new()
                                .create(true)
                                .read(true)
                                .append(true)
                                .open(file_path)
                                .expect("unable to open file"),
            is_stale: true,
        }
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let offset = self.file_handle.seek(SeekFrom::End(0)).unwrap();
        self.index.insert(key.clone(), offset);
        let cmd = Commands::Set{key, value};
        self.append_to_file(cmd)
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        self.load()?;
        self.read_from_file(&key)
    }
   
    fn read_from_file(&mut self, key: &str) -> Result<Option<String>> {
        if let Some(&offset) = self.index.get(key) {
            self.file_handle.seek(SeekFrom::Start(offset)).expect("file IO failure: not able to seek");
            let mut reader = BufReader::new(&mut self.file_handle);
            let mut line = String::new();
            reader.read_line(&mut line)?;

            match serde_json::from_str(&line)? {
                Commands::Set{key: _, value} => {
                    return Ok(Some(value));
                },

                Commands::Rm{key: _} => {
                    return Ok(None);
                },
                _ => { panic!("wrong data in wal!"); }
            }
        }
        Ok(None)
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        self.load()?;
        if self.index.remove(&key).is_none() {
            println!("Key not found");
            return Err(
                std::io::Error::new(std::io::ErrorKind::InvalidData, "key not found ")); 
        }
        let cmd = Commands::Rm { key };
        self.append_to_file(cmd)
    }

    pub fn open<T: Into<PathBuf>>(path: T) -> Result<KvStore> {
        let new_path = path.into().join(KvStore::DEFAULT_PATH);

        let new_fp = match new_path.to_str() {
            Some(fp) => fp.to_string(),
            None => {
                        return Err(
                                std::io::Error::new(std::io::ErrorKind::InvalidData,
                                "Path is not string"
                                ));
                    }, 
        };
        let new_file = match OpenOptions::new()
                                        .create(true)
                                        .read(true)
                                        .append(true)
                                        .open(new_path) {
                                            Ok(f) => f,
                                            Err(e) => {
                                                return Err(
                                                        std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
                                                    );
                                            },
                                        };

        Ok(KvStore {
            index: HashMap::new(),
            file_path: new_fp,
            file_handle: new_file,
            is_stale: true,
        })
    }

    fn append_to_file(&mut self, cmd: Commands) -> Result<()> {
        let cmd_str = match serde_json::to_string(&cmd) {
            Ok(s) => s,
            Err(e) => {
                return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData, e));
            },
        };
        writeln!(self.file_handle, "{cmd_str}")
    }

    fn load_index(&mut self) -> Result<()> {
        let mut reader = BufReader::new(&mut self.file_handle); 
        reader.rewind()?;
        let mut line = String::new();
        let mut offset: u64 = 0;
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break;
            }
            match serde_json::from_str(&line)? {
                Commands::Set {key, value: _ } => {
                    self.index.insert(key, offset);
                },
                Commands::Rm {key} => {
                    self.index.remove(&key);
                },
                _ => {
                    eprintln!("wal has unwanted lines: get"); 
                }
            }
            offset += bytes_read as u64;
        }
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        if self.is_stale {
            self.is_stale = false;
            self.load_index()?;
        }
        Ok(())
    }

}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}
