use std::ffi::OsString;
use anyhow::Context;
use clap::Parser;
use crate::args::{Commands, PanReleaseArgs};
use crate::conf::loader::ConfigLoader;
use crate::init;
use crate::system::FileSystem;

pub fn run<I, T, S>(args: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
        S: FileSystem + 'static,
{
    let opts = match PanReleaseArgs::try_parse_from(args) {
        Ok(opts) => opts,
        Err(err) => err.exit(),
    };

    match opts.subcommand {
        Commands::Init(init_args) => {
            init::run::<S>(opts.path, init_args).context("Error initializing project")?;
        }
        Commands::Release(rel_args) => {
            let project = ConfigLoader::parse_config::<S>(opts.path)
                .context("Error parsing configuration file")?;
            project.release(rel_args).context("Error releasing project")?;
        }
        Commands::Show(show_args) => {
            let project = ConfigLoader::parse_config::<S>(opts.path)
                .context("Error parsing configuration file")?;
            let version = project.current_version()?;
            println!("{}", show_args.render_version(&version));
        }
    }
    Ok(())
}
