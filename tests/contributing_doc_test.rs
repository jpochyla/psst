use std::fs;
use std::path::Path;

#[test]
fn contributing_file_exists() {
    let path = Path::new("CONTRIBUTING.md");
    assert!(path.exists(), "CONTRIBUTING.md should exist in the repository root");
}

#[test]
fn contributing_file_contains_platform_sections() {
    let content = fs::read_to_string("CONTRIBUTING.md")
        .expect("Should be able to read CONTRIBUTING.md");

    assert!(content.contains("## Platform-Specific Code"),
        "Should contain platform-specific code section");
    assert!(content.contains("Linux"),
        "Should mention Linux");
    assert!(content.contains("Windows"),
        "Should mention Windows");
    assert!(content.contains("macOS"),
        "Should mention macOS");
}

#[test]
fn contributing_file_contains_project_structure() {
    let content = fs::read_to_string("CONTRIBUTING.md")
        .expect("Should be able to read CONTRIBUTING.md");

    assert!(content.contains("## Project Structure"),
        "Should contain project structure section");
    assert!(content.contains("psst-core"),
        "Should mention psst-core crate");
    assert!(content.contains("psst-gui"),
        "Should mention psst-gui crate");
    assert!(content.contains("psst-cli"),
        "Should mention psst-cli crate");
}

#[test]
fn contributing_file_contains_maintainer_information() {
    let content = fs::read_to_string("CONTRIBUTING.md")
        .expect("Should be able to read CONTRIBUTING.md");

    assert!(content.contains("Platform Maintainer"),
        "Should contain information about becoming a platform maintainer");
    assert!(content.contains("Linux and Windows platform maintainers"),
        "Should call out need for Linux and Windows maintainers");
}

#[test]
fn contributing_file_is_not_empty() {
    let content = fs::read_to_string("CONTRIBUTING.md")
        .expect("Should be able to read CONTRIBUTING.md");

    assert!(content.len() > 500,
        "CONTRIBUTING.md should have substantial content");
}
