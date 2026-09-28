use std::process;
use aeonlog::{Commands, Cli, KvStore};
use clap::Parser;

//use clap::{Parser, Subcommand};
// use serde::{Serialize, Deserialize};

/*
#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
enum Commands {
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
}*/

fn main() {
    let args = Cli::parse();
    // println!("{args:?}");
    let mut kvs = KvStore::new(); 
    match args.command {
        Commands::Set{key, value} => {
            if let Err(e) = kvs.set(key, value) {
                eprintln!("set failed {}", e);
                process::exit(1);
            }
        },
        Commands::Get{key} => {
           match kvs.get(key) {
               Err(e) =>  {
                    eprintln!("get failed {e}");
                    process::exit(1);
                },
                Ok(v) => {
                    if let Some(val) = v {
                        println!("{val}");
                    } else {
                        println!("Key not found");
                    }
                }
           }
        },
        Commands::Rm{key } => {
            if kvs.remove(key).is_err() {
              process::exit(1);
            }
        }
    }
}
