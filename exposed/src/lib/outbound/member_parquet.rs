//! Schema-v1 Parquet translation into valid source observations.
use crate::domain::{
    models::{member_ingestion::MemberObservations, parliament_member::ParliamentMember},
    repositories::parliament_api::{HouseMembership, MemberHistory},
};
use anyhow::{Context, bail, ensure};
use arrow_array::{Array, ArrayRef, Date32Array, Int16Array, Int32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use chrono::NaiveDate;
use parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::Compression,
    file::properties::WriterProperties,
};
use std::{collections::BTreeMap, fs::File, path::Path, sync::Arc};

fn profiles_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("parliament_member_id", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("party_id", DataType::Int32, false),
        Field::new("party_name", DataType::Utf8, false),
        Field::new("latest_house", DataType::Int16, false),
        Field::new("latest_membership_from", DataType::Utf8, false),
    ]))
}
fn current_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![Field::new(
        "parliament_member_id",
        DataType::Int32,
        false,
    )]))
}
fn memberships_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("parliament_member_id", DataType::Int32, false),
        Field::new("house", DataType::Int16, false),
        Field::new("source_start_date", DataType::Date32, false),
        Field::new("source_end_date", DataType::Date32, true),
    ]))
}

pub(super) fn write(path: &Path, data: &MemberObservations) -> anyhow::Result<()> {
    let profiles = data.profiles();
    write_batch(
        &path.join("profiles.parquet"),
        profiles_schema(),
        vec![
            Arc::new(Int32Array::from_iter_values(
                profiles.iter().map(ParliamentMember::parliament_member_id),
            )),
            Arc::new(StringArray::from_iter_values(
                profiles.iter().map(ParliamentMember::name),
            )),
            Arc::new(Int32Array::from_iter_values(
                profiles.iter().map(ParliamentMember::party_id),
            )),
            Arc::new(StringArray::from_iter_values(
                profiles.iter().map(ParliamentMember::party_name),
            )),
            Arc::new(Int16Array::from_iter_values(
                profiles.iter().map(ParliamentMember::latest_house),
            )),
            Arc::new(StringArray::from_iter_values(
                profiles
                    .iter()
                    .map(ParliamentMember::latest_membership_from),
            )),
        ],
    )?;
    write_batch(
        &path.join("current_commons.parquet"),
        current_schema(),
        vec![Arc::new(Int32Array::from(
            data.current_commons().iter().copied().collect::<Vec<_>>(),
        ))],
    )?;
    let rows: Vec<_> = data
        .histories()
        .values()
        .flat_map(|h| {
            h.house_memberships()
                .iter()
                .map(move |m| (h.parliament_member_id(), m))
        })
        .collect();
    write_batch(
        &path.join("house_memberships.parquet"),
        memberships_schema(),
        vec![
            Arc::new(Int32Array::from_iter_values(rows.iter().map(|(id, _)| *id))),
            Arc::new(Int16Array::from_iter_values(
                rows.iter().map(|(_, m)| m.house()),
            )),
            Arc::new(Date32Array::from_iter_values(
                rows.iter().map(|(_, m)| m.start_date().to_epoch_days()),
            )),
            Arc::new(Date32Array::from_iter(
                rows.iter()
                    .map(|(_, m)| m.end_date().map(|d| d.to_epoch_days())),
            )),
        ],
    )?;
    Ok(())
}
fn write_batch(path: &Path, schema: SchemaRef, columns: Vec<ArrayRef>) -> anyhow::Result<()> {
    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    let file = File::create_new(path).with_context(|| format!("create {}", path.display()))?;
    let properties = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();
    let mut writer = ArrowWriter::try_new(&file, schema, Some(properties))?;
    writer.write(&batch)?;
    writer.close()?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn read(path: &Path) -> anyhow::Result<MemberObservations> {
    let mut profiles = Vec::new();
    for batch in read_batches(&path.join("profiles.parquet"), &profiles_schema())? {
        let ids: &Int32Array = column(&batch, 0)?;
        let names: &StringArray = column(&batch, 1)?;
        let parties: &Int32Array = column(&batch, 2)?;
        let party_names: &StringArray = column(&batch, 3)?;
        let houses: &Int16Array = column(&batch, 4)?;
        let locations: &StringArray = column(&batch, 5)?;
        for row in 0..batch.num_rows() {
            profiles.push(ParliamentMember::new(
                names.value(row).into(),
                ids.value(row),
                parties.value(row),
                party_names.value(row).into(),
                houses.value(row),
                locations.value(row).into(),
            )?);
        }
    }
    let mut current_commons = Vec::new();
    for batch in read_batches(&path.join("current_commons.parquet"), &current_schema())? {
        let ids: &Int32Array = column(&batch, 0)?;
        current_commons.extend(ids.values());
    }
    let mut histories: BTreeMap<i32, Vec<HouseMembership>> = BTreeMap::new();
    for batch in read_batches(
        &path.join("house_memberships.parquet"),
        &memberships_schema(),
    )? {
        let ids: &Int32Array = column(&batch, 0)?;
        let houses: &Int16Array = column(&batch, 1)?;
        let starts: &Date32Array = column(&batch, 2)?;
        let ends: &Date32Array = column(&batch, 3)?;
        for row in 0..batch.num_rows() {
            let id = ids.value(row);
            let date = |value| {
                NaiveDate::from_epoch_days(value)
                    .with_context(|| format!("member {id}: invalid source calendar date"))
            };
            histories.entry(id).or_default().push(
                HouseMembership::new(
                    houses.value(row),
                    date(starts.value(row))?,
                    if ends.is_null(row) {
                        None
                    } else {
                        Some(date(ends.value(row))?)
                    },
                )
                .with_context(|| format!("member {id}: invalid house membership"))?,
            );
        }
    }
    let histories = histories
        .into_iter()
        .map(|(id, periods)| MemberHistory::new(id, periods))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MemberObservations::new(
        profiles,
        current_commons,
        histories,
    )?)
}

fn read_batches(path: &Path, schema: &SchemaRef) -> anyhow::Result<Vec<RecordBatch>> {
    let result = (|| {
        let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?;
        // Earlier schema-v1 captures marked profile columns nullable. Accept
        // their metadata, but require every now-required value below.
        let fields = builder.schema().fields();
        ensure!(
            fields.len() == schema.fields().len()
                && fields
                    .iter()
                    .zip(schema.fields())
                    .all(|(actual, expected)| {
                        actual.name() == expected.name()
                            && actual.data_type() == expected.data_type()
                    }),
            "Parquet types or fields do not match schema v1"
        );
        let batches = builder.build()?.collect::<Result<Vec<_>, _>>()?;
        for batch in &batches {
            for (field, column) in schema.fields().iter().zip(batch.columns()) {
                if !field.is_nullable()
                    && let Some(row) = (0..column.len()).find(|&row| column.is_null(row))
                {
                    let member = batch
                        .column_by_name("parliament_member_id")
                        .and_then(|ids| ids.as_any().downcast_ref::<Int32Array>())
                        .and_then(|ids| (!ids.is_null(row)).then(|| ids.value(row)))
                        .map_or_else(|| format!("row {}", row + 1), |id| format!("member {id}"));
                    bail!("{member}: {} is required", field.name());
                }
            }
        }
        Ok(batches)
    })();
    result.with_context(|| format!("read {}", path.display()))
}
fn column<T: Array + 'static>(batch: &RecordBatch, index: usize) -> anyhow::Result<&T> {
    batch
        .column(index)
        .as_any()
        .downcast_ref()
        .context("unexpected Parquet column type")
}
