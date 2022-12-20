mod project;
mod args;
mod package;
mod runner;
mod parser;
mod utils;

use std::process;
use clap::Parser;
use update_informer::registry::Crates;
use update_informer::{Check, UpdateInformer};
use crate::args::{Commands, PanReleaseArgs};
use crate::project::core::PanProject;

fn main() {
    env_logger::init();
    check_version();

    let args: PanReleaseArgs = PanReleaseArgs::parse();

    let Ok(cwd) = std::env::current_dir() else {
        eprintln!("Error loading current directory");
        process::exit(exitcode::IOERR);
    };

    let Ok(project) = PanProject::load(cwd.as_path()) else {
        eprintln!("Error loading project");
        process::exit(exitcode::IOERR);
    };

    match args.subcommand {
        Commands::Release(rel_args) => {
            if let Err(err) = project.release(rel_args) {
                eprintln!("Error releasing project - {err}");
                process::exit(exitcode::SOFTWARE);
            }
        }
    }
}

fn check_version() {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(Crates, name, version);
    if let Some(version) = informer.check_version().ok().flatten()  {
        println!("New version is available: {}", version);
    }
}
