use quote::ToTokens;
use std::{collections::BTreeSet, env, fs, path::PathBuf};
use syn::{Attribute, Item, Path, punctuated::Punctuated, visit_mut::VisitMut};

struct ProofAttributes;

fn retain_attributes(attributes: &mut Vec<Attribute>) {
    attributes.retain_mut(|attribute| {
        if attribute.path().is_ident("derive") {
            let mut derives = attribute
                .parse_args_with(Punctuated::<Path, syn::Token![,]>::parse_terminated)
                .expect("parse production derives");
            derives = derives
                .into_iter()
                .filter(|path| {
                    matches!(
                        path.segments.last().unwrap().ident.to_string().as_str(),
                        "Clone"
                            | "Copy"
                            | "Debug"
                            | "Default"
                            | "PartialEq"
                            | "Eq"
                            | "PartialOrd"
                            | "Ord"
                            | "Hash"
                    )
                })
                .collect();
            if derives.is_empty() {
                return false;
            }
            *attribute = syn::parse_quote!(#[derive(#derives)]);
            return true;
        }
        if attribute.path().is_ident("default") {
            return true;
        }
        assert!(
            ["doc", "serde", "strum", "error", "from", "cfg_attr"]
                .iter()
                .any(|name| attribute.path().is_ident(name)),
            "unreviewed attribute on a canonical proof type: {}",
            attribute.to_token_stream()
        );
        false
    });
}

impl VisitMut for ProofAttributes {
    fn visit_visibility_mut(&mut self, visibility: &mut syn::Visibility) {
        if let syn::Visibility::Restricted(restricted) = visibility
            && restricted.path.is_ident("super")
        {
            *visibility = syn::parse_quote!(pub(crate));
        }
    }

    fn visit_item_struct_mut(&mut self, item: &mut syn::ItemStruct) {
        retain_attributes(&mut item.attrs);
        syn::visit_mut::visit_item_struct_mut(self, item);
    }

    fn visit_item_enum_mut(&mut self, item: &mut syn::ItemEnum) {
        retain_attributes(&mut item.attrs);
        syn::visit_mut::visit_item_enum_mut(self, item);
    }

    fn visit_field_mut(&mut self, field: &mut syn::Field) {
        retain_attributes(&mut field.attrs);
        syn::visit_mut::visit_field_mut(self, field);
    }

    fn visit_variant_mut(&mut self, variant: &mut syn::Variant) {
        retain_attributes(&mut variant.attrs);
        syn::visit_mut::visit_variant_mut(self, variant);
    }
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = env::var_os("EXPOSED_SOURCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.ancestors().nth(4).unwrap().to_owned())
        .canonicalize()
        .expect("source root exists");
    println!("cargo:rerun-if-env-changed=EXPOSED_SOURCE_ROOT");
    println!("cargo:rustc-env=EXPOSED_SOURCE_ROOT={}", root.display());
    let models = root.join("exposed/src/lib/domain/models");
    let selections: &[(&str, &[&str])] = &[
        (
            "declaration_cleaning.rs",
            &[
                "AddressMatchQuality",
                "FunderRole",
                "FunderObservationId",
                "FundingEntryId",
            ],
        ),
        ("declaration_ingestion.rs", &["DeclarationId"]),
        ("parliament_member.rs", &["MemberId"]),
        ("entity_ingestion.rs", &["EntityIngestionError"]),
        (
            "declaration_resolution.rs",
            &[
                "AUTOMATIC_THRESHOLD",
                "IdentityBasis",
                "ObservationResolution",
                "PairDisposition",
                "PairReason",
                "PairDecision",
                "AttributionBasis",
                "UnavailableReason",
                "AttributionIssue",
                "AttributionDecision",
                "PaymentAttribution",
                "invalid",
            ],
        ),
        (
            "declaration_resolution/input.rs",
            &[
                "Observation",
                "Payment",
                "ResolutionInput",
                "ScoringInput",
                "ComparisonRow",
                "ScoredPair",
            ],
        ),
    ];
    let mut generated = String::new();
    for (relative, names) in selections {
        let source = models.join(relative);
        println!("cargo:rerun-if-changed={}", source.display());
        let parsed = syn::parse_file(&fs::read_to_string(&source).unwrap()).unwrap();
        let mut remaining: BTreeSet<_> = names.iter().copied().collect();
        for mut item in parsed.items {
            let name = match &item {
                Item::Struct(item) => item.ident.to_string(),
                Item::Enum(item) => item.ident.to_string(),
                Item::Const(item) => item.ident.to_string(),
                Item::Fn(item) => item.sig.ident.to_string(),
                _ => continue,
            };
            if remaining.remove(name.as_str()) {
                ProofAttributes.visit_item_mut(&mut item);
                generated.push_str(&item.into_token_stream().to_string());
                generated.push('\n');
            }
        }
        assert!(
            remaining.is_empty(),
            "missing canonical items in {relative}: {remaining:?}"
        );
    }
    let input_path = models.join("declaration_resolution/input.rs");
    let parsed = syn::parse_file(&fs::read_to_string(input_path).unwrap()).unwrap();
    let root_check = parsed
        .items
        .into_iter()
        .find(|item| matches!(item, Item::Fn(f) if f.sig.ident == "is_root"))
        .expect("canonical is_root");
    generated.push_str(&format!(
        "mod input {{ {} }}\n",
        root_check.into_token_stream()
    ));
    let cleaning =
        syn::parse_file(&fs::read_to_string(models.join("declaration_cleaning.rs")).unwrap())
            .unwrap();
    for item in cleaning.items {
        if let Item::Impl(implementation) = &item
            && let syn::Type::Path(path) = &*implementation.self_ty
            && path.path.is_ident("FunderObservationId")
        {
            for method in &implementation.items {
                if let syn::ImplItem::Fn(method) = method
                    && method.sig.ident == "as_str"
                {
                    generated.push_str(&format!(
                        "impl FunderObservationId {{ {} }}\n",
                        method.to_token_stream()
                    ));
                }
            }
        }
    }
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("domain.rs"),
        generated,
    )
    .unwrap();
    let mut source_projection = String::new();
    for (source, names) in [
        (
            models.join("declaration_ingestion.rs"),
            vec!["CapturedFundingEntry"],
        ),
        (
            root.join("exposed/src/lib/outbound/declaration_source.rs"),
            vec![
                "SourceField",
                "Currency",
                "FundingFields",
                "Amount",
                "has_funding_anchor",
                "funding_entries",
                "parse_funding_entry",
            ],
        ),
    ] {
        println!("cargo:rerun-if-changed={}", source.display());
        let parsed = syn::parse_file(&fs::read_to_string(&source).unwrap()).unwrap();
        let mut remaining: BTreeSet<_> = names.iter().copied().collect();
        for item in parsed.items {
            let name = match &item {
                Item::Struct(item) => Some(item.ident.to_string()),
                Item::Enum(item) => Some(item.ident.to_string()),
                Item::Fn(item) => Some(item.sig.ident.to_string()),
                Item::Impl(item) => match &*item.self_ty {
                    syn::Type::Path(path) => path.path.get_ident().map(ToString::to_string),
                    _ => None,
                },
                _ => None,
            };
            if let Some(name) = name
                && names.contains(&name.as_str())
            {
                remaining.remove(name.as_str());
                source_projection.push_str(&item.into_token_stream().to_string());
                source_projection.push('\n');
            }
        }
        assert!(
            remaining.is_empty(),
            "missing source-projection items: {remaining:?}"
        );
    }
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("source_projection.rs"),
        source_projection,
    )
    .unwrap();
}
