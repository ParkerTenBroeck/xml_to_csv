use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::xml_path::PathType;

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct Config<'l> {
    #[serde(borrow = "'l")]
    pub csv_columns: Vec<CsvColumn<'l>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CsvColumn<'l> {
    #[serde(borrow = "'l")]
    pub title: Cow<'l, str>,
    #[serde(flatten)]
    pub column_type: ColumnType<'l>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ColumnType<'l> {
    ExtractXmlPath {
        #[serde(flatten)]
        #[serde(borrow = "'l")]
        path: PathType<'l>,
        default: Option<Cow<'l, str>>,
    },
    Text {
        text: Cow<'l, str>,
    },
    Intrinsic {
        intrinsic: Intrinsic,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Intrinsic {
    FilePath,
}
