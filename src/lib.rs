use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions, read_to_string};
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
    num_set: usize,
}

impl KvStore {
    const DEFAULT_PATH: &'static str = "wal.log";
    fn get_file_path() -> String {
        env::current_dir().unwrap().join(KvStore::DEFAULT_PATH).to_str().unwrap().to_string()
    }

    pub fn new() -> Self {
        let file_path = KvStore::get_file_path();
        let mut store = KvStore {
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
            num_set : 0,
        };
        store.load().expect("load failure");
        store
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        if self.num_set > 2*self.index.len() {
            self.compact();
        }
        let offset = self.file_handle.seek(SeekFrom::End(0)).unwrap();
        self.index.insert(key.clone(), offset);
        let cmd = Commands::Set{key, value};
        self.num_set += 1;
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
        self.num_set += 1;
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

        let mut store = KvStore {
            index: HashMap::new(),
            file_path: new_fp,
            file_handle: new_file,
            is_stale: true,
            num_set: 0,
        };
        store.load().expect("load failure");
        Ok(store)
    }
    

    pub fn print_wal(&mut self, s: &str) {
        let content = fs::read_to_string(&self.file_path).unwrap();
        eprintln!("WAL#####################  {s}\n: {}",content); 
        eprintln!("INDEX MAP_____: {:#?}", self.index);
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

    fn compact(&mut self) {
        if self.index.is_empty() {
            return;
        }

        // create a new file 
        //let compact_file = KvStore::get_file_path() + ".comp";
        let compact_file = format!("{}.comp", self.file_path);
        let compact_file_path = PathBuf::from(&compact_file); 
        let file_err = OpenOptions::new().create(true).truncate(true).write(true).open(&compact_file_path); 
        if file_err.is_err() {
            eprintln!("[compaction failure] {}", file_err.err().unwrap());
            return;
        }
        let mut bytes_read: u64 = 0;
        let mut file_comp = file_err.unwrap();
        let content = fs::read_to_string(&self.file_path).unwrap();
        // eprintln!("file_comp {}", compact_file);
        let mut compact_index = HashMap::<String, u64>::new();
        for line in content.split_inclusive('\n') {
            match serde_json::from_str(line).expect("form_str failed: {line} ") {
                Commands::Set{key, value: _} => {
                    // eprintln!("[compaction] encountered {key} and current bytes read: {bytes_read}");
                    if let Some(&off) = self.index.get(&key) && off == bytes_read {
                        let of = file_comp.seek(SeekFrom::End(0)).unwrap();
                        // eprintln!("[compaction] compating {key} and current bytes read: {bytes_read} at ne loc: {of}");
                        compact_index.insert(key.clone(), of);
                        // eprintln!("line length: {}, contents: {:?}", line.len(), line);
                        // write!(file_comp, "{line}").unwrap();
                        //eprintln!("file size after write: {}", file_comp.metadata().unwrap().len());
                        if write!(file_comp, "{line}").is_err() {
                            return;
                        }
                    }
                },
                Commands::Rm{key} => {
                    if let Some(&off) = self.index.get(&key) && off == bytes_read {
                    }
                },
                _ => {
                    eprintln!("invalid cmds in wal");
                },
            }
            bytes_read += line.len() as u64;
        }

        if fs::rename(compact_file_path, PathBuf::from(&self.file_path)).is_err() {
           return;
        }

        self.file_handle = OpenOptions::new()
                                        .create(true)
                                        .read(true)
                                        .append(true)
                                        .open(&self.file_path).expect(" load failure ");
        self.index = compact_index;
        self.num_set = self.index.len();
    }

    fn load_index(&mut self) -> Result<()> {
        let mut reader = BufReader::new(&mut self.file_handle); 
        reader.rewind()?;
        let mut line = String::new();
        let mut offset: u64 = 0;
        self.index.clear();
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
        self.num_set = self.index.len();
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        if self.is_stale {
            self.is_stale = false;
            self.load_index()?;
            self.compact();
            self.num_set = self.index.len();
        }
        Ok(())
    }

}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}
