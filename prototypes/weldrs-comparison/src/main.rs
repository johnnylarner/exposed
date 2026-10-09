use anyhow::{Context, Result, ensure};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, env, fs};
use weldrs::{
    blocking::{BlockingRule, generate_blocked_pairs},
    comparison::{Comparison, ComparisonBuilder},
    comparison_vectors::compute_comparison_vectors,
    predict::{predict, predict_direct},
    settings::LinkType,
};

#[allow(dead_code)]
mod policy;

#[derive(Clone, Deserialize, Serialize)]
struct Row {
    key: String,
    name: String,
    address: Option<String>,
    blocking_keys: Vec<String>,
    needs_resolution: bool,
}

#[derive(Clone, Deserialize, Serialize)]
struct Pair {
    left: String,
    right: String,
    probability: f64,
    name_level: i32,
    address_level: i32,
}

#[derive(Deserialize, Serialize)]
struct Case {
    name: String,
    rows: Vec<Row>,
    #[serde(default)]
    pairs: Vec<Pair>,
}

#[derive(Deserialize, Serialize)]
struct Cases {
    #[serde(default)]
    generator: Value,
    cases: Vec<Case>,
}

#[derive(Clone, Copy, Debug)]
enum Variant {
    Stock,
    NullNeutral,
    SplinkCompatible,
}

fn frame(rows: &[Row], tokens: bool) -> Result<DataFrame> {
    let expanded: Vec<_> = rows
        .iter()
        .flat_map(|row| {
            if tokens {
                row.blocking_keys
                    .iter()
                    .map(|key| (row, Some(key.as_str())))
                    .collect()
            } else {
                vec![(row, None)]
            }
        })
        .collect();
    let surname = |name: &str| {
        name.chars()
            .rev()
            .take_while(char::is_ascii_lowercase)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    };
    Ok(df!(
        "key" => expanded.iter().map(|(r, _)| r.key.as_str()).collect::<Vec<_>>(),
        "name" => expanded.iter().map(|(r, _)| r.name.as_str()).collect::<Vec<_>>(),
        "name_bytes" => expanded.iter().map(|(r, _)| r.name.bytes().map(char::from).collect::<String>()).collect::<Vec<_>>(),
        "address" => expanded.iter().map(|(r, _)| r.address.as_deref()).collect::<Vec<_>>(),
        "needs_resolution" => expanded.iter().map(|(r, _)| r.needs_resolution).collect::<Vec<_>>(),
        "prefix" => expanded.iter().map(|(r, _)| r.name.chars().take(3).collect::<String>()).collect::<Vec<_>>(),
        "initial" => expanded.iter().map(|(r, _)| r.name.chars().take(1).collect::<String>()).collect::<Vec<_>>(),
        "surname" => expanded.iter().map(|(r, _)| surname(&r.name)).collect::<Vec<_>>(),
        "surname_prefix" => expanded.iter().map(|(r, _)| surname(&r.name).chars().take(2).collect::<String>()).collect::<Vec<_>>(),
        "token" => expanded.iter().map(|(_, token)| *token).collect::<Vec<_>>()
    )?)
}

fn candidates(rows: &[Row]) -> Result<DataFrame> {
    let base = frame(rows, false)?.lazy();
    let token = frame(rows, true)?.lazy();
    let mut frames = Vec::new();
    for (index, columns) in [
        vec!["name"],
        vec!["address"],
        vec!["prefix"],
        vec!["initial", "surname_prefix"],
        vec!["token"],
    ]
    .iter()
    .enumerate()
    {
        let mut pairs = generate_blocked_pairs(
            if index == 4 { &token } else { &base },
            &[BlockingRule::on(columns)],
            &LinkType::DedupeOnly,
            "key",
            None,
        )?;
        if index == 3 {
            for excluded in ["limited", "ltd", "plc", "llp", "company", "inc", "and"] {
                pairs = pairs.filter(col("surname_l").neq(lit(excluded)));
            }
        }
        frames.push(
            pairs
                .filter(col("needs_resolution_l").or(col("needs_resolution_r")))
                .select([
                    col("key_l"),
                    col("key_r"),
                    col("name_l"),
                    col("name_r"),
                    col("name_bytes_l"),
                    col("name_bytes_r"),
                    col("address_l"),
                    col("address_r"),
                ]),
        );
    }
    Ok(concat(frames, UnionArgs::default())?
        .unique_stable(Some(cols(["key_l", "key_r"])), UniqueKeepStrategy::First)
        .collect()?)
}

fn comparisons(model: &Value, variant: Variant) -> Result<Vec<Comparison>> {
    let column = if matches!(variant, Variant::SplinkCompatible) && !cfg!(feature = "splink-simd") {
        "name_bytes"
    } else {
        "name"
    };
    let mut name = ComparisonBuilder::new(column)
        .null_level()
        .exact_match_level()
        .levenshtein_level(2)
        .else_level()
        .build()?;
    name.output_column_name = "name".into();
    let mut address = ComparisonBuilder::new("address")
        .null_level()
        .exact_match_level()
        .else_level()
        .build()?;
    for (comparison, settings) in [&mut name, &mut address].into_iter().zip(
        model["settings"]["comparisons"]
            .as_array()
            .context("comparisons")?,
    ) {
        let levels = settings["comparison_levels"].as_array().context("levels")?;
        ensure!(
            levels.len() == comparison.comparison_levels.len(),
            "comparison shape changed"
        );
        for (level, parameters) in comparison
            .non_null_levels_mut()
            .into_iter()
            .zip(&levels[1..])
        {
            level.m_probability = Some(parameters["m_probability"].as_f64().context("m")?);
            level.u_probability = Some(parameters["u_probability"].as_f64().context("u")?);
        }
    }
    Ok(vec![name, address])
}

fn score(
    rows: &[Row],
    model: &Value,
    variant: Variant,
    direct: bool,
    budget: usize,
) -> Result<Vec<Pair>> {
    if rows.len() < 2 {
        return Ok(Vec::new());
    }
    let blocked = candidates(rows)?;
    ensure!(
        blocked.height() <= budget,
        "scorer exceeded candidate budget"
    );
    if blocked.height() == 0 {
        return Ok(Vec::new());
    }
    let comparisons = comparisons(model, variant)?;
    let mut vectors = compute_comparison_vectors(blocked.lazy(), &comparisons, "gamma_")?;
    if !matches!(variant, Variant::Stock) {
        vectors = vectors.with_column(
            when(col("address_l").is_null().or(col("address_r").is_null()))
                .then(lit(-1i8))
                .otherwise(col("gamma_address"))
                .alias("gamma_address"),
        );
    }
    let prior = model["settings"]["probability_two_random_records_match"]
        .as_f64()
        .context("prior")?;
    let output = if direct {
        predict_direct(
            vectors.collect()?,
            &comparisons,
            prior,
            "gamma_",
            "bf_",
            None,
            None,
        )?
    } else {
        predict(vectors, &comparisons, prior, "gamma_", "bf_", None, None)?.collect()?
    };
    let strings = |column: &str| output.column(column)?.str();
    let left = strings("key_l")?;
    let right = strings("key_r")?;
    let probability = output.column("match_probability")?.f64()?;
    let name_level = output.column("gamma_name")?.i8()?;
    let address_level = output.column("gamma_address")?.i8()?;
    let mut pairs = (0..output.height())
        .map(|i| Pair {
            left: left.get(i).unwrap().into(),
            right: right.get(i).unwrap().into(),
            probability: probability.get(i).unwrap(),
            name_level: i32::from(name_level.get(i).unwrap()),
            address_level: i32::from(address_level.get(i).unwrap()),
        })
        .collect::<Vec<_>>();
    pairs.sort_by(|a, b| (&a.left, &a.right).cmp(&(&b.left, &b.right)));
    Ok(pairs)
}

fn differences(expected: &[Pair], actual: &[Pair]) -> Value {
    let map = |pairs: &[Pair]| {
        pairs
            .iter()
            .map(|p| ((p.left.clone(), p.right.clone()), p.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let expected = map(expected);
    let actual = map(actual);
    let missing = expected
        .keys()
        .filter(|k| !actual.contains_key(*k))
        .collect::<Vec<_>>();
    let extra = actual
        .keys()
        .filter(|k| !expected.contains_key(*k))
        .collect::<Vec<_>>();
    let mut levels = Vec::new();
    let mut scores = Vec::new();
    let mut maximum = 0.0_f64;
    for (key, a) in &actual {
        if let Some(e) = expected.get(key) {
            let delta = (a.probability - e.probability).abs();
            maximum = maximum.max(delta);
            if (a.name_level, a.address_level) != (e.name_level, e.address_level) {
                levels.push(json!({"pair":key,"splink":[e.name_level,e.address_level],"weldrs":[a.name_level,a.address_level]}));
            }
            if delta > 1e-12 * e.probability.abs().max(1e-12) {
                scores.push(json!({"pair":key,"splink":e.probability,"weldrs":a.probability,"absolute_delta":delta}));
            }
        }
    }
    json!({"splink_pairs":expected.len(),"weldrs_pairs":actual.len(),"missing_candidates":missing,"extra_candidates":extra,"level_differences":levels,"score_differences":scores,"maximum_absolute_probability_delta":maximum})
}

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let model: Value = serde_json::from_str(include_str!(
        "../../../exposed/resources/funder-resolution/model.json"
    ))?;
    let mut policies = policy::cases()?;
    if let Some(path) = args.get(if args.get(1).map(String::as_str) == Some("export") {
        3
    } else {
        4
    }) {
        policies.push(policy::from_file(path)?);
    }
    match args.get(1).map(String::as_str) {
        Some("export") if (3..=4).contains(&args.len()) => {
            let mut cases: Cases = serde_json::from_str(include_str!(
                "../../../exposed/tests/fixtures/funder-resolution.json"
            ))?;
            cases.cases.push(Case {
                name: "scalar_distance_counterexamples".into(),
                rows: [
                    "andrew lichnowski",
                    "andy lichnowski",
                    "david luxton",
                    "david lyon",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, name)| Row {
                    key: format!("distance-{i}"),
                    name: name.into(),
                    address: None,
                    blocking_keys: vec!["distance-counterexamples".into()],
                    needs_resolution: true,
                })
                .collect(),
                pairs: Vec::new(),
            });
            cases.cases.extend(policies.iter().map(|p| Case {
                name: p.name.clone(),
                rows: p.rows(),
                pairs: Vec::new(),
            }));
            for case in &mut cases.cases {
                case.pairs.clear();
            }
            fs::write(&args[2], serde_json::to_string_pretty(&cases)?)?;
            println!("Exported {} cases", cases.cases.len());
        }
        Some("compare") if (4..=5).contains(&args.len()) => {
            let baseline: Cases = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
            let mut reports = Vec::new();
            for case in &baseline.cases {
                let policy = policies.iter().find(|p| p.name == case.name);
                if case.name.starts_with("policy_") || case.name == "local_capture" {
                    let policy = policy.context("policy input is required for this case")?;
                    ensure!(
                        serde_json::to_value(policy.rows())? == serde_json::to_value(&case.rows)?,
                        "baseline profiles differ from the current canonical input"
                    );
                }
                let mut variants = Vec::new();
                for variant in [
                    Variant::Stock,
                    Variant::NullNeutral,
                    Variant::SplinkCompatible,
                ] {
                    let actual = score(&case.rows, &model, variant, false, usize::MAX)?;
                    let direct = score(&case.rows, &model, variant, true, usize::MAX)?;
                    let mut reversed = case.rows.clone();
                    reversed.reverse();
                    let reverse = score(&reversed, &model, variant, false, usize::MAX)?;
                    let policy_result = policy.map(|p| {
                        let reference = p.resolve(&case.pairs);
                        let result = p.resolve(&actual);
                        json!({"equal":reference == result,"splink":policy::summary(&reference),"weldrs":policy::summary(&result),
                            "weldrs_budget_outcome": match score(&case.rows, &model, variant, false, p.budget) {
                                Ok(pairs) => policy::summary(&p.resolve(&pairs)), Err(e) => json!({"error":e.to_string()})
                            }})
                    });
                    variants.push(json!({"variant":format!("{variant:?}"),"comparison":differences(&case.pairs,&actual),
                        "direct_vs_lazy":differences(&actual,&direct),"reversed_input":differences(&actual,&reverse),"policy":policy_result}));
                }
                println!(
                    "Compared {} ({} reference pairs)",
                    case.name,
                    case.pairs.len()
                );
                reports.push(json!({"case":case.name,"variants":variants}));
            }
            fs::write(
                &args[3],
                serde_json::to_string_pretty(
                    &json!({"weldrs_version":"0.2.2","distance_engine":if cfg!(feature="splink-simd") { "simd-byte" } else { "scalar-character" },"model":model,"baseline_generator":baseline.generator,"cases":reports}),
                )?,
            )?;
        }
        _ => anyhow::bail!(
            "usage: weldrs-comparison export INPUT.json [CLEANED.json] | compare BASELINE.json REPORT.json [CLEANED.json]"
        ),
    }
    Ok(())
}
