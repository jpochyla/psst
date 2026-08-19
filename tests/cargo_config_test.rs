use std::fs;
use std::path::Path;

#[test]
fn cargo_config_exists() {
    let path = Path::new(".cargo/config.toml");
    assert!(path.exists(), ".cargo/config.toml should exist");
}

#[test]
fn cargo_config_sets_macosx_deployment_target() {
    let content = fs::read_to_string(".cargo/config.toml")
        .expect("Failed to read .cargo/config.toml");
    assert!(
        content.contains("MACOSX_DEPLOYMENT_TARGET"),
        "config.toml should set MACOSX_DEPLOYMENT_TARGET"
    );
}

#[test]
fn cargo_config_deployment_target_is_valid_version() {
    let content = fs::read_to_string(".cargo/config.toml")
        .expect("Failed to read .cargo/config.toml");
    for line in content.lines() {
        if line.contains("MACOSX_DEPLOYMENT_TARGET") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            assert_eq!(parts.len(), 2);
            let value = parts[1].trim().trim_matches('"');
            let version_parts: Vec<&str> = value.split('.').collect();
            assert!(
                version_parts.len() >= 2,
                "Deployment target should be a valid version string"
            );
            for part in &version_parts {
                part.parse::<u32>()
                    .expect("Version components should be numeric");
            }
            return;
        }
    }
    panic!("MACOSX_DEPLOYMENT_TARGET line not found");
}
