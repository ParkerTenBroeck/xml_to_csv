use std::{borrow::Cow, str::FromStr};

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug)]
pub enum PathType<'l> {
    #[serde(borrow = "'l")]
    #[serde(rename = "path_text")]
    PathText(Path<'l>),
    #[serde(borrow = "'l")]
    #[serde(rename = "path_len")]
    PathLen(Path<'l>),
    #[serde(borrow = "'l")]
    #[serde(rename = "path_attr")]
    PathAttr(Path<'l>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct Path<'l> {
    #[serde(serialize_with = "ser")]
    #[serde(deserialize_with = "de")]
    #[serde(borrow = "'l")]
    pub parts: Vec<PathPart<'l>>,
}

impl<'l> Path<'l> {
    fn parts_to_string(parts: &[PathPart<'_>]) -> String {
        let mut string = String::new();

        for (index, part) in parts.iter().enumerate() {
            match part {
                PathPart::Element(str) => {
                    string.push_str(str);
                }
                PathPart::Index(index) => {
                    string.push_str(&format!("{}", index));
                }
            }

            if parts.len() - 1 != index {
                string.push('.');
            }
        }

        string
    }
}

fn ser<S>(parts: &[PathPart<'_>], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&Path::parts_to_string(parts))
}

fn de<'l, D>(d: D) -> Result<Vec<PathPart<'l>>, D::Error>
where
    D: Deserializer<'l>,
{
    struct StrVisitor;
    impl<'de> Visitor<'de> for StrVisitor {
        type Value = Vec<PathPart<'de>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("A path 'example.foo.1.bar.5'")
        }

        fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Path::try_from(v)
                .map_err(|e| serde::de::Error::custom(e))
                .map(|v| v.parts)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_string(v.to_owned())
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Path::try_from(v)
                .map_err(|e| serde::de::Error::custom(e))
                .map(|v| v.parts)
        }
    }
    d.deserialize_str(StrVisitor)
}

#[derive(Debug)]
pub enum PathParseError {
    EmptyPart,
}

impl std::fmt::Display for PathParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'l> TryFrom<&'l str> for Path<'l> {
    type Error = PathParseError;

    fn try_from(s: &'l str) -> Result<Self, Self::Error> {
        let mut parts = Vec::new();

        for part in s.split('.') {
            let part = usize::from_str(part)
                .map(PathPart::Index)
                .unwrap_or(PathPart::Element(Cow::Borrowed(part)));
            parts.push(part);
        }

        Ok(Self { parts })
    }
}

impl TryFrom<String> for Path<'_> {
    type Error = PathParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let mut parts = Vec::new();

        for part in s.split('.') {
            let part = usize::from_str(part)
                .map(PathPart::Index)
                .unwrap_or(PathPart::Element(Cow::Owned(part.to_owned())));
            parts.push(part);
        }

        Ok(Self { parts })
    }
}

impl<'l> ToString for Path<'l> {
    fn to_string(&self) -> String {
        Path::parts_to_string(&self.parts)
    }
}

#[derive(Debug)]
pub enum PathPart<'l> {
    Element(Cow<'l, str>),
    Index(usize),
}
