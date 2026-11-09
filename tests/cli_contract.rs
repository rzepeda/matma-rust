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
    let manifest = fs::read_to_string(root.join("tests/mirror/gateway.auto.yaml")).unwrap();
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
    let test = root.join("tests/unit/mirror/test_gateway.rs");
    let original = fs::read_to_string(&test).unwrap();
    assert!(original.contains("panic!"));
    assert!(root.join("tests/mocks/mirror/mock_gateway.rs").exists());
    assert!(root.join("tests/matma_unit.rs").exists());
    fs::write(&test,"// developer-authored test\n").unwrap();
    let second = run(root,&["scaffold","-f","Gateway"]);
    assert!(second.status.success(),"{}",String::from_utf8_lossy(&second.stderr));
    assert_eq!(fs::read_to_string(test).unwrap(),"// developer-authored test\n");
}

#[test]
fn structural_validation_rejects_misnamed_types() {
    let temp = fixture();
    fs::write(temp.path().join("src/wrong.rs"),"pub struct Different;\n").unwrap();
    let output = run(temp.path(), &["validate","--check","-o","json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("expected filename different.rs"));
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
    let before=fs::read(root.join("tests/mirror/gateway.auto.yaml")).unwrap();
    assert!(run(root,&["index"]).status.success());
    assert_eq!(before,fs::read(root.join("tests/mirror/gateway.auto.yaml")).unwrap());
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
