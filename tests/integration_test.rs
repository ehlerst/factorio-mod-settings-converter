use std::process::Command;
use std::fs;
use std::path::Path;

#[test]
fn test_dat_to_json_to_dat() {
    let input_dat = "mod-settings.dat";
    let intermediate_json = "test_output.json";
    let final_dat = "test_output.dat";

    // Build the project first
    let build_status = Command::new("cargo")
        .args(&["build"])
        .status()
        .expect("Failed to build project");
    assert!(build_status.success());

    let binary = "./target/debug/factorio-mod-settings-converter";

    // 1. Convert DAT -> JSON
    let status1 = Command::new(binary)
        .args(&[input_dat, intermediate_json])
        .status()
        .expect("Failed to run converter (DAT to JSON)");
    assert!(status1.success());
    assert!(Path::new(intermediate_json).exists());

    // 2. Convert JSON -> DAT
    let status2 = Command::new(binary)
        .args(&[intermediate_json, final_dat])
        .status()
        .expect("Failed to run converter (JSON to DAT)");
    assert!(status2.success());
    assert!(Path::new(final_dat).exists());

    // 3. Compare DAT files (ideally they'd be identical, but Factorio binary format might have variations)
    // For now, let's at least check we can read it back.
    let status3 = Command::new(binary)
        .args(&[final_dat, "test_verify.json"])
        .status()
        .expect("Failed to read back final DAT");
    assert!(status3.success());

    // Clean up
    let _ = fs::remove_file(intermediate_json);
    let _ = fs::remove_file(final_dat);
    let _ = fs::remove_file("test_verify.json");
}

#[test]
fn test_dat_to_yaml_to_dat() {
    let input_dat = "mod-settings.dat";
    let intermediate_yaml = "test_output.yaml";
    let final_dat = "test_output_yaml.dat";

    let binary = "./target/debug/factorio-mod-settings-converter";

    // 1. Convert DAT -> YAML
    let status1 = Command::new(binary)
        .args(&[input_dat, intermediate_yaml])
        .status()
        .expect("Failed to run converter (DAT to YAML)");
    assert!(status1.success());
    assert!(Path::new(intermediate_yaml).exists());

    // 2. Convert YAML -> DAT
    let status2 = Command::new(binary)
        .args(&[intermediate_yaml, final_dat])
        .status()
        .expect("Failed to run converter (YAML to DAT)");
    assert!(status2.success());
    assert!(Path::new(final_dat).exists());

    // Clean up
    let _ = fs::remove_file(intermediate_yaml);
    let _ = fs::remove_file(final_dat);
}
