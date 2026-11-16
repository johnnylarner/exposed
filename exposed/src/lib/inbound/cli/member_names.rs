use super::{
    CliSuccess, IntoResponse, Response,
    config::{DataConfig, LoaderConfig},
    response_help,
    stages::{StageArgs, StageInput},
};
use crate::{
    domain::{
        models::{entity_ingestion::IngestionKey, member_names::MemberNamesStage},
        services::member_names,
    },
    outbound::{ExposedDatabase, MemberNamesApi, MemberNamesFiles},
};
use schemars::JsonSchema;
use serde::Serialize;
use std::path::PathBuf;

/// Shared preferred-name capture and refresh arguments.
pub type MemberNamesArgs = StageArgs<MemberNamesStage>;
impl MemberNamesArgs {
    pub(super) fn schema_response(&self) -> Option<Response> {
        matches!(self.input, StageInput::Schema)
            .then(|| Response::schema(&response_help::inspection(run)))
    }
}
pub(super) async fn execute(args: MemberNamesArgs) -> Response {
    if let Some(response) = args.schema_response() {
        return response;
    }
    let (config, key) = args.input.into_execution();
    run(args.stage, config, key).await.into_response()
}
#[derive(Serialize, JsonSchema)]
struct NamesResponse {
    ingestion_key: IngestionKey,
    path: PathBuf,
    members: usize,
}
async fn run(
    stage: MemberNamesStage,
    config: PathBuf,
    key: IngestionKey,
) -> anyhow::Result<CliSuccess<NamesResponse>> {
    let config = LoaderConfig::try_from(&config)?;
    let data = DataConfig::from_env()?;
    let database = ExposedDatabase::new(&data.connection_string).await?;
    let storage = MemberNamesFiles::new(&config.data_dir, &key);
    let members = match stage {
        MemberNamesStage::Fetch => {
            member_names::fetch(&MemberNamesApi::new()?, &storage, &database).await?
        }
        MemberNamesStage::Load => member_names::load(&storage, &database).await?,
    };
    Ok(CliSuccess::new(NamesResponse {
        ingestion_key: key,
        path: std::path::absolute(storage.path())?,
        members,
    }))
}
