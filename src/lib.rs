use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions};
use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use std::io::Write;

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
    kvs: HashMap<String, String>,
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
            kvs: HashMap::new(),
            file_path: file_path.clone(),
            file_handle: OpenOptions::new().create(true).append(true).open(file_path).expect("unable to open file"),
            is_stale: true,
        }
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        self.set_in_mem(key.clone(), value.clone());
        self.is_stale = true;
        let cmd = Commands::Set{key, value};
        self.append_to_file(cmd)
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        self.load()?;
        if let Some(v) = self.kvs.get(&key) {
            // println!("key {key} found: {}", v);
            return Ok(Some(v.clone()));
        }
        Ok(None)
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        self.load()?;
        if self.kvs.remove(&key).is_none() {
            println!("Key not found");
            return Err(
                std::io::Error::new(std::io::ErrorKind::InvalidData, "key not found ")); 
        }
        self.is_stale = true;
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
        // println!("IIIIIIIIIIIIIIIIIIIfile path open {new_fp}");
        let new_file = match OpenOptions::new()
                                        .create(true)
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
            kvs: HashMap::new(),
            file_path: new_fp,
            file_handle: new_file,
            is_stale: true,
        })
    }

    fn set_in_mem(&mut self, key: String, value: String) {
        self.kvs.insert(key, value);
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

    fn load(&mut self) -> Result<()> {
        if self.is_stale {
            let content = fs::read_to_string(&self.file_path)?;
            for line in content.lines() {
                match serde_json::from_str(line)? {
                    Commands::Set{key, value} => {
                        self.set_in_mem(key, value);
                    },
                    Commands::Rm{key} => {
                        if self.kvs.remove(&key).is_none() {
                            eprintln!("[In Load]: not able to remove the {key}: NOT FOUND");
                        }
                    },
                    Commands::Get { key } => {
                        eprintln!("Get foun in WAL: Get {key}");
                    }
                }
            }
            self.is_stale = false;
        }
        Ok(())
    }

}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}
