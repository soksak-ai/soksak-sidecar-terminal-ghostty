//! Link the target-addressed Ghostty VT archive prepared by the repository Makefile.
//!
//! External source, commit, tool versions and target outputs belong only to
//! `build-dependencies.json`. The Make command validates its generated receipt before Cargo
//! starts. This boundary selects the receipt for Cargo's exact `TARGET`; it does not accept a
//! raw library path and does not discover a checkout.

use std::path::{Path, PathBuf};

fn main() {
    let target = std::env::var("TARGET").expect("Cargo supplies TARGET");
    let root = PathBuf::from(
        std::env::var("SOKSAK_BUILD_DEPENDENCY_ROOT")
            .expect("make build supplies SOKSAK_BUILD_DEPENDENCY_ROOT"),
    );
    assert!(root.is_absolute(), "build dependency root must be absolute");
    let canonical_root = std::fs::canonicalize(&root)
        .unwrap_or_else(|error| panic!("build dependency root is unavailable: {error}"));
    assert_eq!(
        canonical_root, root,
        "build dependency root must not use a symbolic path"
    );

    let receipt_path = root.join("receipts").join(format!("{target}.json"));
    let receipt_bytes = std::fs::read(&receipt_path)
        .unwrap_or_else(|error| panic!("build dependency receipt is unavailable: {error}"));
    let receipt: serde_json::Value = serde_json::from_slice(&receipt_bytes)
        .unwrap_or_else(|error| panic!("build dependency receipt is invalid JSON: {error}"));
    assert_eq!(receipt["schema"], "soksak-build-dependency-receipt-v1");
    assert_eq!(receipt["dependency"], "ghostty-vt-sdk");
    assert_eq!(receipt["target"], target);

    let archive_name = if target.contains("windows") {
        "ghostty-vt-static.lib"
    } else {
        "libghostty-vt.a"
    };
    let relative = format!("targets/{target}/lib/{archive_name}");
    let declared = receipt["outputs"]
        .as_array()
        .expect("build dependency receipt outputs must be an array")
        .iter()
        .any(|output| output["path"].as_str() == Some(relative.as_str()));
    assert!(
        declared,
        "build dependency receipt does not declare {relative}"
    );

    let archive = root.join(Path::new(&relative));
    let canonical_archive = std::fs::canonicalize(&archive)
        .unwrap_or_else(|error| panic!("declared Ghostty archive is unavailable: {error}"));
    assert_eq!(
        canonical_archive, archive,
        "Ghostty archive must not use a symbolic path"
    );
    assert!(
        archive.is_file(),
        "declared Ghostty archive is not a regular file"
    );

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("Cargo supplies OUT_DIR"));
    let staged = out_dir.join(archive_name);
    std::fs::copy(&archive, &staged)
        .unwrap_or_else(|error| panic!("staging {} failed: {error}", archive.display()));

    let link_name = if target.contains("windows") {
        "ghostty-vt-static"
    } else {
        "ghostty-vt"
    };
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static={link_name}");
    println!("cargo:rerun-if-changed={}", receipt_path.display());
    println!("cargo:rerun-if-changed={}", archive.display());
    println!("cargo:rerun-if-env-changed=SOKSAK_BUILD_DEPENDENCY_ROOT");
}
