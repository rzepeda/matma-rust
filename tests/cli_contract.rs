use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn fixture() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/gateway.rs"), "pub struct Gateway;\n").unwrap();
    fs::write(root.join("src/consumer.rs"), "use crate::gateway::Gateway;\npub struct Consumer { pub gateway: Gateway }\n").unwrap();
    fs::write(root.join("src/import_only.rs"), "use crate::gateway::Gateway;\npub struct ImportOnly;\n").unwrap();
    temp
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_matma-indexer"))
        .current_dir(root).args(args).output().unwrap()
}

#[test]
fn index_impact_and_staleness_contract() {
    let temp = fixture();
    let root = temp.path();
    let index = run(root,&["index","-o","json"]);
    assert!(index.status.success(),"{}",String::from_utf8_lossy(&index.stderr));
    let index_body = String::from_utf8(index.stdout).unwrap();
    assert!(index_body.contains("\"indexed\": 3"),"{index_body}");
    let manifest = fs::read_to_string(root.join("tests/mirror/gateway.rs/gateway.auto.yaml")).unwrap();
    assert!(manifest.contains("level_1"),"{manifest}");
    assert!(manifest.contains("level_2"),"{manifest}");

    let zero = run(root,&["impact","-f","Gateway","-l","0","-o","json"]);
    assert!(zero.status.success(),"{}",String::from_utf8_lossy(&zero.stderr));
    let data: serde_json::Value = serde_json::from_slice(&zero.stdout).unwrap();
    assert_eq!(data["affected_sources"].as_array().unwrap().len(),1);
    assert_eq!(data["unit_tests"].as_array().unwrap().len(),0);
    assert_eq!(data["missing_tests"].as_array().unwrap().len(),1);

    let two = run(root,&["impact","-f","Gateway","-l","2","-o","json"]);
    assert!(two.status.success(),"{}",String::from_utf8_lossy(&two.stderr));
    let data: serde_json::Value = serde_json::from_slice(&two.stdout).unwrap();
    assert_eq!(data["affected_sources"].as_array().unwrap().len(),3);

    fs::write(root.join("src/gateway.rs"),"pub struct Gateway { pub id: usize }\n").unwrap();
    let stale = run(root,&["impact","-f","Gateway"]);
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("stale MATMA index"));
}

#[test]
fn scaffold_is_non_destructive_and_failing_by_design() {
    let temp = fixture();
    let root = temp.path();
    let first = run(root,&["scaffold","-f","Gateway"]);
    assert!(first.status.success(),"{}",String::from_utf8_lossy(&first.stderr));
    let test = root.join("tests/unit/mirror/gateway.rs/test_gateway.rs");
    let original = fs::read_to_string(&test).unwrap();
    assert!(original.contains("panic!"));
    assert!(root.join("tests/mocks/mirror/gateway.rs/mock_gateway.rs").exists());
    let bridged = fs::read_to_string(root.join("src/gateway.rs")).unwrap();
    assert!(bridged.contains("mod matma_unit_gateway;"));
    assert!(bridged.contains("mod matma_mock_gateway;"));
    assert!(!root.join("tests/matma_unit.rs").exists());
    fs::write(&test,"// developer-authored test\n").unwrap();
    let second = run(root,&["scaffold","-f","Gateway"]);
    assert!(second.status.success(),"{}",String::from_utf8_lossy(&second.stderr));
    assert_eq!(fs::read_to_string(test).unwrap(),"// developer-authored test\n");
}

#[test]
fn structural_validation_accepts_native_filename_without_forcing_class_name() {
    let temp = fixture();
    fs::write(temp.path().join("src/wrong.rs"),"pub struct Different;\n").unwrap();
    let output = run(temp.path(), &["validate","--check","-o","json"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn usage_and_import_only_have_distinct_inclusive_levels() {
    let temp = fixture();
    let root = temp.path();
    assert!(run(root, &["index"]).status.success());
    let one = run(root, &["impact","-f","Gateway","-l","1","-o","json"]);
    assert!(one.status.success(),"{}",String::from_utf8_lossy(&one.stderr));
    let first:serde_json::Value = serde_json::from_slice(&one.stdout).unwrap();
    let affected = first["affected_sources"].as_array().unwrap();
    assert_eq!(affected.len(), 2);
    assert!(affected.iter().any(|v|v.as_str()==Some("src/consumer.rs")));
    assert!(!affected.iter().any(|v|v.as_str()==Some("src/import_only.rs")));
    let two = run(root, &["impact","-f","Gateway","-l","2","-o","json"]);
    let second:serde_json::Value = serde_json::from_slice(&two.stdout).unwrap();
    assert_eq!(second["affected_sources"].as_array().unwrap().len(),3);
}

#[test]
fn business_and_regression_are_explicitly_registered_and_not_run() {
    let temp = fixture();
    let root = temp.path();
    fs::create_dir_all(root.join("tests/business")).unwrap();
    fs::create_dir_all(root.join("tests/regression")).unwrap();
    fs::write(root.join("tests/business/order_flow.rs"),"#[test] fn workflow() {}\n").unwrap();
    fs::write(root.join("tests/regression/live.rs"),"#[test] fn end_to_end() {}\n").unwrap();
    fs::write(root.join("tests/matma-verification.yaml"), "entries:\n  - source: src/gateway.rs\n    business:\n      - tests/business/order_flow.rs\n    regression:\n      - tests/regression/live.rs\n").unwrap();
    assert!(run(root,&["index"]).status.success());
    let impact=run(root,&["impact","-f","Gateway","-o","json"]);
    assert!(impact.status.success(),"{}",String::from_utf8_lossy(&impact.stderr));
    let report:serde_json::Value=serde_json::from_slice(&impact.stdout).unwrap();
    assert_eq!(report["business_tests"].as_array().unwrap().len(),1);
    assert_eq!(report["regression_tests"].as_array().unwrap().len(),1);
}

#[test]
fn indexing_is_byte_stable_and_does_not_rewrite_application_source() {
    let temp=fixture();
    let root=temp.path();
    let source=fs::read(root.join("src/gateway.rs")).unwrap();
    assert!(run(root,&["index"]).status.success());
    let before=fs::read(root.join("tests/mirror/gateway.rs/gateway.auto.yaml")).unwrap();
    assert!(run(root,&["index"]).status.success());
    assert_eq!(before,fs::read(root.join("tests/mirror/gateway.rs/gateway.auto.yaml")).unwrap());
    assert_eq!(source,fs::read(root.join("src/gateway.rs")).unwrap());
}

#[test]
fn source_names_are_resolved_by_explicit_alias_and_transitive_hops() {
    let temp=fixture();
    let root=temp.path();
    fs::write(root.join("src/consumer.rs"), "use crate::gateway::Gateway as MainGateway;\npub struct Consumer { pub gateway: MainGateway }\n").unwrap();
    fs::write(root.join("src/top.rs"), "use crate::consumer::Consumer;\npub struct Top { pub inner: Consumer }\n").unwrap();
    assert!(run(root,&["index"]).status.success());
    let one=run(root,&["impact","-f","Gateway","-l","1","-o","json"]);
    let one:serde_json::Value=serde_json::from_slice(&one.stdout).unwrap();
    assert_eq!(one["affected_sources"].as_array().unwrap().len(),2);
    let two=run(root,&["impact","-f","Gateway","-l","2","-o","json"]);
    let two:serde_json::Value=serde_json::from_slice(&two.stdout).unwrap();
    assert!(two["affected_sources"].as_array().unwrap().iter().any(|s|s.as_str()==Some("src/top.rs")));
}

#[test]
fn same_file_mutual_dependencies_are_distinct_and_cycle_safe() {
    let temp = fixture();
    let root = temp.path();
    fs::write(root.join("src/paired.rs"),
        "pub struct A { pub b: Option<Box<B>> }\nimpl A { pub fn interact(&self, b: &B) {} }\npub struct B { pub a: Option<Box<A>> }\nimpl B { pub fn interact(&self, a: &A) {} }\n").unwrap();
    let out = run(root, &["index", "-o", "json"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let data: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(data["indexed"], 5);
    let a_manifest: serde_json::Value = serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/mirror/paired.rs/a.auto.yaml")).unwrap()).unwrap();
    let b_manifest: serde_json::Value = serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/mirror/paired.rs/b.auto.yaml")).unwrap()).unwrap();
    assert_eq!(a_manifest["dependencies"][0]["identity"], "crate::paired::B");
    assert_eq!(b_manifest["dependencies"][0]["identity"], "crate::paired::A");
    assert_eq!(a_manifest["dependents"]["level_1"].as_array().unwrap().len(), 1);
    assert_eq!(b_manifest["dependents"]["level_1"].as_array().unwrap().len(), 1);
    assert_eq!(a_manifest["dependents"]["level_1"][0]["identity"], "crate::paired::B");
    assert_eq!(b_manifest["dependents"]["level_1"][0]["identity"], "crate::paired::A");
    let selection = run(root, &["impact", "-f", "A", "-l", "10000", "-o", "json"]);
    assert!(selection.status.success(), "{}", String::from_utf8_lossy(&selection.stderr));
    let selected: serde_json::Value = serde_json::from_slice(&selection.stdout).unwrap();
    assert_eq!(selected["target"], "crate::paired::A");
    assert_eq!(selected["affected_symbols"].as_array().unwrap().len(), 2);
    assert_eq!(selected["affected_sources"].as_array().unwrap().iter()
        .filter(|file| file.as_str() == Some("src/paired.rs")).count(), 1);
    assert!(selected["affected_symbols"].as_array().unwrap().iter()
        .any(|id| id.as_str() == Some("crate::paired::B")));
    assert!(!run(root, &["impact", "-f", "src/paired.rs"]).status.success());
    assert!(run(root, &["scaffold"]).status.success());
    assert!(root.join("tests/unit/mirror/paired.rs/test_a.rs").is_file());
    assert!(root.join("tests/unit/mirror/paired.rs/test_b.rs").is_file());
    assert!(root.join("tests/mocks/mirror/paired.rs/mock_a.rs").is_file());
}
#[test]
fn cycles_and_diamonds_keep_minimum_reverse_impact_level() {
    let temp = fixture();
    let root = temp.path();
    fs::write(root.join("src/chain.rs"),
        "pub struct A { pub b: B }\npub struct B { pub a: Box<A>, pub c: C }\npub struct C;\npub struct D { pub a: A, pub c: C }\n").unwrap();
    assert!(run(root, &["index"]).status.success());
    let first = run(root, &["impact", "-f", "C", "-l", "1", "-o", "json"]);
    let body: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let ids = body["affected_symbols"].as_array().unwrap();
    assert_eq!(ids.len(), 3);
    assert!(!ids.iter().any(|x| x.as_str() == Some("crate::chain::A")));
    let second = run(root, &["impact", "-f", "C", "-l", "2", "-o", "json"]);
    let body: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(body["affected_symbols"].as_array().unwrap().len(), 4);
    assert_eq!(body["target"], "crate::chain::C");
    let c_manifest: serde_json::Value = serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/mirror/chain.rs/c.auto.yaml")).unwrap()).unwrap();
    assert_eq!(c_manifest["dependents"]["level_1"].as_array().unwrap().len(), 2);
    assert_eq!(c_manifest["dependents"]["level_2"].as_array().unwrap().len(), 1);
    assert_eq!(c_manifest["dependents"]["level_2"][0]["identity"], "crate::chain::A");
}


#[test]
fn unrelated_types_share_file_but_not_tests_mocks_or_impact() {
    let temp = fixture();
    let root = temp.path();
    fs::create_dir_all(root.join("src/services")).unwrap();
    fs::write(root.join("src/external.rs"), "pub struct External;\n").unwrap();
    fs::write(root.join("src/services/order_processing.rs"),
        "use crate::external::External;\npub struct A { pub external: External }\nimpl A { pub fn run(&self) {} }\npub struct B { pub number: u8 }\nimpl B { pub fn run(&self) {} }\n").unwrap();
    assert!(run(root, &["index"]).status.success());
    let a: serde_json::Value = serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/mirror/services/order_processing.rs/a.auto.yaml")).unwrap()).unwrap();
    let b: serde_json::Value = serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/mirror/services/order_processing.rs/b.auto.yaml")).unwrap()).unwrap();
    assert_eq!(a["dependencies"][0]["identity"], "crate::external::External");
    assert!(b["dependencies"].as_array().unwrap().is_empty());
    assert_eq!(a["mirrors"]["mock_file"], "tests/mocks/mirror/services/order_processing.rs/mock_a.rs");
    assert_eq!(b["mirrors"]["mock_file"], "tests/mocks/mirror/services/order_processing.rs/mock_b.rs");

    assert!(run(root, &["scaffold", "-f", "A"]).status.success());
    assert!(root.join("tests/unit/mirror/services/order_processing.rs/test_a.rs").is_file());
    assert!(root.join("tests/mocks/mirror/services/order_processing.rs/mock_a.rs").is_file());
    assert!(!root.join("tests/unit/mirror/services/order_processing.rs/test_b.rs").exists());
    assert!(!root.join("tests/mocks/mirror/services/order_processing.rs/mock_b.rs").exists());
    assert!(run(root, &["scaffold", "-f", "B"]).status.success());
    assert!(root.join("tests/unit/mirror/services/order_processing.rs/test_b.rs").is_file());
    assert!(root.join("tests/mocks/mirror/services/order_processing.rs/mock_b.rs").is_file());

    for level in ["0", "1", "2", "1000"] {
        let result = run(root, &["impact", "-f", "A", "-l", level, "-o", "json"]);
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        let selection: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(selection["affected_symbols"].as_array().unwrap().len(), 1, "level {level}");
        assert_eq!(selection["affected_symbols"][0], "crate::services::order_processing::A");
        assert!(!selection["unit_tests"].as_array().unwrap().iter()
            .any(|v| v.as_str() == Some("tests/unit/mirror/services/order_processing.rs/test_b.rs")));
    }
    let result = run(root, &["impact", "-f", "External", "-l", "2", "-o", "json"]);
    let selection: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(selection["affected_symbols"].as_array().unwrap().len(), 2);
    assert!(!selection["affected_symbols"].as_array().unwrap().iter()
        .any(|v| v.as_str() == Some("crate::services::order_processing::B")));
}
#[test]
fn actual_same_file_dependency_only_expands_reverse_impact_at_level_one() {
    let temp = fixture();
    let root = temp.path();
    fs::write(root.join("src/paired.rs"), "pub struct A;\npub struct B { pub a: A }\npub struct C;\n").unwrap();
    assert!(run(root, &["index"]).status.success());
    let zero = run(root, &["impact", "-f", "A", "-l", "0", "-o", "json"]);
    let a: serde_json::Value = serde_json::from_slice(&zero.stdout).unwrap();
    assert_eq!(a["affected_symbols"].as_array().unwrap().len(), 1);
    let one = run(root, &["impact", "-f", "A", "-l", "1", "-o", "json"]);
    let a: serde_json::Value = serde_json::from_slice(&one.stdout).unwrap();
    let symbols = a["affected_symbols"].as_array().unwrap();
    assert_eq!(symbols.len(), 2);
    assert!(symbols.iter().any(|v| v.as_str() == Some("crate::paired::B")));
    assert!(!symbols.iter().any(|v| v.as_str() == Some("crate::paired::C")));
}
#[test]
fn business_registry_scopes_to_type_and_rejects_ambiguous_file_registration() {
    let temp = fixture();
    let root = temp.path();
    fs::write(root.join("src/paired.rs"), "pub struct A;\npub struct B;\n").unwrap();
    fs::create_dir_all(root.join("tests/business")).unwrap();
    fs::write(root.join("tests/business/a.rs"), "#[test] fn a() {}\n").unwrap();
    fs::write(root.join("tests/business/b.rs"), "#[test] fn b() {}\n").unwrap();
    fs::write(root.join("tests/matma-verification.yaml"),
        "entries:\n  - source: crate::paired::A\n    business:\n      - tests/business/a.rs\n  - source: crate::paired::B\n    business:\n      - tests/business/b.rs\n").unwrap();
    assert!(run(root, &["index"]).status.success());
    let output = run(root, &["impact", "-f", "A", "-o", "json"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["business_tests"].as_array().unwrap().len(), 1);
    assert_eq!(result["business_tests"][0], "tests/business/a.rs");
    fs::write(root.join("tests/matma-verification.yaml"),
        "entries:\n  - source: src/paired.rs\n    business:\n      - tests/business/b.rs\n").unwrap();
    let error = run(root, &["impact", "-f", "A"]);
    assert!(!error.status.success());
    assert!(String::from_utf8_lossy(&error.stderr).contains("ambiguous MATMA verification registration"));
}

#[test]
fn scaffolding_bridges_private_types_without_overwriting_authored_tests() {
    let temp=fixture();
    let root=temp.path();
    fs::write(root.join("src/paired.rs"),
      "struct A { private_value: i32 }\nstruct B { private_value: i32 }\n").unwrap();
    assert!(run(root,&["scaffold","-f","A"]).status.success());
    let source=fs::read_to_string(root.join("src/paired.rs")).unwrap();
    assert!(source.contains("mod matma_unit_a;"));
    assert!(!source.contains("mod matma_unit_b;"));
    assert!(run(root,&["scaffold","-f","B"]).status.success());
    let source2=fs::read_to_string(root.join("src/paired.rs")).unwrap();
    assert_eq!(source2.matches("// MATMA TEST BRIDGES BEGIN").count(),1);
    assert!(source2.contains("mod matma_unit_a;"));
    assert!(source2.contains("mod matma_unit_b;"));
    assert!(source2.contains("mod matma_mock_a;"));
    assert!(source2.contains("mod matma_mock_b;"));
    assert_eq!(source2.matches("#[cfg(test)]").count(),6);
    assert!(run(root,&["scaffold","-f","A"]).status.success());
    assert_eq!(fs::read_to_string(root.join("src/paired.rs")).unwrap(),source2);
    let impact=run(root,&["impact","-f","A","-o","json"]);
    assert!(impact.status.success(),"{}",String::from_utf8_lossy(&impact.stderr));
}


#[test]
fn changes_tracks_individual_ast_fingerprints_and_context_spans() {
    let temp = fixture();
    let root = temp.path();
    let file = root.join("src/paired.rs");
    fs::write(&file, "pub struct A;\nimpl A { pub fn value(&self) -> u8 { 1 } }\npub struct B;\nimpl B { pub fn value(&self) -> u8 { 8 } }\n").unwrap();
    assert!(run(root,&["index"]).status.success());
    let context = run(root,&["context","-f","A","-o","json"]);
    assert!(context.status.success(),"{}",String::from_utf8_lossy(&context.stderr));
    let data:serde_json::Value=serde_json::from_slice(&context.stdout).unwrap();
    let snippets:Vec<_>=data["regions"].as_array().unwrap().iter()
       .map(|v|v["snippet"].as_str().unwrap()).collect();
    assert_eq!(snippets.len(),2);
    assert!(snippets.join("\n").contains("pub struct A"));
    assert!(snippets.join("\n").contains("impl A"));
    assert!(!snippets.join("\n").contains("impl B"));

    fs::write(&file, "pub struct A;\nimpl A { pub fn value(&self) -> u8 { 2 } }\npub struct B;\nimpl B { pub fn value(&self) -> u8 { 8 } }\n").unwrap();
    assert!(run(root,&["index"]).status.success());
    let output=run(root,&["changes","-l","0","-o","json"]);
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let data:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data["changed_symbols"].as_array().unwrap().len(),1);
    assert_eq!(data["changed_symbols"][0],"crate::paired::A");
    assert_eq!(data["affected_symbols"].as_array().unwrap().len(),1);

    // Reindexing an identical AST does not discard the meaningful baseline.
    fs::write(&file, "pub struct A;\nimpl A {pub fn value(&self)->u8{2}}\npub struct B;\nimpl B { pub fn value(&self) -> u8 { 8 } }\n").unwrap();
    assert!(run(root,&["index"]).status.success());
    let output=run(root,&["changes","-l","0","-o","json"]);
    let data:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data["changed_symbols"][0],"crate::paired::A");
}
#[test]
fn changes_requires_two_meaningful_indexed_revisions() {
    let temp=fixture();
    let root=temp.path();
    assert!(run(root,&["index"]).status.success());
    let output=run(root,&["changes"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no preceding type index"));
}

#[test]
fn cross_file_impl_is_owned_by_type_and_trait_has_real_implementor() {
    let temp=fixture();
    let root=temp.path();
    fs::write(root.join("src/contract.rs"),"pub trait Contract { fn value(&self)->u8; }\n").unwrap();
    fs::write(root.join("src/model.rs"),"pub struct Model;\n").unwrap();
    fs::write(root.join("src/implementation.rs"),
        "use crate::model::Model;\nimpl crate::contract::Contract for Model {fn value(&self)->u8{4}}\n").unwrap();
    let index=run(root,&["index"]);
    assert!(index.status.success(),"{}",String::from_utf8_lossy(&index.stderr));
    let trait_index:serde_json::Value=serde_yaml::from_str(
        &fs::read_to_string(root.join("tests/mirror/contract.rs/contract.auto.yaml")).unwrap()).unwrap();
    assert_eq!(trait_index["implementors"][0],"crate::model::Model");
    let impact=run(root,&["impact","-f","Contract","-l","1","-o","json"]);
    let selection:serde_json::Value=serde_json::from_slice(&impact.stdout).unwrap();
    assert!(selection["affected_symbols"].as_array().unwrap().iter()
       .any(|row|row.as_str()==Some("crate::model::Model")));
    let context=run(root,&["context","-f","Model","-o","json"]);
    assert!(context.status.success(),"{}",String::from_utf8_lossy(&context.stderr));
    let result:serde_json::Value=serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(result["regions"].as_array().unwrap().len(),2);
    assert_eq!(result["regions"][1]["source_file"],"src/implementation.rs");
    fs::write(root.join("src/implementation.rs"),
        "use crate::model::Model;\nimpl crate::contract::Contract for Model {fn value(&self)->u8{9}}\n").unwrap();
    assert!(run(root,&["index"]).status.success());
    let diff=run(root,&["changes","-l","0","-o","json"]);
    let changes:serde_json::Value=serde_json::from_slice(&diff.stdout).unwrap();
    assert_eq!(changes["changed_symbols"].as_array().unwrap().len(),1);
    assert_eq!(changes["changed_symbols"][0],"crate::model::Model");
}

#[test]
fn validate_detects_missing_or_damaged_private_bridges() {
    let temp=fixture();
    let root=temp.path();
    fs::write(root.join("src/paired.rs"),"pub struct A;\n").unwrap();
    assert!(run(root,&["scaffold","-f","A"]).status.success());
    let source=fs::read_to_string(root.join("src/paired.rs")).unwrap();
    fs::write(root.join("src/paired.rs"),source.replace("mod matma_unit_a;","mod wrong_test_name;")).unwrap();
    let output=run(root,&["validate","--check","-o","json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("incorrect or stale MATMA test bridge"));
}
