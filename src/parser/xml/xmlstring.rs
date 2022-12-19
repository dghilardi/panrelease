use crate::parser::FormatCodec;
use crate::parser::xml::parsexml::{XMLDocument, XMLNode};
use crate::parser::xml::value::Value;
use crate::utils::get_range;

pub struct XmlString {
    inner: String,
}

impl XmlString {
    pub fn new(input: &str) -> Self {
        Self {
            inner: String::from(input),
        }
    }
}

impl FormatCodec for XmlString {
    fn extract(&self, path: &str) -> anyhow::Result<Option<&str>> {
        let document = XMLDocument::try_from(&self.inner[..])?;

        let path_parts = path.split('/');

        let mut next = &document.content;
        for key in path_parts {
            let Some(XMLNode::Element(_, _, content)) = next.iter().find(|node| has_name(node, key)) else {
                anyhow::bail!("Parsed value is not an object");
            };
            next = content;
        }
        let Some(XMLNode::Text(Value::String(value))) = next.first() else {
            println!("{:?}", next.first());
            anyhow::bail!("Could not find {path}")
        };
        Ok(Some(*value))
    }

    fn replace(&mut self, path: &str, value: &str) -> anyhow::Result<()> {
        let Some(current) = self.extract(path)? else {
            anyhow::bail!("Could not find {path} in given xml")
        };
        let (start, end) = get_range(&self.inner, current);
        self.inner.replace_range(start..end, value);
        Ok(())
    }
}

fn has_name(node: &XMLNode, name: &str) -> bool {
    match node {
        XMLNode::Element(qualname, _, _) => qualname.get_localname().eq(name),
        _ => false
    }
}

impl ToString for XmlString {
    fn to_string(&self) -> String {
        self.inner.clone()
    }
}