use std::path::Path;
use anyhow::anyhow;
use git2::{BlameOptions, DescribeFormatOptions, DescribeOptions, Repository, RepositoryOpenFlags, Sort, StatusOptions};
use crate::changelog::{BlameLine, CommitInfo};
use crate::project::config::GitConfig;
use crate::system::FileSystem;

pub struct GitRepo {
    config: GitConfig,
    repo: Repository,
}

impl GitRepo {
    pub fn open<F: FileSystem>(config: GitConfig, path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            repo: Repository::open_ext(path, RepositoryOpenFlags::empty(), [path])?
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

    pub fn current_branch(&self) -> anyhow::Result<Option<String>> {
        if self.repo.head_detached()? {
            return Ok(None);
        }
        let head = self.repo.head()?;
        Ok(head.shorthand().map(String::from))
    }

    pub fn merge_base(&self, mainline: &str) -> anyhow::Result<String> {
        let head_oid = self.repo.head()?.peel_to_commit()?.id();
        let main_ref = self.repo.revparse_single(mainline)?;
        let main_oid = main_ref.peel_to_commit()?.id();
        let base = self.repo.merge_base(head_oid, main_oid)?;
        Ok(base.to_string())
    }

    pub fn latest_version_tag(&self, commit: &str) -> anyhow::Result<Option<String>> {
        let oid = git2::Oid::from_str(commit)?;
        let commit_obj = self.repo.find_object(oid, None)?;

        let mut describe_opts = DescribeOptions::new();
        describe_opts.describe_tags();

        let describe_result = commit_obj.describe(&describe_opts);
        match describe_result {
            Ok(desc) => {
                let mut fmt_opts = DescribeFormatOptions::new();
                fmt_opts.abbreviated_size(0);
                let tag = desc.format(Some(&fmt_opts))?;
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
        let mut opts = StatusOptions::new();
        opts
            .include_unmodified(false)
            .include_untracked(false)
            .include_ignored(false);

        Ok(self.repo.statuses(Some(&mut opts))?.is_empty())
    }

    pub fn commits_since_tag(&self, tag: &str) -> anyhow::Result<Vec<CommitInfo>> {
        let tag_obj = self.repo.revparse_single(tag)?;
        let tag_oid = tag_obj.peel_to_commit()?.id();
        let head_oid = self.repo.head()?.peel_to_commit()?.id();

        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(Sort::TIME)?;
        revwalk.push(head_oid)?;
        revwalk.hide(tag_oid)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.message().unwrap_or("").to_string();
            let timestamp = commit.time().seconds();
            commits.push(CommitInfo {
                hash: oid.to_string(),
                message,
                timestamp,
            });
        }

        Ok(commits)
    }

    pub fn blame_file(&self, path: &Path) -> anyhow::Result<Vec<BlameLine>> {
        let workdir = self.repo.workdir()
            .ok_or_else(|| anyhow!("Repository has no working directory"))?;
        let relative = path.strip_prefix(workdir).unwrap_or(path);

        let mut opts = BlameOptions::new();
        let blame = self.repo.blame_file(relative, Some(&mut opts))?;

        let content = std::fs::read_to_string(path)?;
        let mut result = Vec::new();

        for (i, line) in content.lines().enumerate() {
            if let Some(hunk) = blame.get_line(i + 1) {
                result.push(BlameLine {
                    commit_hash: hunk.final_commit_id().to_string(),
                    line_content: line.to_string(),
                });
            }
        }

        Ok(result)
    }

    pub fn update_and_commit(&self, version: semver::Version) -> anyhow::Result<()> {
        if self.config.force_sign {
            anyhow::bail!("Commit/tag sign is not supported in lib mode...");
        }

        let mut index = self.repo.index()?;
        index.update_all(["*"].iter(), Some(&mut (|name, _content| {
            log::debug!("Adding {:?}", name);
            0
        })))?;
        index.write()?;

        let signature = self.repo.signature()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;

        let descr = version.to_string();
        let commit_oid = self.repo.commit(Some("HEAD"), &signature, &signature, &descr, &tree, &[&parent_commit])?;

        let commit_obj = self.repo.find_object(commit_oid, None)?;
        self.repo.tag_lightweight(&descr, &commit_obj, false)?;

        Ok(())
    }
}