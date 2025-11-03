use std::path::Path;

// log and env_logger crates for simple logging
use log::{info, warn, error};
use env_logger;

//
use thiserror::Error;
#[derive(Error, Debug)]
pub enum NDiskError {
    #[error("File '{0}' could not be found.")]
    FileNotFound(std::path::PathBuf),
}

// clap for simpler cli goodness
use clap::Parser;
/// a declarative disk management tool
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path for file to search
    #[arg(short, long)]
    path: std::path::PathBuf,
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    // just some test logging statements
    //info!("this is information");
    warn!("this is a warning");
    //error!("this is an error");

    info!("checking file");
    let _ret_str = parse_file();
}

fn parse_file() -> Result<(),NDiskError> {
    //use NDiskError::*;

    let args = Cli::parse();
    let _content = std::fs::read_to_string(&args.path);
    if Path::new(&args.path).exists() {
        println!("File found!");
    } else {
        error!("File '{0}' could not be found", args.path.display());
    }

    // TODO: breakout error typing further.
    return Err(NDiskError::FileNotFound(args.path));
}
