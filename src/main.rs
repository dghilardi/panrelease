mod project;
mod args;
mod package;
mod runner;
mod parser;
mod utils;

use clap::Parser;
use update_informer::registry::Crates;
use update_informer::{Check, UpdateInformer};
use crate::args::{Commands, PanReleaseArgs};
use crate::project::core::PanProject;

fn main() {
    env_logger::init();
    check_version();

    let args: PanReleaseArgs = PanReleaseArgs::parse();

    let cwd = std::env::current_dir()
        .expect("Error loading current directory");

    let project = PanProject::load(cwd.as_path())
        .expect("Error loading project");

    match args.subcommand {
        Commands::Release(rel_args) => project.release(rel_args)
            .expect("Error releasing project")
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
