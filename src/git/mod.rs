mod cligit;

#[cfg(not(feature = "git2"))]
pub use cligit::GitRepo;

#[cfg(feature = "git2")]
mod libgit;

#[cfg(feature = "git2")]
pub use libgit::GitRepo;

fn stable_release_tag(tag: &str, template: &str) -> bool {
    crate::project::strict::version_from_tag(tag, template)
        .is_some_and(|version| version.pre.is_empty() && version.build.is_empty())
}
