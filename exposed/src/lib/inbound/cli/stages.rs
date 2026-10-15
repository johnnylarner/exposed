use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches};
use serde::Serialize;
use strum::IntoEnumIterator;

use super::config_help;
use crate::domain::models::entity_ingestion::{
    DeclarationIngestionStage, IngestionKey, MemberIngestionStage,
};

pub(super) enum StageInput {
    Schema,
    Execute {
        config: PathBuf,
        ingestion_key: IngestionKey,
    },
}

impl StageInput {
    pub(super) fn into_execution(self) -> (PathBuf, IngestionKey) {
        match self {
            Self::Execute {
                config,
                ingestion_key,
            } => (config, ingestion_key),
            Self::Schema => unreachable!("schema inspection returns before execution"),
        }
    }
}

/// A stage parsed with the inputs required to execute it or inspect its schema.
pub struct StageArgs<S> {
    pub(super) stage: S,
    pub(super) input: StageInput,
}

pub trait Stage: IntoEnumIterator + Serialize + Clone {
    const FAMILY: &'static str;
    fn about(&self) -> &'static str;
    fn config_help(&self) -> String;
    fn requires_key(&self) -> bool;
}

impl Stage for DeclarationIngestionStage {
    const FAMILY: &'static str = "declarations";

    fn about(&self) -> &'static str {
        match self {
            Self::Fetch => "Capture declaration source evidence for stored members",
            Self::Clean => "Clean captured declarations offline",
            Self::Resolve => "Resolve funders and attribute funding offline",
            Self::Load => "Load resolved declarations into the database",
        }
    }

    fn config_help(&self) -> String {
        config_help::declaration(self)
    }

    fn requires_key(&self) -> bool {
        match self {
            Self::Fetch => false,
            Self::Clean | Self::Resolve | Self::Load => true,
        }
    }
}

impl Stage for MemberIngestionStage {
    const FAMILY: &'static str = "members";

    fn about(&self) -> &'static str {
        match self {
            Self::Fetch => "Capture members from the Parliament API",
            Self::Load => "Load captured members into the database",
        }
    }

    fn config_help(&self) -> String {
        config_help::member(self)
    }

    fn requires_key(&self) -> bool {
        match self {
            Self::Fetch => false,
            Self::Load => true,
        }
    }
}

fn name<S: Stage>(stage: &S) -> String {
    serde_json::to_value(stage)
        .expect("stage serializes")
        .as_str()
        .expect("stage is a string")
        .to_owned()
}

impl<S: Stage> Args for StageArgs<S> {
    fn augment_args(command: Command) -> Command {
        S::iter().fold(command.subcommand_required(true), |command, stage| {
            let stage_name = name(&stage);
            let example = format!(
                "Example:\n  exposed data {} {stage_name} --config config.yaml{}\n\nUse --schema to inspect success and failure JSON schemas without running the stage.",
                S::FAMILY,
                if stage.requires_key() { " --ingestion-key 019b733c-2360-7000-8000-000000000001" } else { "" }
            );
            command.subcommand(
                Command::new(stage_name)
                    .about(stage.about())
                    .arg(Arg::new("config").long("config").value_name("CONFIG")
                        .value_parser(clap::value_parser!(PathBuf))
                        .required_unless_present("schema")
                        .help("Path to the stage YAML configuration")
                        .long_help(stage.config_help()))
                    .arg({ let arg = Arg::new("ingestion_key").long("ingestion-key").value_name("INGESTION_KEY")
                        .value_parser(clap::value_parser!(IngestionKey))
                        .help(if stage.requires_key() { "Existing ingestion run UUID. Required to run this stage; omitted with --schema" } else { "Capture run UUID. Defaults to a fresh UUID-v7" });
                        if stage.requires_key() { arg.required_unless_present("schema") } else { arg } })
                    .arg(Arg::new("schema").long("schema").action(ArgAction::SetTrue)
                        .help("Print response schemas without running the stage"))
                    .after_long_help(example),
            )
        })
    }

    fn augment_args_for_update(command: Command) -> Command {
        Self::augment_args(command)
    }
}

impl<S: Stage> FromArgMatches for StageArgs<S> {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let (selected, args) = matches.subcommand().expect("stage is required");
        let stage = S::iter()
            .find(|stage| name(stage) == selected)
            .expect("known stage");
        let input = if args.get_flag("schema") {
            StageInput::Schema
        } else {
            let config = args
                .get_one::<PathBuf>("config")
                .expect("config is required")
                .clone();
            let ingestion_key = args
                .get_one::<IngestionKey>("ingestion_key")
                .cloned()
                .unwrap_or_default();
            StageInput::Execute {
                config,
                ingestion_key,
            }
        };
        Ok(Self { stage, input })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}
