mod project;
mod args;
mod package;
mod runner;

use clap::Parser;
use crate::args::{Commands, PanReleaseArgs};
use crate::project::core::PanProject;

fn main() {
    env_logger::init();

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
