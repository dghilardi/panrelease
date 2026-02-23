use anyhow::anyhow;

use crate::parser::FormatCodec;

pub struct TomlString {
    doc: toml_edit::DocumentMut,
}

impl TomlString {
    pub fn new(input: &str) -> anyhow::Result<Self> {
        let doc = input
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| anyhow!("TOML parse error: {e}"))?;
        Ok(Self { doc })
    }
}

impl FormatCodec for TomlString {
    fn extract(&self, path: &str) -> anyhow::Result<Option<&str>> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut item: &toml_edit::Item = self.doc.as_item();
        for (i, part) in parts.iter().enumerate() {
            match item.get(part) {
                Some(next) => item = next,
                None => {
                    let traversed = parts[..=i].join(".");
                    anyhow::bail!(
                        "Key '{}' not found at '{}' while traversing TOML path '{}'",
                        part,
                        traversed,
                        path
                    );
                }
            }
        }
        Ok(item.as_str())
    }

    fn replace(&mut self, path: &str, value: &str) -> anyhow::Result<()> {
        let parts: Vec<&str> = path.split('.').collect();
        let (last, prefix) = parts
            .split_last()
            .ok_or_else(|| anyhow!("Empty TOML path"))?;

        let mut item: &mut toml_edit::Item = self.doc.as_item_mut();
        for part in prefix {
            item = item.get_mut(part).ok_or_else(|| {
                anyhow!(
                    "Key '{}' not found in TOML while traversing path '{}'",
                    part,
                    path
                )
            })?;
        }

        let target = item
            .get_mut(last)
            .ok_or_else(|| anyhow!("Key '{}' not found in TOML at path '{}'", last, path))?;
        *target = toml_edit::value(value);
        Ok(())
    }
}

impl ToString for TomlString {
    fn to_string(&self) -> String {
        self.doc.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extract_and_replace_nested() {
        let input = "[package]\nversion = \"1.0.0\"\n";
        let mut ts = TomlString::new(input).unwrap();
        let ver = ts.extract("package.version").unwrap().unwrap();
        assert_eq!(ver, "1.0.0");

        ts.replace("package.version", "2.0.0").unwrap();
        let ver2 = ts.extract("package.version").unwrap().unwrap();
        assert_eq!(ver2, "2.0.0");

        let output = ts.to_string();
        assert!(output.contains("[package]"));
        assert!(output.contains("version = \"2.0.0\""));
    }

    #[test]
    fn extract_top_level_key() {
        let input = "version = \"3.0.0\"\nname = \"test\"\n";
        let ts = TomlString::new(input).unwrap();
        let ver = ts.extract("version").unwrap().unwrap();
        assert_eq!(ver, "3.0.0");
    }

    #[test]
    fn extract_missing_key_gives_error() {
        let input = "version = \"1.0.0\"\n";
        let ts = TomlString::new(input).unwrap();
        let err = ts.extract("missing").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("missing"),
            "error should mention the missing key: {msg}"
        );
    }
}
