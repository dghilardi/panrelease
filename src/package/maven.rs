use std::fs;
use std::path::PathBuf;

use anyhow::anyhow;
use regex::Regex;
use semver::Version;

use crate::package::PanPackage;
use crate::parser::FormatCodec;
use crate::parser::xml::xmlstring::XmlString;

pub struct MavenPackage {
    path: PathBuf,
    doc: XmlString,
}

impl MavenPackage {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let package_str = fs::read_to_string(path.join("pom.xml"))?;
        Ok(Self {
            path,
            doc: XmlString::new(&package_str),
        })
    }
}

impl PanPackage for MavenPackage {
    fn extract_version(&self) -> anyhow::Result<Version> {
        let version_str = self.doc.extract("project/version")?
            .ok_or_else(|| anyhow!("Could not find version in pom.xml"))?;

        let placeholder_regex = Regex::new(r#"^\$\{([A-Za-z0-9_.-]+)\}$"#)?;
        let maybe_placeholder = placeholder_regex.captures(version_str)
            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()));

        if let Some(placeholder) = maybe_placeholder {
            let version_prop_str = self.doc.extract(&format!("project/properties/{placeholder}"))?
                .ok_or_else(|| anyhow!("Could not find version property in pom.xml"))?;

            Ok(Version::parse(version_prop_str)?)
        } else {
            Ok(Version::parse(version_str)?)
        }
    }

    fn set_version(&mut self, version: &Version) -> anyhow::Result<()> {
        let version_str = self.doc.extract("project/version")?
            .ok_or_else(|| anyhow!("Could not find version in pom.xml"))?;

        let placeholder_regex = Regex::new(r#"^\$\{([A-Za-z0-9_.-]+)\}$"#)?;
        let maybe_placeholder = placeholder_regex.captures(version_str)
            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()));

        if let Some(placeholder) = maybe_placeholder {
            self.doc.replace(&format!("project/properties/{placeholder}"), &version.to_string())?;
        } else {
            self.doc.replace("project/version", &version.to_string())?;
        }
        Ok(())
    }

    fn persist(&self) -> anyhow::Result<()> {
        fs::write(self.path.join("pom.xml"), self.doc.to_string())?;
        Ok(())
    }

    fn hook_after_rel(&self) -> anyhow::Result<()> {
        Ok(())
    }
}