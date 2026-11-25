use std::ffi::OsString;
use std::path::{Component, PathBuf};
use anyhow::Context;
use clap::Parser;
use crate::args::{Commands, PanReleaseArgs};
use crate::conf::loader::ConfigLoader;
use crate::init;
use crate::skill;
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
    let path = opts.path.map(absolute_path::<S>).transpose()?;

    match opts.subcommand {
        Commands::Init(init_args) => {
            init::run::<S>(path, init_args).context("Error initializing project")?;
        }
        Commands::Release(rel_args) => {
            let project = ConfigLoader::parse_config::<S>(path)
                .context("Error parsing configuration file")?;
            project.release(rel_args).context("Error releasing project")?;
        }
        Commands::Show(show_args) => {
            let project = ConfigLoader::parse_config::<S>(path)
                .context("Error parsing configuration file")?;
            let version = project.current_version()?;
            println!("{}", show_args.render_version(&version));
        }
        Commands::Skill(skill_args) => {
            skill::run::<S>(path, skill_args).context("Error handling agent skill")?;
        }
    }
    Ok(())
}

/// Resolve `--path` against the current directory and drop `.`/`..` segments,
/// so the Git root lookup walks real parent directories.
fn absolute_path<S: FileSystem>(path: PathBuf) -> anyhow::Result<PathBuf> {
    let path = if path.is_absolute() { path } else { S::current_dir()?.join(path) };
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::NativeSystem;

    #[test]
    fn absolute_path_keeps_absolute_paths() {
        let path = std::env::temp_dir().join("panrelease");
        assert_eq!(absolute_path::<NativeSystem>(path.clone()).unwrap(), path);
    }

    #[test]
    fn absolute_path_resolves_relative_paths_from_current_dir() {
        let resolved = absolute_path::<NativeSystem>(PathBuf::from("./../other/sub")).unwrap();
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(resolved, cwd.parent().unwrap().join("other/sub"));
    }
}
