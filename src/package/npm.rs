use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::anyhow;
use semver::Version;

use crate::package::PanPackage;
use crate::parser::FormatCodec;
use crate::parser::json::JsonString;
use crate::runner::CmdRunner;
use crate::system::FileSystem;

pub struct NpmPackage<F> {
    path: PathBuf,
    doc: JsonString,
    filesystem: PhantomData<F>
}

impl <F: FileSystem> NpmPackage<F> {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let package_str = F::read_string(&path.join("package.json"))?;
        Ok(Self {
            path,
            doc: JsonString::new(&package_str),
            filesystem: PhantomData
        })
    }
}

impl <F: FileSystem> PanPackage for NpmPackage<F> {
    fn extract_version(&self) -> anyhow::Result<Version> {
        self.doc.extract("version")?
            .ok_or_else(|| anyhow!("Could not find version in package.json"))
            .and_then(|v| Ok(Version::parse(v)?))
    }

    fn set_version(&mut self, version: &Version) -> anyhow::Result<()> {
        self.doc.replace("version", &version.to_string())
    }

    fn persist(&self) -> anyhow::Result<()> {
        F::write_string(&self.path.join("package.json"), &self.doc.to_string())?;
        Ok(())
    }

    fn hook_after_rel(&self) -> anyhow::Result<()> {
        let mut runner = if F::is_a_file(&self.path.join("package-lock.json")) {
            CmdRunner::build("npm", &[String::from("i"), String::from("--package-lock-only")], &self.path)?
        } else if F::is_a_file(&self.path.join("yarn.lock")) {
            CmdRunner::build("yarn", &[String::from("--mode"), String::from("update-lockfile")], &self.path)?
        } else if F::is_a_file(&self.path.join("pnpm-lock.yaml")) {
            CmdRunner::build("echo", &[String::from("lockfile update skipped")], &self.path)?
        } else {
            anyhow::bail!("Cannot find any lockfile for package.json")
        };
        runner.run()?;
        Ok(())
    }
}