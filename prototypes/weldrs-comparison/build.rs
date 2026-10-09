use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../exposed/src/lib/domain/models");
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());
    let modules = [
        "declaration_cleaning",
        "declaration_ingestion",
        "declaration_resolution",
        "funder_name",
    ];
    let mut declarations = String::new();
    for module in modules {
        let source = root.join(format!("{module}.rs"));
        let target = output.join(format!("{module}.rs"));
        fs::copy(&source, &target).unwrap();
        println!("cargo:rerun-if-changed={}", source.display());
        declarations.push_str(&format!(
            "#[path = {path:?}] mod {module};\n",
            path = target.to_str().unwrap()
        ));
    }
    for module in ["attribution", "identity", "input"] {
        let source = root.join(format!("declaration_resolution/{module}.rs"));
        fs::copy(&source, output.join(format!("{module}.rs"))).unwrap();
        println!("cargo:rerun-if-changed={}", source.display());
    }
    fs::write(output.join("canonical.rs"), declarations).unwrap();
}
