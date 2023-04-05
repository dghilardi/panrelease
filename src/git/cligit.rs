use std::path::{Path, PathBuf};
use anyhow::anyhow;
use crate::runner::CmdRunner;
use crate::system::FileSystem;

pub struct GitRepo {
    path: PathBuf,
}

impl GitRepo {
    pub fn open<F: FileSystem>(path: &Path) -> anyhow::Result<Self> {
        let mut current = path;
        loop {
            if F::is_a_dir(&current.join(".git")) {
                break Ok(Self {
                    path: current.to_path_buf(),
                })
            } else {
                current = current.parent()
                    .ok_or(anyhow!("Could not find repo dir"))?;
            }
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn is_staging_clean(&self) -> anyhow::Result<bool> {
        let mut runner = CmdRunner::build("git", &[String::from("status"), String::from("--porcelain=v2")], &self.path)?;
        runner.run()?;
        todo!()
    }

    pub fn update_and_commit(&self, version: semver::Version) -> anyhow::Result<()> {
        todo!()
    }
}