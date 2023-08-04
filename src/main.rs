use std::{borrow::Cow, error::Error, path::PathBuf};

use clap::Parser;
use config::Config;

pub mod config;
pub mod xml_path;

/// XML to CSV converter
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// path to file or folder containing XML files to extract from
    #[arg(value_parser = verify_path_parser)]
    xml_folder: PathBuf,

    /// Path to json config. if blank internal default will be used
    #[arg(short, long, value_name = "CONFIG", value_parser = verify_path_parser)]
    config: Option<PathBuf>,

    /// Path of csv file to be made
    #[arg(short, long, value_name = "OUTPUT", default_value = get_default_save_path().into_os_string())]
    save: PathBuf,

    /// Log xml filepaths to stdout when parsing
    #[arg(short, long)]
    log: bool,

    /// skip over files that don't end with a .xml file extension
    #[arg(short, long)]
    filter: bool,
}

fn verify_path_parser(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.try_exists().unwrap_or(false) {
        Ok(path)
    } else {
        Err(format!("Path: '{s}' doesn't exist"))
    }
}

fn get_default_save_path() -> PathBuf {
    PathBuf::from("output.csv")
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    run(args)
}

fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let config = if let Some(path) = &args.config {
        Cow::Owned(std::fs::read_to_string(path).map_err(|e| {
            format!(
                "Failed to load file '{}' reason: {e}",
                path.to_str().unwrap_or("<Blank>")
            )
        })?)
    } else {
        Cow::Borrowed(include_str!("./default.json"))
    };
    let config: Config<'_> = serde_json::from_str(config.as_ref()).map_err(|e| {
        format!(
            "Failed to parse config file '{}': {e}",
            args.config
                .as_ref()
                .and_then(|v| v.as_os_str().to_str())
                .unwrap_or("<INTERNAL CONFIG>")
        )
    })?;

    let csv_file = std::fs::File::create(&args.save).map_err(|e| {
        format!(
            "Failed to create csv file: '{}': {e}",
            args.save.to_string_lossy()
        )
    })?;
    let mut csv_writter = csv::Writer::from_writer(csv_file);

    for column in &config.csv_columns {
        csv_writter
            .write_field(column.title.as_ref())
            .map_err(|e| format!("Failed to write CSV field: {e}"))?;
    }
    csv_writter
        .write_record(None::<&[u8]>)
        .map_err(|e| format!("Failed to write CSV record: {e}"))?;

    let dir = std::fs::read_dir(&args.xml_folder).map_err(|e|{
        format!("Failed to read xml directory '{}': {}",args.xml_folder.to_string_lossy(), e)
    })?;

    for item in dir.into_iter().flatten() {
        if args.filter {
            if let Some(ex) = item.path().extension() {
                if ex != "xml" {
                    if args.log {
                        println!("skipping: {:?}", item.path());
                    }
                    continue;
                }
            }
        }
        if args.log {
            println!("parsing: {:?}", item.path());
        }

        let xml_file = std::fs::File::open(item.path()).map_err(|e| {
            format!(
                "Failed to open xml file '{}': {e}",
                item.path().to_string_lossy()
            )
        })?;

        let xml = xmltree::Element::parse(xml_file).map_err(|e| {
            format!(
                "Failed to parse xml file '{}': {e}",
                item.path().to_string_lossy()
            )
        })?;

        for column in &config.csv_columns {
            let value = match &column.column_type {
                config::ColumnType::ExtractXmlPath { path, default } => {
                    let res = extract_from_xml(&xml, path);
                    if let Some(default) = default {
                        res.map(Cow::Owned)
                            .unwrap_or(Cow::Borrowed(default.as_ref()))
                    } else {
                        Cow::Owned(res.map_err(|e| {
                            format!(
                                "Failed to extract column from xml file '{}': {e}",
                                item.path().to_string_lossy()
                            )
                        })?)
                    }
                }
                config::ColumnType::Text { text } => Cow::Borrowed(text.as_ref()),
                config::ColumnType::Intrinsic { intrinsic } => match intrinsic {
                    config::Intrinsic::FilePath => {
                        Cow::Owned(item.path().into_os_string().to_string_lossy().into_owned())
                    }
                },
            };

            csv_writter
                .write_field(value.as_ref())
                .map_err(|e| format!("Failed to write CSV field: {e}"))?;
        }

        csv_writter
            .write_record(None::<&[u8]>)
            .map_err(|e| format!("Failed to write CSV record: {e}"))?;
    }

    Ok(())
}

fn extract_from_xml(
    xml: &xmltree::Element,
    xml_path: &xml_path::PathType,
) -> Result<String, Box<dyn Error>> {
    let (follow_last, path) = match xml_path {
        xml_path::PathType::PathText(path) => (true, path),
        xml_path::PathType::PathLen(path) => (true, path),
        xml_path::PathType::PathAttr(path) => (false, path),
    };

    let mut element = xml;

    let parts = if follow_last {
        &path.parts
    } else {
        &path.parts[..path.parts.len() - 1]
    };

    for part in parts {
        match part {
            xml_path::PathPart::Element(node_name) => {
                element = element
                    .get_child(node_name.as_ref())
                    .ok_or_else(|| format!("Cannot find node: {} from xml path: {}", node_name.as_ref(), path.to_string()))?;
            }
            xml_path::PathPart::Index(index) => {
                element = element
                    .children
                    .get(*index)
                    .ok_or_else(|| format!("Cannog get child node: {index}"))
                    .map(|v| {
                        v.as_element().ok_or_else(|| {
                            format!("The item at the index: {index} is not an element")
                        })
                    })??;
            }
        }
    }

    match &xml_path {
        xml_path::PathType::PathText(path) => Ok(element
            .get_text()
            .ok_or_else(|| format!("Failed to get text from {}", path.to_string()))?
            .into_owned()),
        xml_path::PathType::PathLen(_) => Ok(element.children.len().to_string()),
        xml_path::PathType::PathAttr(path) => {
            let last = path.parts.last().ok_or("Paths need at least one part")?;
            match last {
                xml_path::PathPart::Element(name) => {
                    let name = name.as_ref();

                    Ok(element
                        .attributes
                        .get(name)
                        .ok_or_else(|| {
                            format!("Failed to get attribute from path: {}", path.to_string())
                        })?
                        .to_owned())
                }
                xml_path::PathPart::Index(_) => Err("Cannot use an index for attributes")?,
            }
        }
    }
}
