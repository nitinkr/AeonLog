use std::process;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
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
}

fn main() {
    let args = Cli::parse();
    println!("{args:?}");

    match args.command {
        Commands::Set{key: _, value: _} => {
            eprintln!("unimplemented");
        },
        Commands::Get{key: _} => {
            eprintln!("unimplemented");
        },
        Commands::Rm{key: _} => {
            eprintln!("unimplemented");
        }
    }
    process::exit(1);
}
