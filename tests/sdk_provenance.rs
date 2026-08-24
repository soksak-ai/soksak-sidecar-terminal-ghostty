use std::fs;

#[test]
fn external_sdk_metadata_has_one_owner() {
    let raw = fs::read_to_string("build-dependencies.json").expect("read build dependencies");
    let document: serde_json::Value = serde_json::from_str(&raw).expect("parse build dependencies");
    let dependencies = document["dependencies"]
        .as_array()
        .expect("dependencies must be an array");
    assert_eq!(dependencies.len(), 1);
    let dependency = &dependencies[0];
    assert_eq!(dependency["id"], "ghostty-vt-sdk");
    let repository = dependency["repository"].as_str().expect("repository");
    let commit = dependency["commit"].as_str().expect("commit");
    let zig = dependency["tools"]["zig"].as_str().expect("Zig version");

    for name in [
        "Makefile",
        "build.rs",
        "README.md",
        ".github/workflows/release.yml",
    ] {
        let source =
            fs::read_to_string(name).unwrap_or_else(|error| panic!("read {name}: {error}"));
        for duplicated in [repository, commit, zig] {
            assert!(
                !source.contains(duplicated),
                "{name} duplicates build-dependencies.json metadata"
            );
        }
    }
}
