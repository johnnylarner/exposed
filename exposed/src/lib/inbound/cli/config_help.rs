use std::fmt::Write;

use schemars::{JsonSchema, generate::SchemaSettings};

use super::config::{
    DeclarationFetcherConfig, DeclarationResolverConfig, FetcherConfig, LoaderConfig,
};
use crate::domain::models::entity_ingestion::{DeclarationIngestionStage, MemberIngestionStage};

fn introduction() -> String {
    "Path to a YAML configuration file.\nRequired to run the command; omitted with --schema.\nRelative data_dir paths resolve against the current working directory.\n".to_owned()
}

fn fields<T: JsonSchema>(help: &mut String, label: &str, database: bool) {
    let schema = SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator()
        .into_root_schema_for::<T>();
    help.push_str("\nExample YAML:\n");
    let value = schema.as_value();
    for (name, field) in value["properties"]
        .as_object()
        .expect("configuration fields")
    {
        let example =
            field
                .get("default")
                .cloned()
                .unwrap_or_else(|| match field["type"].as_str() {
                    Some("integer") => serde_json::json!(100),
                    Some("string") => serde_json::json!("./data"),
                    _ => unreachable!("configuration field is a string or integer"),
                });
        writeln!(help, "  {name}: {example}").expect("writing to a String succeeds");
    }
    write!(help, "\n{label}:\n").expect("writing to a String succeeds");
    let schema = schema.as_value();
    let required = schema["required"].as_array();
    for (name, field) in schema["properties"]
        .as_object()
        .expect("configuration schema has fields")
    {
        let presence = if required.is_some_and(|names| names.iter().any(|value| value == name)) {
            "required"
        } else {
            "optional"
        };
        write!(
            help,
            "  {name} ({}, {presence}",
            field["type"]
                .as_str()
                .expect("configuration field has a primitive type")
        )
        .expect("writing to a String succeeds");
        if let Some(default) = field.get("default") {
            write!(help, ", default {default}").expect("writing to a String succeeds");
        }
        writeln!(
            help,
            "):\n    {}",
            field["description"].as_str().unwrap_or_default()
        )
        .expect("writing to a String succeeds");
    }
    if database {
        help.push_str("  Environment: DATABASE_URL is required separately from the YAML file.\n");
    }
}

pub(super) fn store() -> String {
    let mut help = introduction();
    fields::<LoaderConfig>(&mut help, "Fields", false);
    help
}

pub(super) fn declaration(stage: &DeclarationIngestionStage) -> String {
    let mut help = introduction();
    match stage {
        DeclarationIngestionStage::Fetch => {
            fields::<DeclarationFetcherConfig>(&mut help, "Fields", true);
        }
        DeclarationIngestionStage::Clean => fields::<LoaderConfig>(&mut help, "Fields", false),
        DeclarationIngestionStage::Resolve => {
            fields::<DeclarationResolverConfig>(&mut help, "Fields", false);
        }
        DeclarationIngestionStage::Load => fields::<LoaderConfig>(&mut help, "Fields", true),
    }
    help
}

pub(super) fn member(stage: &MemberIngestionStage) -> String {
    let mut help = introduction();
    match stage {
        MemberIngestionStage::Fetch => fields::<FetcherConfig>(&mut help, "Fields", false),
        MemberIngestionStage::Load => fields::<LoaderConfig>(&mut help, "Fields", true),
    }
    help
}
