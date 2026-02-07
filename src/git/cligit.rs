use crate::project::config::GitConfig;
use crate::runner::CmdRunner;
use crate::system::FileSystem;
use anyhow::anyhow;
use std::path::{Path, PathBuf};

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
                current = current.parent().ok_or(anyhow!("Could not find repo dir"))?;
            }
        }
    }

    pub fn current_branch(&self) -> anyhow::Result<Option<String>> {
        let mut runner = CmdRunner::build(
            "git",
            &[String::from("rev-parse"), String::from("--abbrev-ref"), String::from("HEAD")],
            &self.path,
        )?;
        let out = runner.output().and_then(|b| Ok(String::from_utf8(b)?))?;
        let branch = out.trim().to_string();
        if branch == "HEAD" {
            Ok(None)
        } else {
            Ok(Some(branch))
        }
    }

    pub fn merge_base(&self, mainline: &str) -> anyhow::Result<String> {
        let mut runner = CmdRunner::build(
            "git",
            &[String::from("merge-base"), String::from("HEAD"), String::from(mainline)],
            &self.path,
        )?;
        let out = runner.output().and_then(|b| Ok(String::from_utf8(b)?))?;
        Ok(out.trim().to_string())
    }

    pub fn latest_version_tag(&self, commit: &str) -> anyhow::Result<Option<String>> {
        let mut runner = CmdRunner::build(
            "git",
            &[String::from("describe"), String::from("--tags"), String::from("--abbrev=0"), String::from(commit)],
            &self.path,
        )?;
        match runner.output() {
            Ok(b) => {
                let tag = String::from_utf8(b)?.trim().to_string();
                if tag.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(tag))
                }
            }
            Err(_) => Ok(None),
        }
    }

    pub fn is_staging_clean(&self) -> anyhow::Result<bool> {
        let mut runner = CmdRunner::build(
            "git",
            &[String::from("status"), String::from("--porcelain=v1")],
            &self.path,
        )?;
        let out = runner.output().and_then(|b| Ok(String::from_utf8(b)?))?;
        let pending = out
            .split('\n')
            .map(|head| head.trim())
            .filter(|head| !head.is_empty() && !(*head).starts_with("??"))
            .collect::<Vec<_>>();

        Ok(pending.is_empty())
    }

    pub fn update_and_commit(&self, version: semver::Version) -> anyhow::Result<()> {
        CmdRunner::build(
            "git",
            &[String::from("add"), String::from("-u")],
            &self.path,
        )?
        .run()?;

        let descr = version.to_string();
        let commit_args = if self.config.force_sign {
            vec! [String::from("commit"), String::from("-S"), String::from("-m"), descr]
        } else {
            vec! [String::from("commit"), String::from("-m"), descr]
        };

        CmdRunner::build(
            "git",
            &commit_args,
            &self.path,
        )?
        .run()?;

        let tag_descr = self
            .config
            .tag_template
            .replace("{{version}}", &version.to_string());

        if self.config.force_sign {
            CmdRunner::build(
                "git",
                &[
                    String::from("tag"),
                    String::from("-a"),
                    tag_descr.clone(),
                    String::from("-m"),
                    tag_descr,
                    String::from("-s"),
                ],
                &self.path,
            )?
            .run()?;
        } else {
            CmdRunner::build("git", &[String::from("tag"), tag_descr], &self.path)?.run()?;
        }

        Ok(())
    }
}
