use std::ffi::OsString;
use anyhow::{bail, Context};
use clap::error::ErrorKind;
use clap::Parser;
use futures::executor::block_on;
use crate::args::{Commands, PanReleaseArgs};
use crate::conf::loader::ConfigLoader;
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

    let project = ConfigLoader::parse_config::<S>(opts.path)
        .context("Error parsing configuration file")?;

    match opts.subcommand {
        Commands::Release(rel_args) => {
            if let Err(err) = project.release(rel_args) {
                bail!("Error releasing project - {err}");
            }
        }
    }
    Ok(())
}