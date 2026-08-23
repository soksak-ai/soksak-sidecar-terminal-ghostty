use std::fs;

const COMMIT: &str = "9ae02a326f62bd88f7f5508cf1807c67e7775cb5";
const ZIG: &str = "0.16.0";

#[test]
fn sdk_provenance_is_identical_in_build_docs_and_release() {
    let build = fs::read_to_string("build.rs").expect("read build.rs");
    let readme = fs::read_to_string("README.md").expect("read README.md");
    let workflow = fs::read_to_string(".github/workflows/release.yml").expect("read workflow");
    for (name, source) in [
        ("build.rs", build),
        ("README.md", readme),
        ("release.yml", workflow),
    ] {
        assert!(
            source.contains(COMMIT),
            "{name} does not contain the exact Ghostty commit"
        );
        assert!(
            source.contains(ZIG),
            "{name} does not contain the exact Zig version"
        );
    }
}
