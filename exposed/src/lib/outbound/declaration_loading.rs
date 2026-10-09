use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{
    ExposedDataPipeline,
    declaration_cleaning::{self},
    declaration_resolution::{self, read_table},
    postgres::ExposedDatabase,
};
use crate::domain::{
    models::{
        declaration_cleaning::CapturedMemberDeclarations,
        declaration_ingestion::CapturedFundingEntry,
        declaration_loading::{
            DeclarationLoad, DeclarationLoadOutcome, DeclarationLoadSummary, LoadDeclaration,
            LoadFunder, LoadFundingEntry,
        },
        declaration_resolution::{
            AttributionBasis, AttributionDecision, IdentityBasis, ObservationResolution,
            PairDecision, PaymentAttribution,
        },
        entity_ingestion::EntityIngestionError,
    },
    repositories::{
        declaration_loading::{DeclarationLoadRepository, DeclarationLoadStorage},
        entity_ingestion::EntitySearchPipelineError,
    },
};

impl DeclarationLoadStorage for ExposedDataPipeline {
    async fn read_declaration_load(&self) -> Result<DeclarationLoad, EntityIngestionError> {
        let cleaned = self.cleaned_declarations_path();
        let resolved = self.resolved_declarations_path();
        let (funders, funders_digest) = read_table::<FunderRow>(
            &cleaned.join("funders.parquet"),
            &declaration_cleaning::funders_schema(),
        )?;
        let (payments, payments_digest) = read_table::<CleanPaymentRow>(
            &cleaned.join("funding_entries.parquet"),
            &declaration_cleaning::funding_schema(),
        )?;
        let (identity_rows, _) = read_table::<ObservationResolution>(
            &resolved.join("observation_resolution.parquet"),
            &declaration_resolution::observation_schema(),
        )?;
        let (attribution_rows, _) = read_table::<PaymentAttribution>(
            &resolved.join("payment_attribution.parquet"),
            &declaration_resolution::attribution_schema(),
        )?;
        let (pair_rows, _) = read_table::<PairDecision>(
            &resolved.join("pair_decisions.parquet"),
            &declaration_resolution::pair_schema(),
        )?;
        let manifest_bytes = fs::read(resolved.join("manifest.json")).map_err(read_error)?;
        let manifest: ResolutionManifest =
            serde_json::from_slice(&manifest_bytes).map_err(read_error)?;
        if manifest.schema_version != 1
            || !matches!(
                manifest.policy_version.as_str(),
                "funder-resolution-v2"
                    | crate::domain::models::declaration_resolution::POLICY_VERSION
            )
        {
            return Err(data_error("unsupported resolved declaration manifest"));
        }
        if manifest.input_sha256.get("funders.parquet") != Some(&funders_digest)
            || manifest.input_sha256.get("funding_entries.parquet") != Some(&payments_digest)
            || manifest.observations != identity_rows.len()
            || manifest.payments != attribution_rows.len()
            || manifest.pair_decisions != pair_rows.len()
            || funders.len() != identity_rows.len()
            || payments.len() != attribution_rows.len()
        {
            return Err(data_error(
                "resolved outputs do not match their manifest inputs or counts",
            ));
        }

        let mut raw_paths = fs::read_dir(self.raw_path().join("declarations"))
            .map_err(read_error)?
            .map(|entry| entry.map(|entry| entry.path()).map_err(read_error))
            .collect::<Result<Vec<_>, _>>()?;
        raw_paths.retain(|path| {
            path.extension()
                .is_some_and(|extension| extension == "parquet")
        });
        raw_paths.sort();
        if raw_paths.is_empty() {
            return Err(data_error("no raw declaration partitions found"));
        }

        let mut declarations = BTreeMap::new();
        let mut expected_funding_ids = BTreeSet::new();
        let mut raw_funding = BTreeMap::new();
        for path in &raw_paths {
            let partition =
                declaration_cleaning::read_partition(path).map_err(EntityIngestionError::from)?;
            if let CapturedMemberDeclarations::Populated {
                member,
                declarations: rows,
            } = partition
            {
                for evidence in rows {
                    let declaration = evidence.declaration();
                    let row = LoadDeclaration {
                        source_declaration_id: declaration.id().value(),
                        member_id: member.member_id(),
                        parliament_member_id: member.parliament_member_id(),
                        category_id: declaration.category_id(),
                        category_name: declaration.category_name().to_owned(),
                        register_id: declaration.register_id(),
                        register_published_date: declaration.register_published_date(),
                        parent_declaration_id: declaration.parent_id().map(|id| id.value()),
                        fetched_at: declaration.fetched_at(),
                        registration_date: declaration.registration_date(),
                    };
                    if declarations
                        .insert(row.source_declaration_id, row)
                        .is_some()
                    {
                        return Err(data_error(
                            "duplicate source declaration across raw partitions",
                        ));
                    }
                    for (ordinal, funding) in declaration.funding_entries().iter().enumerate() {
                        let ordinal = u32::try_from(ordinal).map_err(db_error)?;
                        let funding_id = format!(
                            "{}/{}/{}/funding/{ordinal}",
                            member.member_id(),
                            declaration.id().value(),
                            declaration.register_id()
                        );
                        if !expected_funding_ids.insert(funding_id.clone())
                            || raw_funding.insert(funding_id, funding.clone()).is_some()
                        {
                            return Err(data_error("duplicate raw funding occurrence"));
                        }
                    }
                }
            }
        }
        let cleaned_funding_ids = payments
            .iter()
            .map(|payment| payment.funding_entry_id.as_str())
            .collect::<BTreeSet<_>>();
        if cleaned_funding_ids.len() != payments.len()
            || cleaned_funding_ids.len() != expected_funding_ids.len()
            || expected_funding_ids
                .iter()
                .any(|id| !cleaned_funding_ids.contains(id.as_str()))
        {
            return Err(data_error(
                "cleaned funding occurrences do not cover the raw capture",
            ));
        }

        let identity_row_count = identity_rows.len();
        let mut identities = BTreeMap::new();
        for row in identity_rows {
            let unresolved = match row.identity_basis {
                IdentityBasis::Unresolved => true,
                IdentityBasis::SourceReportedCompany
                | IdentityBasis::StatisticalLink
                | IdentityBasis::DonorNameLink
                | IdentityBasis::ExtractedNameLink
                | IdentityBasis::TradeUnionFamilyLink
                | IdentityBasis::StatisticalAndDonorNameLink
                | IdentityBasis::StatisticalAndSupportingNameLink
                | IdentityBasis::ProvisionalSingleton => false,
            };
            if unresolved != row.identity_id.is_none()
                || row.identity_id.as_deref().is_some_and(str::is_empty)
                || identities
                    .insert(row.funder_id.as_str().to_owned(), row.identity_id)
                    .is_some()
            {
                return Err(data_error(
                    "invalid or duplicate resolved observation identity",
                ));
            }
        }
        let observation_row_count = funders.len();
        let observations = funders
            .into_iter()
            .map(|row| (row.funder_id.clone(), row))
            .collect::<BTreeMap<_, _>>();
        if identities.len() != identity_row_count
            || observations.len() != observation_row_count
            || identities.len() != observations.len()
        {
            return Err(data_error("duplicate resolved observation identity"));
        }
        let mut pair_ids = BTreeSet::new();
        let payments_by_id = payments
            .iter()
            .map(|payment| (payment.funding_entry_id.clone(), payment.clone()))
            .collect::<BTreeMap<_, _>>();
        for row in &pair_rows {
            if row.left_funder_id == row.right_funder_id
                || row.left_funder_id >= row.right_funder_id
                || !pair_ids.insert((row.left_funder_id.as_str(), row.right_funder_id.as_str()))
                || !observations.contains_key(row.left_funder_id.as_str())
                || !observations.contains_key(row.right_funder_id.as_str())
                || !row.probability.is_finite()
                || !(0.0..=1.0).contains(&row.probability)
            {
                return Err(data_error("invalid resolved pair decision"));
            }
        }

        let mut attributions = BTreeMap::new();
        for row in attribution_rows {
            let funding_id = row.funding_entry_id.as_str().to_owned();
            if attributions.insert(funding_id, row).is_some() {
                return Err(data_error("duplicate funding attribution"));
            }
        }
        let mut selected_identity_ids = BTreeSet::new();
        let mut funding_entries = Vec::with_capacity(payments.len());
        for payment in payments {
            if raw_funding.remove(&payment.funding_entry_id) != Some(payment.funding.clone()) {
                return Err(data_error(
                    "cleaned funding values disagree with raw capture",
                ));
            }
            let declaration = declarations.get(&payment.declaration_id).ok_or_else(|| {
                data_error("cleaned funding row references a missing declaration")
            })?;
            if payment.member_id != declaration.member_id.to_string()
                || payment.parliament_member_id != declaration.parliament_member_id
                || payment.category_id != declaration.category_id
                || payment.category_name != declaration.category_name
                || payment.register_id != declaration.register_id
                || payment.register_published_date != declaration.register_published_date
                || payment.parent_declaration_id != declaration.parent_declaration_id
                || payment.funding_entry_id
                    != format!(
                        "{}/{}/{}/funding/{}",
                        declaration.member_id,
                        payment.declaration_id,
                        payment.register_id,
                        payment.funding_ordinal
                    )
                || payment.fetched_at != declaration.fetched_at
                || payment.registration_date != declaration.registration_date
            {
                return Err(data_error(
                    "cleaned funding row disagrees with raw declaration metadata",
                ));
            }
            let attribution = attributions
                .remove(&payment.funding_entry_id)
                .ok_or_else(|| data_error("funding entry has no resolved attribution"))?;
            let (
                selected_observation_id,
                identity_id,
                attribution_basis,
                parent_id,
                unavailable_reason,
                issues,
            ) = match attribution.decision {
                AttributionDecision::Selected {
                    selected_funder_id,
                    attribution_basis,
                    selected_parent_declaration_id,
                } => {
                    let observation =
                        observations
                            .get(selected_funder_id.as_str())
                            .ok_or_else(|| {
                                data_error("attribution references a missing observation")
                            })?;
                    let expected_declaration_id =
                        selected_parent_declaration_id.unwrap_or(payment.declaration_id);
                    let expected_role = match attribution_basis {
                        AttributionBasis::ExplicitUltimatePayer => "ultimate_payer",
                        AttributionBasis::ParentPayer | AttributionBasis::Payer => "payer",
                        AttributionBasis::Donor => "donor",
                    };
                    if observation.member_id != payment.member_id
                        || observation.declaration_id != expected_declaration_id
                        || observation.role != expected_role
                        || (selected_parent_declaration_id.is_some()
                            && selected_parent_declaration_id != payment.parent_declaration_id)
                        || (selected_parent_declaration_id.is_some() && observation.role != "payer")
                        || (selected_parent_declaration_id.is_none()
                            && observation.source_scope == "funding_entry"
                            && observation.funding_entry_id.as_deref()
                                != Some(payment.funding_entry_id.as_str()))
                    {
                        return Err(data_error(
                            "selected observation does not belong to this payment or its parent",
                        ));
                    }
                    let identity_id = identities
                        .get(selected_funder_id.as_str())
                        .ok_or_else(|| data_error("attribution references a missing observation"))?
                        .clone();
                    let identity_id = identity_id.ok_or_else(|| {
                        data_error("selected observation has no resolved identity")
                    })?;
                    selected_identity_ids.insert(identity_id.clone());
                    (
                        Some(selected_funder_id.as_str().to_owned()),
                        Some(identity_id),
                        Some(attribution_basis),
                        selected_parent_declaration_id,
                        None,
                        attribution.issues,
                    )
                }
                AttributionDecision::Unavailable { unavailable_reason } => (
                    None,
                    None,
                    None,
                    None,
                    Some(unavailable_reason),
                    attribution.issues,
                ),
            };
            funding_entries.push(LoadFundingEntry {
                source_id: payment.funding_entry_id,
                source_declaration_id: payment.declaration_id,
                identity_id,
                selected_observation_id,
                attribution_basis,
                selected_parent_declaration_id: parent_id,
                unavailable_reason,
                issues,
                amount: payment
                    .funding
                    .amount()
                    .map(str::parse::<BigDecimal>)
                    .transpose()
                    .map_err(|error| data_error(format!("invalid funding amount: {error}")))?,
                currency: payment.funding.currency().map(str::to_owned),
                payment_type: payment.funding.payment_type().map(str::to_owned),
            });
        }
        if !attributions.is_empty() {
            return Err(data_error(
                "resolved output contains an attribution without a funding entry",
            ));
        }
        if !raw_funding.is_empty() {
            return Err(data_error("cleaned funding rows omit raw funding values"));
        }
        if observations.values().any(|row| {
            !matches!(row.role.as_str(), "donor" | "payer" | "ultimate_payer")
                || !matches!(row.source_scope.as_str(), "declaration" | "funding_entry")
                || row.funder_id.is_empty()
                || row.member_id.parse::<uuid::Uuid>().is_err()
                || row.declaration_id == 0
                || (row.source_scope == "declaration" && row.funding_entry_id.is_some())
                || (row.source_scope == "funding_entry"
                    && row.funding_entry_id.as_ref().is_none_or(|id| {
                        payments_by_id.get(id).is_none_or(|payment| {
                            payment.member_id != row.member_id
                                || payment.declaration_id != row.declaration_id
                                || match row.role.as_str() {
                                    "donor" => payment.donor_funder_id.as_deref(),
                                    "payer" => payment.payer_funder_id.as_deref(),
                                    "ultimate_payer" => payment.ultimate_payer_funder_id.as_deref(),
                                    _ => None,
                                } != Some(row.funder_id.as_str())
                        })
                    }))
                || (row.source_scope == "declaration"
                    && !declarations.values().any(|declaration| {
                        declaration.member_id.to_string() == row.member_id
                            && declaration.source_declaration_id == row.declaration_id
                    }))
        }) {
            return Err(data_error("invalid cleaned funder observation"));
        }
        let mut funder_groups = BTreeMap::<String, Vec<&FunderRow>>::new();
        for (observation_id, identity_id) in &identities {
            let Some(identity_id) = identity_id.as_ref() else {
                continue;
            };
            if selected_identity_ids.contains(identity_id) {
                funder_groups
                    .entry(identity_id.clone())
                    .or_default()
                    .push(&observations[observation_id]);
            }
        }
        let mut load_funders = Vec::with_capacity(funder_groups.len());
        for (identity_id, mut group) in funder_groups {
            group.sort_by(|left, right| left.funder_id.cmp(&right.funder_id));
            let company_number = group
                .iter()
                .filter(|row| row.donor_kind.as_deref() == Some("Company"))
                .find_map(|row| row.donor_company_number.clone());
            let kind = if company_number.is_some() {
                Some("Company".to_owned())
            } else {
                group.iter().find_map(|row| row.donor_kind.clone())
            };
            let names = group
                .iter()
                .filter_map(|row| row.name_raw.as_deref())
                .filter(|name| !name.trim().is_empty());
            let aliases = group
                .iter()
                .filter_map(|row| row.name_raw.as_ref())
                .filter(|name| !name.trim().is_empty())
                .cloned()
                .collect::<BTreeSet<_>>();
            let name = canonical_name(names).map_or_else(
                || {
                    company_number.as_ref().map_or_else(
                        || "Unnamed resolved funder".to_owned(),
                        |number| format!("Companies House {number}"),
                    )
                },
                str::to_owned,
            );
            let aliases = aliases.into_iter().collect();
            load_funders.push(LoadFunder {
                identity_id,
                name,
                aliases,
                kind,
                company_number,
            });
        }

        let fingerprint = fingerprint_files(raw_paths.iter().cloned().chain([
            cleaned.join("funders.parquet"),
            cleaned.join("funding_entries.parquet"),
            resolved.join("observation_resolution.parquet"),
            resolved.join("payment_attribution.parquet"),
            resolved.join("pair_decisions.parquet"),
            resolved.join("manifest.json"),
        ]))?;
        DeclarationLoad::checked(
            self.key().into(),
            fingerprint,
            declarations.into_values().collect(),
            load_funders,
            funding_entries,
        )
    }
}

fn canonical_name<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for name in names {
        *counts.entry(name).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by(|(left, left_count), (right, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| {
                    let left_unannotated = !left.contains('(');
                    let right_unannotated = !right.contains('(');
                    left_unannotated.cmp(&right_unannotated)
                })
                .then_with(|| right.cmp(left))
        })
        .map(|(name, _)| name)
}

#[derive(Deserialize)]
struct ResolutionManifest {
    schema_version: u32,
    policy_version: String,
    input_sha256: BTreeMap<String, String>,
    observations: usize,
    payments: usize,
    pair_decisions: usize,
}

#[derive(Deserialize)]
struct FunderRow {
    funder_id: String,
    member_id: String,
    declaration_id: u32,
    role: String,
    source_scope: String,
    funding_entry_id: Option<String>,
    name_raw: Option<String>,
    donor_kind: Option<String>,
    donor_company_number: Option<String>,
}

#[derive(Deserialize, Clone)]
struct CleanPaymentRow {
    funding_entry_id: String,
    member_id: String,
    parliament_member_id: u32,
    declaration_id: u32,
    register_id: u32,
    category_id: u32,
    category_name: String,
    parent_declaration_id: Option<u32>,
    register_published_date: NaiveDate,
    funding_ordinal: u32,
    registration_date: Option<NaiveDate>,
    fetched_at: DateTime<Utc>,
    donor_funder_id: Option<String>,
    payer_funder_id: Option<String>,
    ultimate_payer_funder_id: Option<String>,
    #[serde(flatten)]
    funding: CapturedFundingEntry,
}

fn fingerprint_files(
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<String, EntityIngestionError> {
    let mut paths = paths.into_iter().collect::<Vec<_>>();
    paths.sort();
    let mut digest = Sha256::new();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| data_error("artifact filename is not UTF-8"))?;
        let bytes = fs::read(&path).map_err(read_error)?;
        digest.update(u64::try_from(name.len()).unwrap_or(u64::MAX).to_be_bytes());
        digest.update(name.as_bytes());
        digest.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        digest.update(bytes);
    }
    Ok(format!("{:x}", digest.finalize()))
}

impl DeclarationLoadRepository for ExposedDatabase {
    async fn load_declarations(
        &self,
        load: &DeclarationLoad,
    ) -> Result<DeclarationLoadSummary, EntityIngestionError> {
        let mut tx = self.pool().begin().await.map_err(db_error)?;
        sqlx::query!("SELECT pg_advisory_xact_lock(hashtext('exposed.declaration-loader'))")
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        if let Some(existing) = sqlx::query!(
            "SELECT fingerprint, declarations, funders, funding_entries FROM exposed.declaration_load_runs WHERE ingestion_key = $1",
            load.ingestion_key.uuid()
        ).fetch_optional(&mut *tx).await.map_err(db_error)? {
            if existing.fingerprint != load.fingerprint {
                return Err(data_error("this ingestion key was loaded from different artifacts"));
            }
            tx.commit().await.map_err(db_error)?;
            return Ok(DeclarationLoadSummary {
                outcome: DeclarationLoadOutcome::AlreadyLoaded,
                declarations: usize::try_from(existing.declarations).map_err(db_error)?,
                funders: usize::try_from(existing.funders).map_err(db_error)?,
                funding_entries: usize::try_from(existing.funding_entries).map_err(db_error)?,
            });
        }

        let mut funder_ids = BTreeMap::new();
        for funder in &load.funders {
            let id = sqlx::query!(
                "INSERT INTO exposed.funders (funder_name, funder_kind, company_number, resolution_identity_id) VALUES ($1, $2, $3, $4) ON CONFLICT (resolution_identity_id) DO UPDATE SET funder_name = EXCLUDED.funder_name, funder_kind = EXCLUDED.funder_kind, company_number = EXCLUDED.company_number RETURNING id",
                funder.name,
                funder.kind,
                funder.company_number,
                funder.identity_id
            ).fetch_one(&mut *tx).await.map_err(db_error)?.id;
            funder_ids.insert(funder.identity_id.as_str(), id);
            sqlx::query!(
                "DELETE FROM exposed.funder_aliases WHERE funder_id = $1",
                id
            )
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
            for alias in funder.aliases.iter().chain(std::iter::once(&funder.name)) {
                sqlx::query!(
                    "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) VALUES ($1, $2) ON CONFLICT (funder_id, funder_alias) DO NOTHING",
                    id,
                    alias
                )
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            }
        }
        for declaration in &load.declarations {
            let stored_member_id = sqlx::query!(
                "SELECT parliament_member_id FROM exposed.members WHERE id = $1",
                declaration.member_id
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?
            .ok_or_else(|| {
                data_error(format!(
                    "member {} must be loaded before declarations",
                    declaration.parliament_member_id
                ))
            })?;
            if stored_member_id.parliament_member_id
                != i32::try_from(declaration.parliament_member_id).map_err(db_error)?
            {
                return Err(data_error(format!(
                    "stored member UUID does not match Parliament member {}",
                    declaration.parliament_member_id
                )));
            }
            let inserted = sqlx::query!(
                "INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (source_declaration_id) DO UPDATE SET member_id = EXCLUDED.member_id, category_id = EXCLUDED.category_id, category_name = EXCLUDED.category_name, fetched_at = EXCLUDED.fetched_at, registration_date = EXCLUDED.registration_date WHERE exposed.declarations.fetched_at <= EXCLUDED.fetched_at RETURNING source_declaration_id",
                i32::try_from(declaration.source_declaration_id).map_err(db_error)?,
                declaration.member_id,
                i32::try_from(declaration.category_id).map_err(db_error)?,
                declaration.category_name,
                sql_timestamp(declaration.fetched_at)?,
                declaration.registration_date.map(|date| date.and_hms_opt(0, 0, 0).expect("midnight is valid").and_utc()).map(sql_timestamp).transpose()?
            ).fetch_optional(&mut *tx).await.map_err(db_error)?;
            if inserted.is_none() {
                return Err(data_error(format!(
                    "declaration {} is already loaded at a newer capture time",
                    declaration.source_declaration_id
                )));
            }
            sqlx::query!(
                "DELETE FROM exposed.funding_entries WHERE source_declaration_id = $1",
                i32::try_from(declaration.source_declaration_id).map_err(db_error)?
            )
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }
        for entry in &load.funding_entries {
            let funder_id = entry
                .identity_id
                .as_deref()
                .map(|key| {
                    funder_ids
                        .get(key)
                        .copied()
                        .ok_or_else(|| data_error("resolved identity was not inserted"))
                })
                .transpose()?;
            let attribution_basis = entry.attribution_basis.as_ref().map(AsRef::as_ref);
            let unavailable_reason = entry.unavailable_reason.as_ref().map(AsRef::as_ref);
            sqlx::query!(
                "INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency, payment_type, source_funding_entry_id, selected_observation_id, attribution_basis, selected_parent_declaration_id, unavailable_reason, attribution_issues) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                i32::try_from(entry.source_declaration_id).map_err(db_error)?,
                funder_id,
                entry.amount,
                entry.currency,
                entry.payment_type,
                entry.source_id,
                entry.selected_observation_id,
                attribution_basis,
                entry.selected_parent_declaration_id.map(i32::try_from).transpose().map_err(db_error)?,
                unavailable_reason,
                serde_json::to_value(&entry.issues).map_err(db_error)?
            ).execute(&mut *tx).await.map_err(db_error)?;
        }
        sqlx::query!(
            "DELETE FROM exposed.funder_aliases AS alias WHERE NOT EXISTS (SELECT 1 FROM exposed.funding_entries AS entry WHERE entry.funder_id = alias.funder_id)"
        )
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        sqlx::query!(
            "DELETE FROM exposed.funders AS funder WHERE NOT EXISTS (SELECT 1 FROM exposed.funding_entries AS entry WHERE entry.funder_id = funder.id)"
        )
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        let summary = load.summary();
        sqlx::query!(
            "INSERT INTO exposed.declaration_load_runs (ingestion_key, fingerprint, declarations, funders, funding_entries) VALUES ($1, $2, $3, $4, $5)",
            load.ingestion_key.uuid(),
            load.fingerprint,
            i64::try_from(summary.declarations).map_err(db_error)?,
            i64::try_from(summary.funders).map_err(db_error)?,
            i64::try_from(summary.funding_entries).map_err(db_error)?
        ).execute(&mut *tx).await.map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        Ok(summary)
    }
}

fn data_error(message: impl Into<String>) -> EntityIngestionError {
    EntityIngestionError::DataError(message.into())
}
fn sql_timestamp(value: DateTime<Utc>) -> Result<time::OffsetDateTime, EntityIngestionError> {
    time::OffsetDateTime::from_unix_timestamp(value.timestamp())
        .and_then(|timestamp| timestamp.replace_nanosecond(value.timestamp_subsec_nanos()))
        .map_err(db_error)
}
fn read_error(error: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::IoError(EntitySearchPipelineError::ReadError(error.to_string()))
}

#[cfg(test)]
mod tests;
fn db_error(error: impl std::fmt::Display) -> EntityIngestionError {
    EntityIngestionError::DataError(error.to_string())
}
