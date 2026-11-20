use std::collections::HashSet;
use std::path::Path;
use anyhow::anyhow;
use git2::{BlameOptions, DescribeFormatOptions, DescribeOptions, Repository, RepositoryOpenFlags, Sort, StatusOptions};
use crate::changelog::{BlameLine, CommitInfo};
use crate::project::config::GitConfig;
use crate::system::FileSystem;

fn tree_entry<'a>(tree: &'a git2::Tree<'_>, path: &str) -> anyhow::Result<Option<git2::TreeEntry<'a>>> {
    match tree.get_path(Path::new(path)) {
        Ok(entry) => Ok(Some(entry)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn regular_blob(entry: &git2::TreeEntry<'_>) -> bool {
    matches!(entry.filemode(), 0o100644 | 0o100755)
}

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
                current = current.parent().ok_or_else(|| anyhow!(
                    "Could not find a git repository: no .git directory found \
                     searching up from {:?}",
                    path
                ))?;
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

    pub fn dirty_files(&self) -> anyhow::Result<Vec<String>> {
        let mut opts = StatusOptions::new();
        opts
            .include_unmodified(false)
            .include_untracked(false)
            .include_ignored(false);

        Ok(self.repo.statuses(Some(&mut opts))?
            .iter()
            .filter_map(|s| s.path().map(String::from))
            .collect())
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

    pub(crate) fn head_revision(&self) -> anyhow::Result<String> {
        Ok(self.repo.head()?.peel_to_commit()?.id().to_string())
    }

    pub(crate) fn is_shallow(&self) -> anyhow::Result<bool> {
        Ok(self.repo.is_shallow())
    }

    pub(crate) fn stable_release_base(&self, revision: &str) -> anyhow::Result<Option<String>> {
        let mut tagged_commits = HashSet::new();
        for tag in self.repo.tag_names(None)?.iter().flatten() {
            // The libgit backend writes bare version tags.
            if super::stable_release_tag(tag, "{{version}}") || super::stable_release_tag(tag, &self.config.tag_template) {
                let object = self.repo.revparse_single(&format!("refs/tags/{tag}"))?;
                if let Ok(commit) = object.peel_to_commit() {
                    tagged_commits.insert(commit.id());
                }
            }
        }
        let mut revisions = self.first_parent_walk(revision)?;
        for oid in &mut revisions {
            let oid = oid?;
            if tagged_commits.contains(&oid) {
                return Ok(Some(oid.to_string()));
            }
        }
        Ok(None)
    }

    fn first_parent_walk(&self, revision: &str) -> anyhow::Result<git2::Revwalk<'_>> {
        let mut revisions = self.repo.revwalk()?;
        revisions.simplify_first_parent()?;
        revisions.push(git2::Oid::from_str(revision)?)?;
        Ok(revisions)
    }

    pub(crate) fn tracked_blobs(&self, revision: &str, path: &str) -> anyhow::Result<Vec<(String, String)>> {
        let tree = self.repo.find_commit(git2::Oid::from_str(revision)?)?.tree()?;
        let Some(entry) = tree_entry(&tree, path)? else {
            return Ok(Vec::new());
        };
        if regular_blob(&entry) {
            return Ok(vec![(path.to_owned(), entry.id().to_string())]);
        }
        if entry.kind() != Some(git2::ObjectType::Tree) {
            return Ok(Vec::new());
        }
        let mut blobs = Vec::new();
        let directory = self.repo.find_tree(entry.id())?;
        directory.walk(git2::TreeWalkMode::PreOrder, |prefix, entry| {
            if let Some(name) = entry.name().filter(|_| regular_blob(entry)) {
                blobs.push((format!("{path}/{prefix}{name}"), entry.id().to_string()));
            }
            git2::TreeWalkResult::Ok
        })?;
        Ok(blobs)
    }

    pub(crate) fn read_blob(&self, oid: &str) -> anyhow::Result<String> {
        let blob = self.repo.find_blob(git2::Oid::from_str(oid)?)?;
        Ok(std::str::from_utf8(blob.content())?.to_owned())
    }

    pub(crate) fn path_revisions(&self, revision: &str, base: Option<&str>, path: &str) -> anyhow::Result<Vec<String>> {
        let mut changed = Vec::new();
        let mut last_blob = None;
        for oid in self.first_parent_walk(revision)? {
            let oid = oid?;
            let revision = oid.to_string();
            if base == Some(revision.as_str()) {
                break;
            }
            let tree = self.repo.find_commit(oid)?.tree()?;
            let blob = tree_entry(&tree, path)?.filter(regular_blob).map(|entry| entry.id());
            if last_blob != Some(blob) {
                changed.push(revision);
                last_blob = Some(blob);
            }
        }
        Ok(changed)
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
