use std::process;

use clap::Parser;
use update_informer::Check;
use update_informer::registry::Crates;

use crate::args::{Commands, PanReleaseArgs};
use crate::project::core::PanProject;

mod project;
mod args;
mod package;
mod runner;
mod parser;
mod utils;

fn main() {
    env_logger::init();
    check_version();

    let args: PanReleaseArgs = PanReleaseArgs::parse();

    let Ok(cwd) = args.path.map(Ok).unwrap_or_else(std::env::current_dir) else {
        eprintln!("Error loading current directory");
        process::exit(exitcode::IOERR);
    };

    let project = match PanProject::load(cwd.as_path()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error loading project - {e}");
            process::exit(exitcode::IOERR);
        }
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
