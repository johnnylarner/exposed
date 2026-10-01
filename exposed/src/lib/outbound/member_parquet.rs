//! Schema-v1 Parquet translation. Domain rules are applied only after reading.
use crate::domain::models::member_ingestion::{
    HouseMembership, MemberHistory, MemberObservations, MemberProfile,
};
use anyhow::{Context, ensure};
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
        Field::new("party_id", DataType::Int32, true),
        Field::new("party_name", DataType::Utf8, true),
        Field::new("latest_house", DataType::Int16, false),
        Field::new("latest_membership_from", DataType::Utf8, true),
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
    let profiles = &data.profiles;
    write_batch(
        &path.join("profiles.parquet"),
        profiles_schema(),
        vec![
            Arc::new(Int32Array::from_iter_values(
                profiles.iter().map(|p| p.parliament_member_id),
            )),
            Arc::new(StringArray::from_iter_values(
                profiles.iter().map(|p| &p.name),
            )),
            Arc::new(Int32Array::from_iter(profiles.iter().map(|p| p.party_id))),
            Arc::new(StringArray::from_iter(
                profiles.iter().map(|p| p.party_name.as_deref()),
            )),
            Arc::new(Int16Array::from_iter_values(
                profiles.iter().map(|p| p.latest_house),
            )),
            Arc::new(StringArray::from_iter(
                profiles.iter().map(|p| p.latest_membership_from.as_deref()),
            )),
        ],
    )?;
    write_batch(
        &path.join("current_commons.parquet"),
        current_schema(),
        vec![Arc::new(Int32Array::from(data.current_commons.clone()))],
    )?;
    let rows: Vec<_> = data
        .histories
        .iter()
        .flat_map(|h| {
            h.house_memberships
                .iter()
                .map(move |m| (h.parliament_member_id, m))
        })
        .collect();
    write_batch(
        &path.join("house_memberships.parquet"),
        memberships_schema(),
        vec![
            Arc::new(Int32Array::from_iter_values(rows.iter().map(|(id, _)| *id))),
            Arc::new(Int16Array::from_iter_values(
                rows.iter().map(|(_, m)| m.house),
            )),
            Arc::new(Date32Array::from_iter_values(
                rows.iter().map(|(_, m)| m.start_date.to_epoch_days()),
            )),
            Arc::new(Date32Array::from_iter(
                rows.iter()
                    .map(|(_, m)| m.end_date.map(|d| d.to_epoch_days())),
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
            profiles.push(MemberProfile {
                parliament_member_id: ids.value(row),
                name: names.value(row).into(),
                party_id: (!parties.is_null(row)).then(|| parties.value(row)),
                party_name: (!party_names.is_null(row)).then(|| party_names.value(row).into()),
                latest_house: houses.value(row),
                latest_membership_from: (!locations.is_null(row))
                    .then(|| locations.value(row).into()),
            });
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
            histories.entry(id).or_default().push(HouseMembership {
                house: houses.value(row),
                start_date: date(starts.value(row))?,
                end_date: if ends.is_null(row) {
                    None
                } else {
                    Some(date(ends.value(row))?)
                },
            });
        }
    }
    Ok(MemberObservations {
        profiles,
        current_commons,
        histories: histories
            .into_iter()
            .map(|(parliament_member_id, house_memberships)| MemberHistory {
                parliament_member_id,
                house_memberships,
            })
            .collect(),
    })
}
fn read_batches(path: &Path, schema: &SchemaRef) -> anyhow::Result<Vec<RecordBatch>> {
    let result = (|| {
        let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?;
        ensure!(
            builder.schema().fields() == schema.fields(),
            "Parquet types, fields or nullability do not match schema v1"
        );
        let batches = builder.build()?.collect::<Result<Vec<_>, _>>()?;
        for batch in &batches {
            for (field, column) in schema.fields().iter().zip(batch.columns()) {
                ensure!(
                    field.is_nullable() || column.null_count() == 0,
                    "required column {} contains nulls",
                    field.name()
                );
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
