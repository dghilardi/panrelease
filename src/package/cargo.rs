use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use semver::Version;

use crate::package::PanPackage;
use crate::runner::CmdRunner;

pub struct CargoPackage {
    path: PathBuf,
    doc: toml_edit::Document,
}

impl CargoPackage {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let cargo_str = fs::read_to_string(path.join("Cargo.toml"))?;
        Ok(Self {
            path,
            doc: cargo_str.parse::<toml_edit::Document>()?,
        })
    }
}

impl PanPackage for CargoPackage {
    fn extract_version(&self) -> anyhow::Result<Version> {
        let Some(ver) = self.doc["package"]["version"].as_str() else {
            anyhow::bail!("cannot find version in Cargo.toml")
        };
        Ok(Version::from_str(ver)?)
    }

    fn set_version(&mut self, version: &Version) -> anyhow::Result<()> {
        self.doc["package"]["version"] = toml_edit::value(version.to_string());
        Ok(())
    }

    fn persist(&self) -> anyhow::Result<()> {
        fs::write(self.path.join("Cargo.toml"), self.doc.to_string())?;
        Ok(())
    }

    fn hook_after_rel(&self) -> anyhow::Result<()> {
        let mut runner = CmdRunner::build("cargo", &[String::from("generate-lockfile")], &self.path)?;
        runner.run()?;
        Ok(())
    }
}