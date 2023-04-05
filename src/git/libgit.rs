use std::path::Path;
use git2::{Repository, RepositoryOpenFlags, StatusOptions};
use crate::system::FileSystem;

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    pub fn open<F: FileSystem>(path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            repo: Repository::open_ext(path, RepositoryOpenFlags::empty(), [path])?
        })
    }

    pub fn path(&self) -> &Path {
        self.repo.path()
    }

    pub fn is_staging_clean(&self) -> anyhow::Result<bool> {
        let mut opts = StatusOptions::new();
        opts
            .include_unmodified(false)
            .include_untracked(false)
            .include_ignored(false);

        Ok(self.repo.statuses(Some(&mut opts))?.is_empty())
    }

    pub fn update_and_commit(&self, version: semver::Version) -> anyhow::Result<()> {
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