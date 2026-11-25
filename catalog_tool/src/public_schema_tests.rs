use serde_json::{json, Value};

fn public(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../public/schema")
        .join(format!("{name}.schema.json"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn current_public_schema_identities_use_library_host() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public/schema");
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".schema.json")
        else {
            continue;
        };
        assert_eq!(
            public(name)["$id"],
            format!("https://workwithsee.github.io/library/schema/{name}.schema.json")
        );
    }
}

#[test]
fn execution_schemas_match_application_and_validate_current_documents() {
    for (name, current) in [
        (
            "workflow-execution",
            s_e_e_core::validation::schema::load_workflow_execution_schema(),
        ),
        (
            "execution-error",
            s_e_e_core::validation::schema::load_execution_error_schema(),
        ),
    ] {
        assert_eq!(
            current["$id"],
            format!("https://workwithsee.github.io/library/schema/{name}.schema.json")
        );
        assert_eq!(public(name), current, "{name}");
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../public/examples/workflow-execution.json");
    let example: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&public("workflow-execution")).unwrap();
    assert!(validator.is_valid(&example));
    let mut invalid = example.clone();
    invalid["runtime_progress"]["completed_tasks"] = json!(-1);
    assert!(!validator.is_valid(&invalid));
    let mut invalid = example;
    invalid["status"] = json!("worker_started");
    assert!(!validator.is_valid(&invalid));
    let errors = jsonschema::validator_for(&public("execution-error")).unwrap();
    assert!(errors
        .is_valid(&json!({"error":"resource limit reached","code":"resource_limit_exceeded"})));
    assert!(!errors.is_valid(&json!({"error":""})));
}

#[test]
fn settings_metadata_matches_application_with_public_identity() {
    for (name, mut current) in [
        (
            "app-settings",
            s_e_e_core::validation::schema::load_app_settings_schema(),
        ),
        (
            "hub-config",
            s_e_e_core::validation::schema::load_hub_config_schema(),
        ),
    ] {
        let published = public(name);
        assert_eq!(
            published["$id"],
            format!("https://workwithsee.github.io/library/schema/{name}.schema.json")
        );
        current["$id"] = published["$id"].clone();
        assert_eq!(published, current, "{name}");
    }
    let schema = public("app-settings");
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&json!({"editor_auto_save":true,"mcp_auth_enabled":true,"mcp_allowed_browser_origins":"http://localhost","default_workflow":"legacy","mcp_enabled":true})));
    assert!(!validator.is_valid(&json!({"editor_auto_save":"true"})));
}

#[test]
fn research_operations_match_registered_markdown_bindings() {
    assert_eq!(
        public("knowledge-research"),
        s_e_e_shapes::tool_contract::research_markdown_public_schema()
    );
}

#[test]
fn research_markdown_example_and_obsolete_fields() {
    let schema = public("knowledge-research");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let example_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../public/examples/knowledge-research.json");
    let example: Value = serde_json::from_slice(&std::fs::read(example_path).unwrap()).unwrap();
    assert!(validator.is_valid(&example));
    let mut old_role = example.clone();
    old_role["pages"][0]["sections"][0]["metadata"]["role"] = json!("method");
    assert!(!validator.is_valid(&old_role));
    let mut global_sources = example.clone();
    global_sources["root"]["metadata"]["sources"] =
        example["pages"][0]["page"]["metadata"]["sources"].clone();
    assert!(!validator.is_valid(&global_sources));
    let mut global_sources = example;
    global_sources["sources"] = json!([]);
    assert!(!validator.is_valid(&global_sources));
    let operations = schema["x-operations"].as_object().unwrap();
    assert!(operations.contains_key("knowledge_research_sources_get"));
    assert!(operations.contains_key("knowledge_research_sources_put"));
    assert!(!operations.contains_key("knowledge_research_profile_get"));
    assert!(!operations.contains_key("knowledge_research_source_put"));
}

#[test]
fn update_catalog_accepts_home_package_and_rejects_shared_or_foreign_identity() {
    let validator = jsonschema::validator_for(&public("application-update-catalog")).unwrap();
    let digest = "a".repeat(64);
    let release = json!({
        "qualification_scope":"automatic", "candidate_id":"macos-g2",
        "app_version":"1.0.2", "release_generation":2, "tracks":["stable"],
        "github_tag":"v1.0.2-macos", "github_prerelease":false, "os":"macos",
        "architecture":"aarch64", "installation_kind":"macos-user-bundle", "scope":"user",
        "minimum_os":"macos", "minimum_os_version":"12.0.0", "package_format":"pkg",
        "archive_layout":"macos-home-package-v1",
        "asset": {"name":"see-macos-aarch64-1.0.2-installer.pkg",
            "url":"https://github.com/workwithsee/releases/releases/download/v1.0.2-macos/see-macos-aarch64-1.0.2-installer.pkg",
            "length":100, "sha256":digest, "unpacked_length":200},
        "runtime_manifest_sha256":digest, "build_identity":format!("runtime-{digest}"),
        "qualification_sha256":digest, "native_identity":"6UUQ92953S/com.see.app",
        "data_read_min":1, "data_read_max":2, "data_write_version":2,
        "rollback_safe":true, "updater_protocol":1,
        "windows_product_version":null, "windows_product_code":null,
        "windows_upgrade_code":null, "withdrawn":false
    });
    let catalog = json!({"schema":1,"releases":[release]});
    assert!(validator.is_valid(&catalog));
    for (field, value) in [
        ("os", "linux"),
        ("installation_kind", "macos-system-package"),
        ("archive_layout", "macos-shared-package-v1"),
        ("native_identity", "FOREIGN/com.see.app"),
        ("build_identity", "unbound"),
    ] {
        let mut invalid = catalog.clone();
        invalid["releases"][0][field] = json!(value);
        assert!(!validator.is_valid(&invalid), "{field}");
    }
    let mut shared = catalog;
    shared["releases"][0]["asset"]["name"] = json!("see-system-installer.pkg");
    assert!(!validator.is_valid(&shared));
}
