//! CLI inbound port

use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};

use crate::{
    domain::{
        models::entity_ingestion::{
            DeclarationIngestionStage, EntityIngestionRequest, IngestionKey, MemberIngestionStage,
        },
        services::declaration_cleaning::DeclarationCleanerService,
        services::declaration_fetch::DeclarationFetcherService,
        services::declaration_loading::DeclarationLoaderService,
        services::declaration_resolution::DeclarationResolverService,
        services::entity_ingestion::{
            EntityFetcherService, EntityIngesterService, FetcherService, IngesterService,
        },
    },
    inbound::cli::config::{
        DataConfig, DeclarationFetcherConfig, DeclarationResolverConfig, FetcherConfig,
        LoaderConfig,
    },
    outbound::{ExposedDataPipeline, ExposedDatabase, ParliamentApiClient, WeldrsFunderScorer},
};

pub mod config;

/// CLI Data Args
#[derive(Parser)]
pub struct DataArgs {
    #[command(subcommand)]
    command: DataCommands,
}

/// CLI Subcommands
#[derive(Subcommand)]
#[allow(missing_docs)]
pub enum DataCommands {
    /// Refresh stored funder groups and direct evidence using the current pipeline policy.
    RefreshFunderGroups,
    /// Show the latest ingestion key and directory path.
    Latest {
        /// YAML configuration with the ingestion `data_dir`.
        #[arg(long)]
        config: PathBuf,
    },
    /// Copy the latest raw capture into a new ingestion run.
    CopyLatestRaw {
        /// YAML configuration with the ingestion `data_dir`.
        #[arg(long)]
        config: PathBuf,
        /// Destination ingestion key. Defaults to a fresh UUID-v7.
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
    Declarations {
        /// Stage to run: fetch, clean, resolve, load, sync, or reconcile.
        stage: DeclarationIngestionStage,
        /// YAML configuration containing the declaration `data_dir`.
        #[arg(long)]
        config: PathBuf,
        /// Ingestion run to clean, resolve, or load.
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
        /// Continue an immutable complete sync capture without source requests.
        #[arg(long)]
        resume: Option<IngestionKey>,
    },
    Members {
        stage: String,
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        ingestion_key: Option<IngestionKey>,
    },
}

/// Entry point for CLI commands
///
/// # Errors
/// - Bad stage arg
/// - Bad config path
/// - Bad config shape
/// - Missing or non-Unicode `DATABASE_URL` for commands that use the database
/// - Service level failures
pub async fn run_cli(args: DataArgs) -> anyhow::Result<()> {
    match args.command {
        DataCommands::RefreshFunderGroups => {
            let config = DataConfig::from_env()?;
            let db = ExposedDatabase::new(&config.connection_string).await;
            let count = db.refresh_funder_groups().await?;
            println!("Refreshed funder groups: {count} identities");
        }
        DataCommands::Latest { config } => {
            let config = LoaderConfig::try_from(&config)?;
            match ExposedDataPipeline::latest_ingestion(&config.data_dir).await? {
                Some((path, key)) => {
                    println!("Ingestion key: {key}");
                    println!("Path: {}", path.display());
                }
                None => println!("No ingestion runs found."),
            }
        }
        DataCommands::CopyLatestRaw {
            config,
            ingestion_key,
        } => {
            let config = LoaderConfig::try_from(&config)?;
            let key = ingestion_key.unwrap_or_default();
            match ExposedDataPipeline::copy_latest_raw(&config.data_dir, key.clone()).await? {
                Some((path, source_key)) => {
                    println!("Source ingestion key: {source_key}");
                    println!("Ingestion key: {key}");
                    println!("Path: {}", path.display());
                }
                None => println!("No ingestion runs found."),
            }
        }
        DataCommands::Declarations {
            stage,
            config,
            ingestion_key,
            resume,
        } => {
            anyhow::ensure!(
                resume.is_none()
                    || matches!(
                        stage,
                        DeclarationIngestionStage::Sync | DeclarationIngestionStage::Reconcile
                    ),
                "--resume is only supported by sync and reconcile"
            );
            match stage {
                DeclarationIngestionStage::Sync | DeclarationIngestionStage::Reconcile => {
                    use crate::domain::{
                        models::declaration_sync::DeclarationSyncMode,
                        services::declaration_sync::{
                            DeclarationSyncOptions, DeclarationSyncService,
                        },
                    };
                    let config = config::DeclarationSyncConfig::try_from(&config)?;
                    let data_config = DataConfig::from_env()?;
                    anyhow::ensure!(
                        ingestion_key.is_none(),
                        "use --resume to continue a sealed synchronization run"
                    );
                    let source = ParliamentApiClient::with_interests_url(
                        config.batch_size,
                        config.interests_url,
                    )?;
                    let repository = ExposedDatabase::new(&data_config.connection_string).await;
                    let storage = crate::outbound::DeclarationSnapshotStore::new(
                        &config.data_dir,
                        config.candidate_budget,
                    )?;
                    let service = DeclarationSyncService::new(repository, storage, source);
                    let artifact_root = config.data_dir.clone();
                    let key = if let Some(run) = resume {
                        service.resume(run).await?
                    } else {
                        let mode = match stage {
                            DeclarationIngestionStage::Sync => DeclarationSyncMode::Daily,
                            DeclarationIngestionStage::Reconcile => DeclarationSyncMode::Reconcile,
                            _ => unreachable!("sync stages only"),
                        };
                        let cutoff = config.cutoff.unwrap_or(
                            chrono::Utc::now()
                                .date_naive()
                                .checked_sub_days(chrono::Days::new(config.indexing_lag_days))
                                .ok_or_else(|| {
                                    anyhow::anyhow!("indexing lag exceeds date range")
                                })?,
                        );
                        service
                            .synchronize(
                                mode,
                                &DeclarationSyncOptions {
                                    cutoff,
                                    overlap_days: config.overlap_days,
                                },
                            )
                            .await?
                    };
                    println!("Published declaration sync run: {key}");
                    let manifest: crate::domain::models::declaration_sync::CompleteDeclarationSnapshot = serde_json::from_slice(&std::fs::read(artifact_root.join(key.to_string()).join("capture.json"))?)?;
                    println!("Excluded declarations: {}", manifest.exclusions.len());
                    println!("Source omissions retained: {}", manifest.omissions.len());
                    println!(
                        "Stored members awaiting reconciliation: {}",
                        manifest.pending_members.len()
                    );
                }
                DeclarationIngestionStage::Fetch => {
                    let config = DeclarationFetcherConfig::try_from(&config)?;
                    let data_config = DataConfig::from_env()?;
                    let api = ParliamentApiClient::new(config.batch_size)?;
                    let db = ExposedDatabase::new(&data_config.connection_string).await;
                    let fs = ExposedDataPipeline::new_with_ingestion_key(
                        &config.data_dir,
                        ingestion_key.unwrap_or_default(),
                    )?;
                    DeclarationFetcherService::new(db, api, fs)
                        .fetch_declarations()
                        .await?;
                }
                DeclarationIngestionStage::Resolve => {
                    let ingestion_key = ingestion_key.ok_or_else(|| {
                        anyhow::anyhow!("declarations resolve requires --ingestion-key")
                    })?;
                    let config = DeclarationResolverConfig::try_from(&config)?;
                    let storage = ExposedDataPipeline::open_cleaned_declarations(
                        &config.data_dir,
                        ingestion_key.clone(),
                    )?;
                    let scorer = WeldrsFunderScorer::new()?;
                    let summary =
                        DeclarationResolverService::new(storage, scorer, config.candidate_budget)
                            .resolve_declarations()
                            .await?;
                    println!("Ingestion key: {ingestion_key}");
                    println!(
                        "Resolved declarations: {}",
                        std::path::absolute(config.data_dir)?
                            .join(ingestion_key.to_string())
                            .join("resolved/declarations")
                            .display()
                    );
                    println!("Observations: {}", summary.observations);
                    println!("Funding occurrences: {}", summary.payments);
                    println!("Pair decisions: {}", summary.pairs);
                }
                DeclarationIngestionStage::Load => {
                    let ingestion_key = ingestion_key.ok_or_else(|| {
                        anyhow::anyhow!("declarations load requires --ingestion-key")
                    })?;
                    let config = LoaderConfig::try_from(&config)?;
                    let data_config = DataConfig::from_env()?;
                    let storage = ExposedDataPipeline::open_resolved_declarations(
                        &config.data_dir,
                        ingestion_key.clone(),
                    )?;
                    let db = ExposedDatabase::new(&data_config.connection_string).await;
                    let summary = DeclarationLoaderService::new(storage, db)
                        .load_declarations()
                        .await?;
                    println!("Ingestion key: {ingestion_key}");
                    match summary.outcome {
                    crate::domain::models::declaration_loading::DeclarationLoadOutcome::Imported => {
                        println!("Loaded declarations: {}", summary.declarations);
                    }
                    crate::domain::models::declaration_loading::DeclarationLoadOutcome::AlreadyLoaded => {
                        println!("Run already loaded. Declarations: {}", summary.declarations);
                    }
                }
                    println!("Resolved identities: {}", summary.funders);
                    println!("Funding occurrences: {}", summary.funding_entries);
                }
                DeclarationIngestionStage::Clean => {
                    let ingestion_key = ingestion_key.ok_or_else(|| {
                        anyhow::anyhow!("declarations clean requires --ingestion-key")
                    })?;
                    let config = LoaderConfig::try_from(&config)?;
                    let fs = ExposedDataPipeline::open_existing_declarations(
                        &config.data_dir,
                        ingestion_key.clone(),
                    )?;
                    let summary = DeclarationCleanerService::new(fs)
                        .clean_declarations()
                        .await?;
                    println!("Ingestion key: {ingestion_key}");
                    println!(
                        "Cleaned declarations: {}",
                        std::path::absolute(&config.data_dir)?
                            .join(ingestion_key.to_string())
                            .join("cleaned/declarations")
                            .display()
                    );
                    println!("Member partitions: {}", summary.member_partitions);
                    println!("Declarations: {}", summary.declarations);
                    println!("Funding entries: {}", summary.funding_entries);
                    println!("Funder observations: {}", summary.funders);
                }
            }
        }
        DataCommands::Members {
            stage,
            config,
            ingestion_key,
        } => {
            let stage = MemberIngestionStage::from_str(&stage)?;
            let ingestion_key = ingestion_key.unwrap_or_default();
            match stage {
                MemberIngestionStage::Fetch => {
                    let config = FetcherConfig::try_from(&config)?;
                    let api = ParliamentApiClient::new(config.batch_size)?;
                    let fs = ExposedDataPipeline::new_with_ingestion_key(
                        &config.data_dir,
                        ingestion_key,
                    )?;
                    let service = FetcherService::new(fs, api);

                    let req = EntityIngestionRequest {};
                    service.fetch_members(&req).await?;
                }
                MemberIngestionStage::Load => {
                    let config = LoaderConfig::try_from(&config)?;
                    let data_config = DataConfig::from_env()?;
                    let fs = ExposedDataPipeline::new_with_ingestion_key(
                        &config.data_dir,
                        ingestion_key,
                    )?;
                    let db = ExposedDatabase::new(&data_config.connection_string).await;
                    let service = IngesterService::new(fs, db);

                    service.load_members().await?;
                }
            }
        }
    }
    Ok(())
}
