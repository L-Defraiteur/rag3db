//! Eight hermetic checks against the attested 927 admission primitives.
//! No source file, environment file, catalog or database is opened.
use rag3weaver::{code, exclusion, generated::GeneratedPolicy};
fn main() {
    let policy = GeneratedPolicy {
        index_exclude: exclusion::ExclusionPolicy {
            minified_line_max_bytes: Some(5000),
            code_file_max_bytes: Some(1048576),
        },
        ..Default::default()
    };
    assert!(code::admission("nested/.git", None, &policy).is_err());
    println!("PASS metadata_git_path_rejected");
    assert!(code::admission("proof.tar.gz", None, &policy).is_err());
    println!("PASS gzip_path_rejected");
    assert!(code::admission("docker-compose.test.yml", Some("services: {}\n"), &policy).is_ok());
    println!("PASS yaml_can_be_admitted_no_suffix_assumption");
    assert!(code::admission("extension/llm/test/.env.example", None, &policy).is_ok());
    println!("PASS env_example_path_is_not_rejected_content_remains_unknown");
    let large_code = "// x\n".repeat(220_000);
    assert_eq!(policy.index_exclude.reason("table.c", &large_code), Some(exclusion::REASON_CODE_TOO_BIG));
    println!("PASS code_over_manifest_cap_rejected");
    assert_eq!(policy.index_exclude.reason("app.js", &"x".repeat(5001)), Some(exclusion::REASON_MINIFIED));
    println!("PASS code_long_line_rejected");
    assert!(code::admission("notes.md", Some(&"x".repeat(9000)), &policy).is_ok());
    println!("PASS_prose_long_line_not_rejected_as_minified");
    assert_eq!(code::admission("sample.txt", Some("a\0b"), &policy), Err(code::REASON_BINARY));
    println!("PASS binary_head_rejected");
}
