fn patch_access_spec(spec: &mut serde_yaml::Value) {
    // Access -> accounts sometimes is null !!!
    if let Some(serde_yaml::Value::Mapping(accounts_map)) = spec
        .get_mut("components")
        .and_then(|c| c.get_mut("schemas"))
        .and_then(|s| s.get_mut("Access"))
        .and_then(|a| a.get_mut("properties"))
        .and_then(|p| p.get_mut("accounts"))
    {
        accounts_map.insert(
            serde_yaml::Value::String("nullable".to_string()),
            serde_yaml::Value::Bool(true),
        );
        println!("Patched Access.accounts schema to be nullable.");
    } else {
        println!("cargo:warning=Could not apply patch: Access.accounts path not found.");
    }
}

fn patch_account_resource_spec(spec: &mut serde_yaml::Value) {
    if let Some(serde_yaml::Value::Mapping(accounts_map)) = spec
        .get_mut("components")
        .and_then(|c| c.get_mut("schemas"))
        .and_then(|s| s.get_mut("AccountResource"))
        .and_then(|a| a.get_mut("properties"))
        .and_then(|p| p.get_mut("all_account_ids"))
    {
        accounts_map.insert(
            serde_yaml::Value::String("nullable".to_string()),
            serde_yaml::Value::Bool(true),
        );
        println!("Patched AccountResource.all_account_ids schema to be nullable.");
    } else {
        println!(
            "cargo:warning=Could not apply patch: AccountResource.all_account_ids path not found."
        );
    }
}

fn patch_transaction_spec(spec: &mut serde_yaml::Value) {
    let schema_root = spec
        .get_mut("components")
        .and_then(|c| c.get_mut("schemas"))
        .and_then(|s| s.get_mut("Transaction"))
        .and_then(|t| t.get_mut("properties"));

    if let Some(properties) = schema_root {
        let fields_to_patch = [
            "remittance_information",
            "debtor_account_additional_identification",
            "creditor_account_additional_identification",
        ];

        for field in fields_to_patch {
            if let Some(field_map) = properties.get_mut(field).and_then(|f| f.as_mapping_mut()) {
                field_map.insert(
                    serde_yaml::Value::String("nullable".to_string()),
                    serde_yaml::Value::Bool(true),
                );
                println!("Patched Transaction.{} to be nullable.", field);
            }
        }
    } else {
        println!("cargo:warning=Could not find Transaction.properties schema.");
    }
}

fn main() {
    let src = "./enablebanking-api.yaml";
    println!("cargo:rerun-if-changed={}", src);

    // 1. Open the YAML file
    let file = std::fs::File::open(src).unwrap();

    // 2. Parse it as YAML into a generic JSON Value
    let mut spec_value = serde_yaml::from_reader(file).unwrap();

    patch_access_spec(&mut spec_value);
    patch_account_resource_spec(&mut spec_value);
    patch_transaction_spec(&mut spec_value);

    let spec: openapiv3::OpenAPI = serde_yaml::from_value(spec_value).unwrap();

    // 3. Generate the Rust code using Progenitor
    let mut generator = progenitor::Generator::default();
    let tokens = generator.generate_tokens(&spec).unwrap();
    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    // 4. Write it out to the cargo build target folder
    let mut out_file = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push("codegen.rs");
    std::fs::write(out_file, content).unwrap();
}
