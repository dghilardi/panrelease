mod cligit;

#[cfg(not(feature = "git2"))]
pub use cligit::GitRepo;

#[cfg(feature = "git2")]
mod libgit;

#[cfg(feature = "git2")]
pub use libgit::GitRepo;