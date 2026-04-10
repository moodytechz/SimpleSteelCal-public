use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use steelcal_core::config;
use steelcal_core::gauges::{builtin_gauge_tables, normalize_table_name};
use steelcal_core::{
    compute_coil, compute_each_total_psf, compute_scrap, CoilInputs, CoilResult, InputMode, Inputs,
    ScrapResult, SheetResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchJobType {
    Sheet,
    Coil,
    Scrap,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchSheetJob {
    pub width: Option<f64>,
    pub length: Option<f64>,
    pub qty: Option<i32>,
    pub gauge: Option<String>,
    pub table: Option<String>,
    pub psf: Option<f64>,
    pub thickness: Option<f64>,
    pub density: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchCoilJob {
    pub coil_width: Option<f64>,
    pub coil_thickness: Option<f64>,
    pub coil_id: Option<f64>,
    pub coil_weight: Option<f64>,
    pub density: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchScrapJob {
    pub actual_weight: Option<f64>,
    pub ending_weight: Option<f64>,
    pub base_cost: Option<f64>,
    pub processing_cost: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum BatchJobKind {
    Sheet(BatchSheetJob),
    Coil(BatchCoilJob),
    Scrap(BatchScrapJob),
    Empty,
}

#[derive(Debug, Clone)]
pub struct BatchJobRecord {
    pub row_index: usize,
    pub kind: BatchJobKind,
    pub parse_error: Option<String>,
    pub raw_fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct BatchFile {
    pub job_type: BatchJobType,
    pub jobs: Vec<BatchJobRecord>,
}

#[derive(Debug, Serialize)]
struct BatchSuccessRecord {
    row_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    sheet: Option<SheetResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    coil: Option<CoilResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scrap: Option<ScrapResult>,
}

#[derive(Debug, Serialize)]
struct BatchErrorRecord {
    row_index: usize,
    message: String,
}

struct ExportRow {
    row_index: usize,
    job: BatchSheetJob,
    raw_fields: BTreeMap<String, String>,
    sheet: Option<SheetResult>,
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawBatchFile {
    #[serde(default)]
    job_type: Option<BatchJobType>,
    jobs: Vec<serde_json::Value>,
}

pub fn parse_batch_jobs_from_path(input_file: &str, reader: &[u8]) -> anyhow::Result<BatchFile> {
    match Path::new(input_file)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("csv"))
    {
        Some(true) => parse_csv_batch_jobs(reader),
        _ => parse_json_batch_jobs(reader),
    }
}

fn parse_json_batch_jobs(reader: &[u8]) -> anyhow::Result<BatchFile> {
    let parsed: RawBatchFile =
        serde_json::from_slice(reader).context("failed to parse batch JSON file")?;
    let job_type = parsed.job_type.unwrap_or(BatchJobType::Sheet);
    let jobs = parsed
        .jobs
        .into_iter()
        .enumerate()
        .map(|(row_index, job)| match parse_json_job(job_type, job) {
            Ok(kind) => BatchJobRecord {
                row_index,
                kind,
                parse_error: None,
                raw_fields: BTreeMap::new(),
            },
            Err(error) => BatchJobRecord {
                row_index,
                kind: BatchJobKind::Empty,
                parse_error: Some(error.to_string()),
                raw_fields: BTreeMap::new(),
            },
        })
        .collect();
    Ok(BatchFile { job_type, jobs })
}

fn parse_json_job(job_type: BatchJobType, job: serde_json::Value) -> anyhow::Result<BatchJobKind> {
    Ok(match job_type {
        BatchJobType::Sheet => BatchJobKind::Sheet(
            serde_json::from_value(job).context("failed to parse sheet batch row")?,
        ),
        BatchJobType::Coil => BatchJobKind::Coil(
            serde_json::from_value(job).context("failed to parse coil batch row")?,
        ),
        BatchJobType::Scrap => BatchJobKind::Scrap(
            serde_json::from_value(job).context("failed to parse scrap batch row")?,
        ),
    })
}

fn parse_csv_batch_jobs(reader: &[u8]) -> anyhow::Result<BatchFile> {
    let mut csv = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);

    let headers = csv
        .headers()
        .context("failed to read CSV headers")?
        .iter()
        .map(|header| header.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();

    for required in ["width", "length"] {
        if !headers.iter().any(|header| header == required) {
            bail!("missing required CSV header '{required}'");
        }
    }

    let mut jobs = Vec::new();
    for (row_index, row) in csv.records().enumerate() {
        let record = row.with_context(|| format!("failed to read CSV row {}", row_index + 1))?;
        let raw_fields = headers
            .iter()
            .enumerate()
            .filter_map(|(index, header)| {
                record
                    .get(index)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| (header.clone(), value.to_string()))
            })
            .collect::<BTreeMap<_, _>>();
        match parse_csv_row(&headers, &record) {
            Ok(job) => jobs.push(BatchJobRecord {
                row_index,
                kind: BatchJobKind::Sheet(job),
                parse_error: None,
                raw_fields,
            }),
            Err(error) => jobs.push(BatchJobRecord {
                row_index,
                kind: BatchJobKind::Empty,
                parse_error: Some(error.to_string()),
                raw_fields,
            }),
        }
    }

    Ok(BatchFile {
        job_type: BatchJobType::Sheet,
        jobs,
    })
}

fn parse_csv_row(headers: &[String], record: &csv::StringRecord) -> anyhow::Result<BatchSheetJob> {
    fn cell<'a>(headers: &[String], record: &'a csv::StringRecord, name: &str) -> Option<&'a str> {
        headers
            .iter()
            .position(|header| header == name)
            .and_then(|index| record.get(index))
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn parse_optional_f64(
        headers: &[String],
        record: &csv::StringRecord,
        name: &str,
    ) -> anyhow::Result<Option<f64>> {
        match cell(headers, record, name) {
            Some(value) => value
                .parse::<f64>()
                .map(Some)
                .with_context(|| format!("invalid value for '{name}': '{value}'")),
            None => Ok(None),
        }
    }

    fn parse_optional_i32(
        headers: &[String],
        record: &csv::StringRecord,
        name: &str,
    ) -> anyhow::Result<Option<i32>> {
        match cell(headers, record, name) {
            Some(value) => value
                .parse::<i32>()
                .map(Some)
                .with_context(|| format!("invalid value for '{name}': '{value}'")),
            None => Ok(None),
        }
    }

    Ok(BatchSheetJob {
        width: parse_optional_f64(headers, record, "width")?,
        length: parse_optional_f64(headers, record, "length")?,
        qty: parse_optional_i32(headers, record, "qty")?,
        gauge: cell(headers, record, "gauge").map(ToString::to_string),
        table: cell(headers, record, "table").map(ToString::to_string),
        psf: parse_optional_f64(headers, record, "psf")?,
        thickness: parse_optional_f64(headers, record, "thickness")?,
        density: parse_optional_f64(headers, record, "density")?,
    })
}

pub fn run_batch(input_file: &str, output_file: Option<&str>, json: bool) -> anyhow::Result<()> {
    if !json {
        bail!("--json is required when using --input-file");
    }

    let bytes = fs::read(input_file)
        .with_context(|| format!("failed to read batch input file '{input_file}'"))?;
    let batch = parse_batch_jobs_from_path(input_file, &bytes)?;

    if output_file.is_some() && batch.job_type != BatchJobType::Sheet {
        bail!("--output-file is currently supported only for sheet batch files");
    }

    let tables = builtin_gauge_tables();
    let config_map = config::config_path()
        .ok()
        .and_then(|path| config::load_normalized_config(&path, &tables).ok())
        .unwrap_or_default();
    let effective_config = config::effective_config(&config_map, &tables);

    let mut results = Vec::new();
    let mut errors = Vec::new();
    let mut export_rows = Vec::new();

    for record in batch.jobs {
        let export_job = match &record.kind {
            BatchJobKind::Sheet(job) => job.clone(),
            _ => BatchSheetJob {
                width: None,
                length: None,
                qty: None,
                gauge: None,
                table: None,
                psf: None,
                thickness: None,
                density: None,
            },
        };

        match record.parse_error.clone().map_or_else(
            || {
                run_batch_record(
                    &record.kind,
                    &tables,
                    &effective_config.default_table,
                    &effective_config.default_gauge,
                )
            },
            |error| Err(anyhow!(error)),
        ) {
            Ok(mut success) => {
                success.row_index = record.row_index;
                if batch.job_type == BatchJobType::Sheet {
                    export_rows.push(ExportRow {
                        row_index: record.row_index,
                        job: export_job,
                        raw_fields: record.raw_fields.clone(),
                        sheet: success.sheet.clone(),
                        error_message: None,
                    });
                }
                results.push(success);
            }
            Err(error) => {
                let message = error.to_string();
                if batch.job_type == BatchJobType::Sheet {
                    export_rows.push(ExportRow {
                        row_index: record.row_index,
                        job: export_job,
                        raw_fields: record.raw_fields.clone(),
                        sheet: None,
                        error_message: Some(message.clone()),
                    });
                }
                errors.push(BatchErrorRecord {
                    row_index: record.row_index,
                    message,
                });
            }
        }
    }

    if let Some(output_file) = output_file {
        write_batch_csv(output_file, &export_rows)?;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "results": results,
            "errors": errors,
        }))
        .context("failed to serialize batch results")?
    );

    Ok(())
}

fn run_batch_record(
    kind: &BatchJobKind,
    tables: &steelcal_core::gauges::GaugeTables,
    default_table: &str,
    default_gauge: &str,
) -> anyhow::Result<BatchSuccessRecord> {
    match kind {
        BatchJobKind::Sheet(job) => {
            let inputs = build_sheet_inputs(job, default_table, default_gauge)?;
            let sheet = compute_each_total_psf(&inputs, tables).map_err(|error| anyhow!(error))?;
            Ok(BatchSuccessRecord {
                row_index: 0,
                sheet: Some(sheet),
                coil: None,
                scrap: None,
            })
        }
        BatchJobKind::Coil(job) => {
            let inputs = build_coil_inputs(job)?;
            let coil = compute_coil(&inputs).map_err(|error| anyhow!(error))?;
            Ok(BatchSuccessRecord {
                row_index: 0,
                sheet: None,
                coil: Some(coil),
                scrap: None,
            })
        }
        BatchJobKind::Scrap(job) => {
            let scrap = build_scrap_result(job)?;
            Ok(BatchSuccessRecord {
                row_index: 0,
                sheet: None,
                coil: None,
                scrap: Some(scrap),
            })
        }
        BatchJobKind::Empty => Err(anyhow!("batch row was not parsed")),
    }
}

fn write_batch_csv(path: &str, rows: &[ExportRow]) -> anyhow::Result<()> {
    let mut writer = csv::Writer::from_path(path)
        .with_context(|| format!("failed to create batch CSV output file '{path}'"))?;

    writer.write_record([
        "row_index",
        "width",
        "length",
        "qty",
        "gauge",
        "table",
        "psf",
        "thickness",
        "density",
        "each_lb",
        "total_lb",
        "psf_result",
        "area_ft2_each",
        "area_ft2_total",
        "used_key",
        "error_message",
    ])?;

    for row in rows {
        writer.write_record([
            row.row_index.to_string(),
            opt_num_or_raw(row.job.width, &row.raw_fields, "width"),
            opt_num_or_raw(row.job.length, &row.raw_fields, "length"),
            opt_i32_or_raw(row.job.qty, &row.raw_fields, "qty"),
            opt_string_or_raw(row.job.gauge.as_ref(), &row.raw_fields, "gauge"),
            opt_string_or_raw(row.job.table.as_ref(), &row.raw_fields, "table"),
            opt_num_or_raw(row.job.psf, &row.raw_fields, "psf"),
            opt_num_or_raw(row.job.thickness, &row.raw_fields, "thickness"),
            opt_num_or_raw(row.job.density, &row.raw_fields, "density"),
            row.sheet
                .as_ref()
                .map(|sheet| sheet.each_lb.to_string())
                .unwrap_or_default(),
            row.sheet
                .as_ref()
                .map(|sheet| sheet.total_lb.to_string())
                .unwrap_or_default(),
            row.sheet
                .as_ref()
                .map(|sheet| sheet.psf.to_string())
                .unwrap_or_default(),
            row.sheet
                .as_ref()
                .map(|sheet| sheet.area_ft2_each.to_string())
                .unwrap_or_default(),
            row.sheet
                .as_ref()
                .map(|sheet| sheet.area_ft2_total.to_string())
                .unwrap_or_default(),
            row.sheet
                .as_ref()
                .and_then(|sheet| sheet.used_key.clone())
                .unwrap_or_default(),
            row.error_message.clone().unwrap_or_default(),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

fn opt_num_or_raw(value: Option<f64>, raw_fields: &BTreeMap<String, String>, key: &str) -> String {
    value
        .map(|v| v.to_string())
        .or_else(|| raw_fields.get(key).cloned())
        .unwrap_or_default()
}

fn opt_i32_or_raw(value: Option<i32>, raw_fields: &BTreeMap<String, String>, key: &str) -> String {
    value
        .map(|v| v.to_string())
        .or_else(|| raw_fields.get(key).cloned())
        .unwrap_or_default()
}

fn opt_string_or_raw(
    value: Option<&String>,
    raw_fields: &BTreeMap<String, String>,
    key: &str,
) -> String {
    value
        .cloned()
        .or_else(|| raw_fields.get(key).cloned())
        .unwrap_or_default()
}

fn build_sheet_inputs(
    job: &BatchSheetJob,
    default_table: &str,
    default_gauge: &str,
) -> anyhow::Result<Inputs> {
    let width = job.width.ok_or_else(|| anyhow!("width is required"))?;
    let length = job.length.ok_or_else(|| anyhow!("length is required"))?;

    let mode_count = usize::from(job.psf.is_some())
        + usize::from(job.gauge.is_some())
        + usize::from(job.thickness.is_some());
    if mode_count > 1 {
        bail!("only one of gauge, psf, or thickness may be set per batch row");
    }

    let table = normalize_table_name(job.table.as_deref().unwrap_or(default_table));
    let mode = if let Some(psf) = job.psf {
        InputMode::Psf(psf)
    } else if let Some(gauge) = job.gauge.clone() {
        InputMode::Gauge { table, key: gauge }
    } else if let Some(thickness) = job.thickness {
        InputMode::Thickness(thickness)
    } else {
        InputMode::Gauge {
            table,
            key: default_gauge.to_string(),
        }
    };

    Ok(Inputs {
        width_in: width,
        length_in: length,
        qty: job.qty.unwrap_or(1),
        mode,
        density_lb_ft3: job
            .density
            .unwrap_or(steelcal_core::DENSITY_LB_PER_FT3_DEFAULT),
    })
}

fn build_coil_inputs(job: &BatchCoilJob) -> anyhow::Result<CoilInputs> {
    Ok(CoilInputs {
        coil_width_in: job
            .coil_width
            .ok_or_else(|| anyhow!("coil_width is required"))?,
        coil_thickness_in: job
            .coil_thickness
            .ok_or_else(|| anyhow!("coil_thickness is required"))?,
        coil_id_in: job.coil_id.ok_or_else(|| anyhow!("coil_id is required"))?,
        coil_weight_lb: job
            .coil_weight
            .ok_or_else(|| anyhow!("coil_weight is required"))?,
        density_lb_ft3: job
            .density
            .unwrap_or(steelcal_core::DENSITY_LB_PER_FT3_DEFAULT),
    })
}

fn build_scrap_result(job: &BatchScrapJob) -> anyhow::Result<ScrapResult> {
    compute_scrap(
        job.actual_weight
            .ok_or_else(|| anyhow!("actual_weight is required"))?,
        job.ending_weight
            .ok_or_else(|| anyhow!("ending_weight is required"))?,
        job.base_cost
            .ok_or_else(|| anyhow!("base_cost is required"))?,
        job.processing_cost
            .ok_or_else(|| anyhow!("processing_cost is required"))?,
    )
    .map_err(|error| anyhow!(error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_batch_jobs_preserves_row_order() {
        let input = br#"{
            "jobs": [
                {"width": 48.0, "length": 96.0, "gauge": "16"},
                {"width": 36.0, "length": 96.0, "psf": 2.5}
            ]
        }"#;

        let parsed = parse_batch_jobs_from_path("jobs.json", input).unwrap();
        assert_eq!(parsed.jobs.len(), 2);
        assert_eq!(parsed.jobs[0].row_index, 0);
        assert_eq!(parsed.jobs[1].row_index, 1);
        assert!(parsed.jobs[0].parse_error.is_none());
        assert!(parsed.jobs[1].parse_error.is_none());
        assert!(matches!(parsed.jobs[0].kind, BatchJobKind::Sheet(_)));
        assert!(matches!(parsed.jobs[1].kind, BatchJobKind::Sheet(_)));
    }

    #[test]
    fn parse_batch_jobs_csv_preserves_row_order() {
        let input = b"width,length,qty,gauge\n48,96,1,16\n36,120,2,14\n";

        let parsed = parse_batch_jobs_from_path("jobs.csv", input).unwrap();
        assert_eq!(parsed.jobs.len(), 2);
        assert_eq!(parsed.jobs[0].row_index, 0);
        assert_eq!(parsed.jobs[1].row_index, 1);
        match &parsed.jobs[0].kind {
            BatchJobKind::Sheet(job) => assert_eq!(job.gauge.as_deref(), Some("16")),
            _ => panic!("expected sheet job"),
        }
        match &parsed.jobs[1].kind {
            BatchJobKind::Sheet(job) => assert_eq!(job.gauge.as_deref(), Some("14")),
            _ => panic!("expected sheet job"),
        }
        assert!(parsed.jobs[0].parse_error.is_none());
        assert!(parsed.jobs[1].parse_error.is_none());
    }
}
