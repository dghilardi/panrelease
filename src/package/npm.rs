use std::fs;
use std::path::PathBuf;
use anyhow::anyhow;
use semver::Version;
use crate::package::PanPackage;
use crate::parser::FormatCodec;
use crate::parser::json::JsonString;
use crate::runner::CmdRunner;

pub struct NpmPackage {
    path: PathBuf,
    doc: JsonString,
}

impl NpmPackage {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let package_str = fs::read_to_string(path.join("package.json"))?;
        Ok(Self {
            path,
            doc: JsonString::new(&package_str),
        })
    }
}

impl PanPackage for NpmPackage {
    fn extract_version(&self) -> anyhow::Result<Version> {
        self.doc.extract("version")?
            .ok_or(anyhow!("Could not find version in package.json"))
            .and_then(|v| Ok(Version::parse(v)?))
    }

    fn set_version(&mut self, version: &Version) -> anyhow::Result<()> {
        self.doc.replace("version", &version.to_string())
    }

    fn persist(&self) -> anyhow::Result<()> {
        fs::write(self.path.join("package.json"), self.doc.to_string())?;
        Ok(())
    }

    fn hook_after_rel(&self) -> anyhow::Result<()> {
        let mut runner = if self.path.join("package-lock.json").is_file() {
            CmdRunner::build("npm", &[String::from("i"), String::from("--package-lock-only")])?
        } else if self.path.join("yarn.lock").is_file() {
            CmdRunner::build("yarn", &[String::from("--mode"), String::from("update-lockfile")])?
        } else {
            anyhow::bail!("Cannot find any lockfile for package.json")
        };
        runner.run()?;
        Ok(())
    }
}