use std::future::Future;

use schemars::{JsonSchema, Schema, generate::SchemaSettings};
use serde::Serialize;
use strum::IntoEnumIterator;

use super::{CliSuccess, response::ErrorResponse};
use crate::domain::models::entity_ingestion::DeclarationIngestionStage;

fn schema<T: JsonSchema>() -> Schema {
    SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>()
}

trait ResponseSchema {
    fn schema() -> Schema;
}

impl<T: Serialize + JsonSchema, E> ResponseSchema for Result<CliSuccess<T>, E> {
    fn schema() -> Schema {
        schema::<T>()
    }
}

pub(super) trait HandlerSchema<Args> {
    fn response_schema(self) -> Schema;
}

macro_rules! handler_schema {
    ($($arg:ident),*) => {
        impl<F, Fut, $($arg,)*> HandlerSchema<($($arg,)*)> for F
        where
            F: FnOnce($($arg),*) -> Fut,
            Fut: Future,
            Fut::Output: ResponseSchema,
        {
            fn response_schema(self) -> Schema {
                <Fut::Output as ResponseSchema>::schema()
            }
        }
    };
}

handler_schema!();
handler_schema!(A);
handler_schema!(A, B);
handler_schema!(A, B, C);

fn section(label: &str, schema: &Schema) -> String {
    format!(
        "{label}\n{}\n",
        serde_json::to_string_pretty(schema).expect("JSON schema serializes")
    )
}

fn failure() -> String {
    format!(
        "Success: one JSON document on stdout, exit 0.\nCommand failure: one JSON document on stderr, exit 1.\nClap argument errors and help use Clap's text output.\nSchemas use JSON Schema draft 2020-12.\n\n{}",
        section("Failure JSON schema:", &schema::<ErrorResponse>())
    )
}

pub(super) fn single<Args>(handler: impl HandlerSchema<Args>) -> String {
    format!(
        "{}\n{}",
        section("Success JSON schema:", &handler.response_schema()),
        failure()
    )
}

pub(super) fn declarations() -> String {
    let mut help = String::new();
    for stage in DeclarationIngestionStage::iter() {
        let mut schema = match stage {
            DeclarationIngestionStage::Fetch => super::fetch_declarations.response_schema(),
            DeclarationIngestionStage::Clean => super::clean_declarations.response_schema(),
            DeclarationIngestionStage::Resolve => super::resolve_declarations.response_schema(),
            DeclarationIngestionStage::Load => super::load_declarations.response_schema(),
        };
        if stage == DeclarationIngestionStage::Fetch {
            schema
                .as_object_mut()
                .expect("response schema is an object")["properties"]["stage"]["const"] =
                serde_json::to_value(&stage).expect("stage serializes");
        }
        let label = serde_json::to_value(&stage).expect("stage serializes");
        help.push_str(&section(
            &format!(
                "Success JSON schema ({}):",
                label.as_str().expect("stage is a string")
            ),
            &schema,
        ));
        help.push('\n');
    }
    help.push_str(&failure());
    help
}
