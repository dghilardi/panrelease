use std::path::{Path, PathBuf};
use anyhow::anyhow;
use crate::project::config::GitConfig;
use crate::runner::CmdRunner;
use crate::system::FileSystem;
use crate::wasm_utils::log;

pub struct GitRepo {
    config: GitConfig,
    path: PathBuf,
}

impl GitRepo {
    pub fn open<F: FileSystem>(config: GitConfig, path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            path: Self::find_git_root::<F>(path)?.to_path_buf(),
        })
    }

    pub fn find_git_root<F: FileSystem>(path: &Path) -> anyhow::Result<&Path> {
        let mut current = path;
        loop {
            if F::is_a_dir(&current.join(".git")) {
                break Ok(current);
            } else {
                current = current.parent()
                    .ok_or(anyhow!("Could not find repo dir"))?;
            }
        }
    }

    pub fn is_staging_clean(&self) -> anyhow::Result<bool> {
        let mut runner = CmdRunner::build("git", &[String::from("status"), String::from("--porcelain=v1")], &self.path)?;
        let out = runner.output()
            .and_then(|b| Ok(String::from_utf8(b)?))?;
        let pending = out.split('\n')
            .map(|head| head.trim())
            .filter(|head| !head.is_empty() && !(*head).starts_with("??"))
            .collect::<Vec<_>>();

        log(&format!("Pending: {pending:?}"));
        Ok(pending.is_empty())
    }

    pub fn update_and_commit(&self, version: semver::Version) -> anyhow::Result<()> {
        CmdRunner::build("git", &[String::from("add"), String::from("-u")], &self.path)?
            .run()?;

        let descr = version.to_string();

        CmdRunner::build("git", &[String::from("commit"), String::from("-m"), descr], &self.path)?
            .run()?;

        let tag_descr = self.config.tag_template.replace("{{version}}", &version.to_string());
        CmdRunner::build("git", &[String::from("tag"), tag_descr], &self.path)?
            .run()?;

        Ok(())
    }
}