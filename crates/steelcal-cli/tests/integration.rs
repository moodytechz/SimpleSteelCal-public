//! Integration tests for the steelcal-cli binary.
//!
//! These tests run the compiled binary with various arguments and verify
//! stdout/stderr output and exit codes.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Return the path to the compiled steelcal-cli binary.
fn cli_bin() -> PathBuf {
    // `cargo test` sets this env var so integration tests can find the binary
    // built by the same `cargo test` invocation.
    let mut path = PathBuf::from(env!("CARGO_BIN_EXE_steelcal-cli"));
    // Sanity-check – if the file doesn't exist, fall back to a target/debug path.
    if !path.exists() {
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/debug/steelcal-cli")
            .with_extension(std::env::consts::EXE_EXTENSION);
    }
    path
}

/// Helper: run CLI with given args, return (exit_code, stdout, stderr).
fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(cli_bin())
        .args(args)
        .output()
        .expect("failed to execute steelcal-cli binary");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn temp_csv_output_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("steelcal-{name}-{nanos}.csv"))
}

fn fixture_path(path: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(path)
        .display()
        .to_string()
}

fn expand_fixture_arg(arg: &str) -> String {
    if let Some(relative) = arg.strip_prefix("@ROOT@/") {
        fixture_path(relative)
    } else {
        arg.to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Calculation Mode Tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn sheet_gauge_calculation() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width", "48", "--length", "120", "--qty", "10", "--gauge", "16",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("Each (lb): 100.000"),
        "expected each_lb=100.000 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("Total (lb): 1000.000"),
        "expected total_lb=1000.000 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("psf: 2.500"),
        "expected psf=2.500 in stdout: {stdout}"
    );
}

#[test]
fn sheet_psf_calculation() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width", "48", "--length", "96", "--qty", "1", "--psf", "3.5",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("Each (lb): 112.000"),
        "expected each_lb=112.000 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("psf: 3.500"),
        "expected psf=3.500 in stdout: {stdout}"
    );
}

#[test]
fn sheet_thickness_calculation() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width",
        "48",
        "--length",
        "96",
        "--qty",
        "1",
        "--thickness",
        "0.25",
        "--density",
        "490",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("Each (lb): 326.667"),
        "expected each_lb=326.667 in stdout: {stdout}"
    );
}

#[test]
fn coil_calculation() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width",
        "48",
        "--length",
        "96",
        "--qty",
        "1",
        "--gauge",
        "16",
        "--coil-width",
        "48",
        "--coil-thickness",
        "0.06",
        "--coil-id",
        "20",
        "--coil-weight",
        "2000",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("Linear ft:"),
        "expected coil footage in stdout: {stdout}"
    );
    assert!(
        stdout.contains("PIW (lb/in):"),
        "expected PIW in stdout: {stdout}"
    );
    assert!(
        stdout.contains("Coil OD (in):"),
        "expected Coil OD in stdout: {stdout}"
    );
}

#[test]
fn scrap_calculation() {
    let (code, stdout, _stderr) = run_cli(&[
        "--scrap-actual",
        "5000",
        "--scrap-ending",
        "4800",
        "--scrap-base-cost",
        "0.35",
        "--scrap-processing-cost",
        "0.05",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("Scrap (lb): 200.000"),
        "expected scrap_lb=200.000 in stdout: {stdout}"
    );
    assert!(
        stdout.contains("Pickup: No"),
        "expected Pickup: No in stdout: {stdout}"
    );
}

#[test]
fn scrap_pickup_calculation() {
    let (code, stdout, _stderr) = run_cli(&[
        "--scrap-actual",
        "4500",
        "--scrap-ending",
        "5000",
        "--scrap-base-cost",
        "0.35",
        "--scrap-processing-cost",
        "0.05",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("Pickup: Yes"),
        "expected Pickup: Yes in stdout: {stdout}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// --json Output Tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn json_output_sheet() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width", "48", "--length", "120", "--qty", "10", "--gauge", "16", "--json",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    let sheet = &parsed["sheet"];
    assert_eq!(sheet["each_lb"], 100.0, "JSON each_lb");
    assert_eq!(sheet["total_lb"], 1000.0, "JSON total_lb");
    assert_eq!(sheet["psf"], 2.5, "JSON psf");
    assert_eq!(sheet["used_key"], "16", "JSON used_key");
}

#[test]
fn json_output_coil() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width",
        "48",
        "--length",
        "96",
        "--qty",
        "1",
        "--gauge",
        "16",
        "--coil-width",
        "48",
        "--coil-thickness",
        "0.06",
        "--coil-id",
        "20",
        "--coil-weight",
        "2000",
        "--json",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed["coil"].is_object(), "expected coil object in JSON");
    let coil = &parsed["coil"];
    assert!(
        coil["coil_footage_ft"].as_f64().unwrap() > 200.0,
        "coil footage > 200"
    );
    assert!(
        coil["coil_piw_lb_per_in"].as_f64().unwrap() > 40.0,
        "PIW > 40"
    );
}

#[test]
fn json_output_scrap() {
    let (code, stdout, _stderr) = run_cli(&[
        "--scrap-actual",
        "5000",
        "--scrap-ending",
        "4800",
        "--scrap-base-cost",
        "0.35",
        "--scrap-processing-cost",
        "0.05",
        "--json",
    ]);
    assert_eq!(code, 0, "expected exit 0");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    let scrap = &parsed["scrap"];
    assert_eq!(scrap["scrap_lb"], 200.0, "JSON scrap_lb");
    assert_eq!(scrap["is_pickup"], false, "JSON is_pickup");
}

#[test]
fn batch_json_happy_path_returns_multiple_results() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-happy-path.json");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 2, "expected two successful rows");
    assert_eq!(errors.len(), 0, "expected no errors");
}

#[test]
fn batch_json_invalid_rows_are_reported_without_hiding_valid_rows() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-invalid-row.json");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for partial-success batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 1, "expected one successful row");
    assert_eq!(errors.len(), 1, "expected one row-level error");
}

#[test]
fn batch_coil_json_happy_path_returns_multiple_results() {
    let input_file = fixture_path("fixtures/batch/coil-batch-happy-path.json");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for coil batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 2, "expected two successful coil rows");
    assert_eq!(errors.len(), 0, "expected no errors");
    assert!(results[0]["coil"].is_object(), "expected coil payload");
}

#[test]
fn batch_coil_json_invalid_rows_are_reported_without_hiding_valid_rows() {
    let input_file = fixture_path("fixtures/batch/coil-batch-invalid-row.json");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for partial-success coil batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 1, "expected one successful coil row");
    assert_eq!(errors.len(), 1, "expected one coil row-level error");
    assert!(results[0]["coil"].is_object(), "expected coil payload");
}

#[test]
fn batch_scrap_json_happy_path_returns_multiple_results() {
    let input_file = fixture_path("fixtures/batch/scrap-batch-happy-path.json");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for scrap batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 2, "expected two successful scrap rows");
    assert_eq!(errors.len(), 0, "expected no errors");
    assert!(results[0]["scrap"].is_object(), "expected scrap payload");
}

#[test]
fn batch_scrap_json_invalid_rows_are_reported_without_hiding_valid_rows() {
    let input_file = fixture_path("fixtures/batch/scrap-batch-invalid-row.json");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for partial-success scrap batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 1, "expected one successful scrap row");
    assert_eq!(errors.len(), 1, "expected one scrap row-level error");
    assert!(results[0]["scrap"].is_object(), "expected scrap payload");
}

#[test]
fn batch_csv_happy_path_returns_multiple_results() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-happy-path.csv");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for CSV batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 2, "expected two successful CSV rows");
    assert_eq!(errors.len(), 0, "expected no errors");
}

#[test]
fn batch_csv_invalid_rows_are_reported_without_hiding_valid_rows() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-invalid-row.csv");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for partial-success CSV batch input\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 1, "expected one successful CSV row");
    assert_eq!(errors.len(), 1, "expected one CSV row-level error");
}

#[test]
fn batch_csv_missing_required_header_is_rejected() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-missing-width-header.csv");
    let (code, _stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_ne!(code, 0, "expected non-zero exit for invalid CSV header");
    assert!(
        stderr.contains("width"),
        "expected missing width header error in stderr: {stderr}"
    );
}

#[test]
fn batch_output_file_writes_combined_csv_for_success_rows() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-happy-path.csv");
    let output_file = temp_csv_output_path("batch-success");
    let output_file_str = output_file.display().to_string();

    let (code, stdout, stderr) =
        run_cli(&["--input-file", &input_file, "--json", "--output-file", &output_file_str]);
    assert_eq!(
        code, 0,
        "expected exit 0 for batch CSV export\nstdout: {stdout}\nstderr: {stderr}"
    );

    let written = std::fs::read_to_string(&output_file).expect("expected CSV output file");
    assert!(
        written.contains("row_index,width,length,qty,gauge,table,psf,thickness,density,each_lb,total_lb,psf_result,area_ft2_each,area_ft2_total,used_key,error_message"),
        "expected CSV export header: {written}"
    );
    assert!(
        written.contains("0,48,96,1,16,HR/HRPO/CR,,,,80,80,2.5,32,32,16,"),
        "expected first success row in export: {written}"
    );
    assert!(
        written.contains("1,36,120,2,,,2.75,,,82.5,165,2.75,30,60"),
        "expected second success row in export: {written}"
    );

    let _ = std::fs::remove_file(output_file);
}

#[test]
fn batch_output_file_writes_error_message_for_failed_rows() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-invalid-row.csv");
    let output_file = temp_csv_output_path("batch-partial");
    let output_file_str = output_file.display().to_string();

    let (code, stdout, stderr) =
        run_cli(&["--input-file", &input_file, "--json", "--output-file", &output_file_str]);
    assert_eq!(
        code, 0,
        "expected exit 0 for partial-success batch CSV export\nstdout: {stdout}\nstderr: {stderr}"
    );

    let written = std::fs::read_to_string(&output_file).expect("expected CSV output file");
    assert!(
        written.contains("0,48,96,1,,,2.5,,,80,80,2.5,32,32,,"),
        "expected successful row in export: {written}"
    );
    assert!(
        written.contains("1,36,120,abc,,,2.75"),
        "expected failed row to preserve raw CSV inputs: {written}"
    );
    assert!(
        written.contains("invalid value for 'qty': 'abc'"),
        "expected failed row with error message in export: {written}"
    );

    let _ = std::fs::remove_file(output_file);
}

#[test]
fn batch_input_requires_json_flag() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-happy-path.json");
    let (code, _stdout, stderr) = run_cli(&["--input-file", &input_file]);
    assert_ne!(code, 0, "expected non-zero exit when --json is omitted");
    assert!(
        stderr.contains("--json is required"),
        "expected batch JSON requirement in stderr: {stderr}"
    );
}

#[test]
fn batch_input_cannot_be_combined_with_single_run_flags() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-happy-path.json");
    let (code, _stdout, stderr) = run_cli(&[
        "--input-file",
        &input_file,
        "--json",
        "--width",
        "48",
        "--length",
        "96",
    ]);
    assert_ne!(
        code, 0,
        "expected non-zero exit when batch mode is combined with single-run flags"
    );
    assert!(
        stderr.contains("cannot be combined"),
        "expected batch/single-run conflict error in stderr: {stderr}"
    );
}

#[test]
fn batch_row_conflicting_modes_becomes_row_error() {
    let input_file = fixture_path("fixtures/batch/sheet-batch-conflicting-modes.json");
    let (code, stdout, stderr) = run_cli(&["--input-file", &input_file, "--json"]);
    assert_eq!(
        code, 0,
        "expected exit 0 for partial-success batch with conflicting row\nstdout: {stdout}\nstderr: {stderr}"
    );

    let body: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid batch JSON");
    let results = body["results"]
        .as_array()
        .expect("batch output should contain results array");
    let errors = body["errors"]
        .as_array()
        .expect("batch output should contain errors array");

    assert_eq!(results.len(), 1, "expected one valid row to succeed");
    assert_eq!(errors.len(), 1, "expected one invalid row to fail");
    assert_eq!(errors[0]["row_index"], 1, "expected second row to fail");
    assert!(
        errors[0]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("only one of gauge, psf, or thickness"),
        "expected conflicting mode error message: {}",
        errors[0]["message"]
    );
}

// ═══════════════════════════════════════════════════════════════════
// --list-tables / --list-gauges Tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn list_tables_output() {
    let (code, stdout, _stderr) = run_cli(&["--list-tables"]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("HR/HRPO/CR"),
        "expected HR/HRPO/CR in --list-tables output: {stdout}"
    );
    assert!(
        stdout.contains("HOT ROLLED PLATE"),
        "expected HOT ROLLED PLATE in --list-tables output: {stdout}"
    );
    assert!(
        stdout.contains("STAINLESS"),
        "expected STAINLESS in --list-tables output: {stdout}"
    );
}

#[test]
fn list_gauges_output() {
    let (code, stdout, _stderr) = run_cli(&["--list-gauges", "HR/HRPO/CR"]);
    assert_eq!(code, 0, "expected exit 0");
    assert!(
        stdout.contains("16"),
        "expected gauge 16 in output: {stdout}"
    );
    assert!(
        stdout.contains("psf"),
        "expected psf values in output: {stdout}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// --table Default / Override Tests
// ═══════════════════════════════════════════════════════════════════

/// When --table is not provided, the default table (from config or HR/HRPO/CR) is used.
/// Gauge 16 in HR/HRPO/CR has psf=2.500. If default table is used correctly,
/// the calculation should succeed and return psf 2.500.
#[test]
fn no_table_flag_uses_config_default_table() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width", "48", "--length", "120", "--qty", "10", "--gauge", "16",
    ]);
    assert_eq!(code, 0, "expected exit 0 when --table is omitted");
    assert!(
        stdout.contains("psf: 2.500"),
        "expected psf=2.500 (from HR/HRPO/CR default table) in stdout: {stdout}"
    );
}

/// When --table is explicitly provided, it overrides the config default.
#[test]
fn table_flag_overrides_default() {
    let (code, stdout, _stderr) = run_cli(&[
        "--width",
        "48",
        "--length",
        "120",
        "--qty",
        "1",
        "--gauge",
        "16",
        "--table",
        "STAINLESS",
    ]);
    assert_eq!(code, 0, "expected exit 0 with explicit --table");
    // Stainless gauge 16 has psf=2.499, different from HR/HRPO/CR's 2.500.
    assert!(
        stdout.contains("psf: 2.499"),
        "expected psf=2.499 (from STAINLESS table) in stdout: {stdout}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Error Cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn error_missing_length() {
    let (code, _stdout, stderr) = run_cli(&["--width", "48"]);
    assert_ne!(code, 0, "expected non-zero exit");
    assert!(
        stderr.contains("--length is required"),
        "expected missing --length error in stderr: {stderr}"
    );
}

#[test]
fn error_conflicting_gauge_psf() {
    let (code, _stdout, stderr) = run_cli(&[
        "--width", "48", "--length", "120", "--gauge", "16", "--psf", "2.5",
    ]);
    assert_ne!(code, 0, "expected non-zero exit");
    assert!(
        stderr.contains("Cannot specify both --gauge and --psf"),
        "expected conflict error in stderr: {stderr}"
    );
}

#[test]
fn error_conflicting_gauge_thickness() {
    let (code, _stdout, stderr) = run_cli(&[
        "--width",
        "48",
        "--length",
        "120",
        "--gauge",
        "16",
        "--thickness",
        "0.1",
    ]);
    assert_ne!(code, 0, "expected non-zero exit");
    assert!(
        stderr.contains("Cannot specify both --gauge and --thickness"),
        "expected conflict error in stderr: {stderr}"
    );
}

#[test]
fn error_invalid_gauge_key() {
    let (code, _stdout, stderr) =
        run_cli(&["--width", "48", "--length", "120", "--gauge", "INVALID"]);
    assert_ne!(code, 0, "expected non-zero exit");
    assert!(
        stderr.contains("not in HR/HRPO/CR table"),
        "expected gauge lookup error in stderr: {stderr}"
    );
}

#[test]
fn error_no_args() {
    let (code, _stdout, stderr) = run_cli(&[]);
    assert_ne!(code, 0, "expected non-zero exit for no args");
    assert!(
        stderr.contains("--width is required") || stderr.contains("error"),
        "expected error in stderr: {stderr}"
    );
}

#[test]
fn error_missing_scrap_ending() {
    let (code, _stdout, stderr) = run_cli(&["--scrap-actual", "5000"]);
    assert_ne!(code, 0, "expected non-zero exit");
    assert!(
        stderr.contains("--scrap-ending is required"),
        "expected missing scrap-ending error in stderr: {stderr}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// --version Test
// ═══════════════════════════════════════════════════════════════════

#[test]
fn version_flag() {
    let (code, stdout, _stderr) = run_cli(&["--version"]);
    let expected_version = env!("CARGO_PKG_VERSION");
    assert_eq!(code, 0, "expected exit 0 for --version");
    assert!(
        stdout.contains("steelcal-cli"),
        "expected crate name in --version output: {stdout}"
    );
    assert!(
        stdout.contains(expected_version),
        "expected version number in --version output: {stdout}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Parity Fixture-Driven Tests
// ═══════════════════════════════════════════════════════════════════

/// A single test case from cli-baseline.json.
#[derive(Debug, serde::Deserialize)]
struct FixtureCase {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    args: Vec<String>,
    returncode: i32,
    expected_json: Option<serde_json::Value>,
    stderr_contains: Option<String>,
}

fn load_fixtures() -> Vec<FixtureCase> {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/parity/cli-baseline.json");
    let content = std::fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
        panic!(
            "failed to read fixture file {}: {e}",
            fixture_path.display()
        )
    });
    serde_json::from_str(&content).expect("failed to parse cli-baseline.json")
}

#[test]
fn parity_fixtures_minimum_count() {
    let fixtures = load_fixtures();
    assert!(
        fixtures.len() >= 12,
        "expected at least 12 parity fixtures, got {}",
        fixtures.len()
    );
}

#[test]
fn parity_fixture_sheet_gauge() {
    run_fixture_by_name("sheet-gauge");
}

#[test]
fn parity_fixture_sheet_psf() {
    run_fixture_by_name("sheet-psf");
}

#[test]
fn parity_fixture_sheet_thickness() {
    run_fixture_by_name("sheet-thickness");
}

#[test]
fn parity_fixture_coil() {
    run_fixture_by_name("coil");
}

#[test]
fn parity_fixture_scrap() {
    run_fixture_by_name("scrap");
}

#[test]
fn parity_fixture_pickup() {
    run_fixture_by_name("pickup");
}

#[test]
fn parity_fixture_minimum_order() {
    run_fixture_by_name("minimum-order");
}

#[test]
fn parity_fixture_invalid_input_missing_length() {
    run_fixture_by_name("invalid-input-missing-length");
}

#[test]
fn parity_fixture_batch_sheet_happy_path() {
    run_fixture_by_name("batch-sheet-happy-path");
}

#[test]
fn parity_fixture_batch_sheet_partial_success() {
    run_fixture_by_name("batch-sheet-partial-success");
}

fn run_fixture_by_name(name: &str) {
    let fixtures = load_fixtures();
    let case = fixtures
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("fixture '{name}' not found in cli-baseline.json"));
    run_fixture(case);
}

fn run_fixture(case: &FixtureCase) {
    let expanded_args: Vec<String> = case.args.iter().map(|arg| expand_fixture_arg(arg)).collect();
    let args_str: Vec<&str> = expanded_args.iter().map(|s| s.as_str()).collect();
    let (code, stdout, stderr) = run_cli(&args_str);

    assert_eq!(
        code, case.returncode,
        "fixture '{}': expected exit code {}, got {}\nstdout: {stdout}\nstderr: {stderr}",
        case.name, case.returncode, code
    );

    // If the fixture defines expected JSON, parse stdout and compare.
    if let Some(ref expected) = case.expected_json {
        let actual: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "fixture '{}': stdout is not valid JSON: {e}\nstdout: {stdout}",
                case.name
            )
        });
        compare_json_subset(expected, &actual, &case.name, "");
    }

    // If the fixture defines stderr_contains, check stderr.
    if let Some(ref pattern) = case.stderr_contains {
        assert!(
            stderr.contains(pattern),
            "fixture '{}': expected stderr to contain '{}'\nstderr: {stderr}",
            case.name,
            pattern
        );
    }
}

/// Recursively check that all keys/values in `expected` are present in `actual`.
/// Allows `actual` to have extra keys (subset comparison).
fn compare_json_subset(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    fixture_name: &str,
    path: &str,
) {
    match expected {
        serde_json::Value::Object(expected_map) => {
            let actual_map = actual.as_object().unwrap_or_else(|| {
                panic!("fixture '{fixture_name}': expected object at {path}, got {actual}")
            });
            for (key, expected_val) in expected_map {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                let actual_val = actual_map.get(key).unwrap_or_else(|| {
                    panic!("fixture '{fixture_name}': missing key '{child_path}' in actual output")
                });
                compare_json_subset(expected_val, actual_val, fixture_name, &child_path);
            }
        }
        serde_json::Value::Number(n) => {
            let expected_f = n.as_f64().unwrap();
            let actual_f = actual.as_f64().unwrap_or_else(|| {
                panic!("fixture '{fixture_name}': expected number at {path}, got {actual}")
            });
            let tolerance = if expected_f.abs() < 1.0 {
                0.001
            } else {
                expected_f.abs() * 0.0001
            };
            assert!(
                (expected_f - actual_f).abs() < tolerance,
                "fixture '{fixture_name}': at {path}, expected {expected_f}, got {actual_f} (tolerance {tolerance})"
            );
        }
        serde_json::Value::Bool(b) => {
            assert_eq!(
                actual.as_bool(),
                Some(*b),
                "fixture '{fixture_name}': at {path}, expected {b}, got {actual}"
            );
        }
        serde_json::Value::String(s) => {
            assert_eq!(
                actual.as_str(),
                Some(s.as_str()),
                "fixture '{fixture_name}': at {path}, expected \"{s}\", got {actual}"
            );
        }
        serde_json::Value::Null => {
            assert!(
                actual.is_null(),
                "fixture '{fixture_name}': at {path}, expected null, got {actual}"
            );
        }
        serde_json::Value::Array(_) => {
            let expected_items = expected.as_array().unwrap();
            let actual_items = actual.as_array().unwrap_or_else(|| {
                panic!("fixture '{fixture_name}': expected array at {path}, got {actual}")
            });

            assert_eq!(
                expected_items.len(),
                actual_items.len(),
                "fixture '{fixture_name}': at {path}, expected array length {}, got {}",
                expected_items.len(),
                actual_items.len()
            );

            for (index, expected_item) in expected_items.iter().enumerate() {
                let child_path = format!("{path}[{index}]");
                compare_json_subset(expected_item, &actual_items[index], fixture_name, &child_path);
            }
        }
    }
}
