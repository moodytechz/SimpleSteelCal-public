mod batch;

use std::collections::BTreeMap;
use std::io::Write;
use std::process::ExitCode;

use anyhow::{bail, Context};
use clap::Parser;
use steelcal_core::config;
use steelcal_core::gauges::{builtin_gauge_tables, normalize_table_name};
use steelcal_core::{
    compute_coil, compute_costs, compute_each_total_psf, compute_scrap, CoilInputs, CostInputs,
    InputMode, Inputs, PriceMode,
};

#[cfg(feature = "selftest")]
use steelcal_core::run_self_tests;

#[derive(Debug, Parser)]
#[command(about = "Steel Sheet Weight Calculator (Imperial) + Costing", version)]
struct Args {
    /// Sheet width in inches (required for sheet calculations).
    #[arg(long, help = "Sheet width in inches (required for sheet calculations)")]
    width: Option<f64>,

    /// Sheet length in inches (required for sheet calculations).
    #[arg(
        long,
        help = "Sheet length in inches (required for sheet calculations)"
    )]
    length: Option<f64>,

    /// Quantity of sheets (must be >= 0, default: 1).
    #[arg(
        long,
        default_value_t = 1,
        value_parser = clap::value_parser!(i32).range(0..),
        allow_hyphen_values = true,
        help = "Quantity of sheets (must be >= 0, default: 1)"
    )]
    qty: i32,

    /// PSF (lb/ft²) value; use instead of --gauge or --thickness for direct input.
    #[arg(
        long,
        help = "PSF (lb/ft²) value; use instead of --gauge or --thickness"
    )]
    psf: Option<f64>,

    /// Gauge key to look up in the gauge table (e.g. '16', '1/4'). Mutually exclusive with --psf and --thickness.
    #[arg(
        long,
        help = "Gauge key to look up in the gauge table (e.g. '16', '1/4')"
    )]
    gauge: Option<String>,

    /// Gauge table name to use for --gauge lookups (default: from config, or HR/HRPO/CR).
    #[arg(
        long,
        help = "Gauge table name for --gauge lookups (default: from config, or HR/HRPO/CR)"
    )]
    table: Option<String>,

    /// Raw material thickness in inches; derives PSF via density. Mutually exclusive with --gauge and --psf.
    #[arg(
        long,
        help = "Raw material thickness in inches; derives PSF via density"
    )]
    thickness: Option<f64>,

    /// Steel density in lb/ft³ (default: 490.0 or from config).
    #[arg(long, help = "Steel density in lb/ft³ (default: 490.0 or from config)")]
    density: Option<f64>,

    /// Coil inner diameter in inches; required for OD calculation.
    #[arg(
        long = "coil-id",
        help = "Coil inner diameter in inches (for OD calculation)"
    )]
    coil_id: Option<f64>,

    /// Coil strip width in inches (defaults to --width value).
    #[arg(
        long = "coil-width",
        help = "Coil strip width in inches (defaults to --width)"
    )]
    coil_width: Option<f64>,

    /// Coil material thickness in inches (defaults to thickness derived from PSF).
    #[arg(
        long = "coil-thickness",
        help = "Coil material thickness in inches (derived from PSF if omitted)"
    )]
    coil_thickness: Option<f64>,

    /// Coil total weight in pounds; triggers weight-based footage calculation.
    #[arg(
        long = "coil-weight",
        help = "Coil total weight in lb; triggers weight-based footage calculation"
    )]
    coil_weight: Option<f64>,

    /// Actual starting weight of the coil in pounds (for scrap calculation).
    #[arg(
        long = "scrap-actual",
        help = "Actual starting weight in lb (for scrap calculation)"
    )]
    scrap_actual: Option<f64>,

    /// Ending (usable) weight of the coil in pounds (for scrap calculation).
    #[arg(
        long = "scrap-ending",
        help = "Ending (usable) weight in lb (for scrap calculation)"
    )]
    scrap_ending: Option<f64>,

    /// Base cost per pound for scrap calculation.
    #[arg(
        long = "scrap-base-cost",
        help = "Base cost per pound ($/lb) for scrap calculation"
    )]
    scrap_base_cost: Option<f64>,

    /// Processing cost per pound for scrap calculation.
    #[arg(
        long = "scrap-processing-cost",
        help = "Processing cost per pound ($/lb) for scrap calculation"
    )]
    scrap_processing_cost: Option<f64>,

    /// Pricing mode: per-lb, per-ft2, or per-sheet.
    #[arg(
        long = "price-mode",
        value_enum,
        default_value_t = PriceMode::PerLb,
        help = "Pricing mode: per-lb, per-ft2, or per-sheet"
    )]
    price_mode: PriceMode,

    /// Unit price in dollars for the selected --price-mode.
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Unit price in dollars for the selected --price-mode"
    )]
    price: f64,

    /// Markup percentage applied to base price (e.g. 10 for 10%).
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Markup percentage applied to base price (e.g. 10 for 10%)"
    )]
    markup: f64,

    /// Tax percentage applied after markup (e.g. 8.25 for 8.25%).
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Tax percentage applied after markup (e.g. 8.25 for 8.25%)"
    )]
    tax: f64,

    /// One-time setup/delivery fee added to the total (in dollars).
    #[arg(
        long = "setup-fee",
        default_value_t = 0.0,
        help = "One-time setup/delivery fee added to the total (in dollars)"
    )]
    setup_fee: f64,

    /// Minimum order total in dollars; applied if subtotal is lower.
    #[arg(
        long = "min-order",
        default_value_t = 0.0,
        help = "Minimum order total in dollars; applied if subtotal is lower"
    )]
    min_order: f64,

    /// Run built-in self-tests and exit.
    #[cfg(feature = "selftest")]
    #[arg(long, help = "Run built-in self-tests and exit")]
    selftest: bool,

    /// Output all calculation results as JSON to stdout.
    #[arg(long, help = "Output all calculation results as JSON to stdout")]
    json: bool,

    /// Read a batch job file (JSON sheet jobs in iteration 1).
    #[arg(long, help = "Read a batch job file (JSON sheet jobs in iteration 1)")]
    input_file: Option<String>,

    /// Write batch results to a CSV file while keeping JSON on stdout.
    #[arg(long, help = "Write batch results to a CSV file while keeping JSON on stdout")]
    output_file: Option<String>,

    /// List all available gauge table names and exit.
    #[arg(
        long = "list-tables",
        help = "List all available gauge table names and exit"
    )]
    list_tables: bool,

    /// List all gauge keys and PSF values for the specified table and exit.
    #[arg(
        long = "list-gauges",
        help = "List all gauge keys and PSF values for the specified table and exit"
    )]
    list_gauges: Option<String>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(args) {
        Ok(()) => {
            // Flush stdout to ensure all output is written before exit.
            let _ = std::io::stdout().flush();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> anyhow::Result<()> {
    #[cfg(feature = "selftest")]
    if args.selftest {
        run_self_tests().context("self-tests failed")?;
        println!("Self-tests passed.");
        return Ok(());
    }

    let tables = builtin_gauge_tables();

    // --- Discovery: --list-tables ---
    if args.list_tables {
        println!("Available gauge tables:");
        for name in tables.keys() {
            println!("  {name}");
        }
        return Ok(());
    }

    // --- Discovery: --list-gauges TABLE ---
    if let Some(ref table_arg) = args.list_gauges {
        let canonical = normalize_table_name(table_arg);
        let table = tables.get(&canonical).ok_or_else(|| {
            let valid = tables.keys().cloned().collect::<Vec<_>>().join(", ");
            anyhow::anyhow!("unknown table '{table_arg}'. Choose one of: {valid}")
        })?;
        println!("Gauges for '{canonical}':");
        for entry in &table.entries {
            println!("  {:<10} {:.4} psf", entry.key, entry.psf);
        }
        return Ok(());
    }

    if let Some(ref input_file) = args.input_file {
        if args.width.is_some()
            || args.length.is_some()
            || args.psf.is_some()
            || args.gauge.is_some()
            || args.table.is_some()
            || args.thickness.is_some()
            || args.density.is_some()
            || args.coil_id.is_some()
            || args.coil_width.is_some()
            || args.coil_thickness.is_some()
            || args.coil_weight.is_some()
            || args.scrap_actual.is_some()
            || args.scrap_ending.is_some()
            || args.scrap_base_cost.is_some()
            || args.scrap_processing_cost.is_some()
            || args.price != 0.0
            || args.markup != 0.0
            || args.tax != 0.0
            || args.setup_fee != 0.0
            || args.min_order != 0.0
            || args.qty != 1
        {
            bail!("--input-file cannot be combined with single-run calculation flags");
        }

        return batch::run_batch(input_file, args.output_file.as_deref(), args.json);
    }

    let config_map = config::config_path()
        .ok()
        .and_then(|path| config::load_normalized_config(&path, &tables).ok())
        .unwrap_or_default();
    let effective_config = config::effective_config(&config_map, &tables);

    let density = args.density.unwrap_or(effective_config.density_lb_ft3);

    // Resolve table: --table flag overrides config's default_table, which itself
    // falls back to DEFAULT_TABLE_NAME.
    let resolved_table = args
        .table
        .clone()
        .unwrap_or_else(|| effective_config.default_table.clone());

    // --- Conflict detection: --gauge + --psf ---
    if args.gauge.is_some() && args.psf.is_some() {
        bail!("Cannot specify both --gauge and --psf. Use one or the other.");
    }
    if args.gauge.is_some() && args.thickness.is_some() {
        bail!("Cannot specify both --gauge and --thickness. Use one or the other.");
    }
    if args.psf.is_some() && args.thickness.is_some() {
        bail!("Cannot specify both --psf and --thickness. Use one or the other.");
    }

    let has_scrap_args = args.scrap_actual.is_some()
        || args.scrap_ending.is_some()
        || args.scrap_base_cost.is_some()
        || args.scrap_processing_cost.is_some();

    let has_coil_args = args.coil_weight.is_some()
        || args.coil_id.is_some()
        || args.coil_width.is_some()
        || args.coil_thickness.is_some();

    let has_sheet_args = args.width.is_some() || args.length.is_some();

    // If no sheet args and no scrap args and no coil args, require sheet args.
    if !has_sheet_args && !has_scrap_args && !has_coil_args {
        bail!("--width is required for sheet calculations");
    }

    // --- Sheet calculation (only when sheet args are provided) ---
    let sheet_result = if has_sheet_args {
        let width = args
            .width
            .ok_or_else(|| anyhow::anyhow!("--width is required for sheet calculations"))?;
        let length = args.length.ok_or_else(|| {
            if args.width.is_some() {
                anyhow::anyhow!("--length is required when --width is provided")
            } else {
                anyhow::anyhow!("--length is required for sheet calculations")
            }
        })?;

        let table_choice = normalize_table_name(&resolved_table);
        if !tables.contains_key(&table_choice) {
            let valid = tables.keys().cloned().collect::<Vec<_>>().join(", ");
            bail!("unknown table '{}'. Choose one of: {valid}", resolved_table);
        }

        // Determine InputMode from flags
        let mode = if let Some(psf) = args.psf {
            InputMode::Psf(psf)
        } else if let Some(gauge_key) = args.gauge.clone() {
            InputMode::Gauge {
                table: table_choice.clone(),
                key: gauge_key,
            }
        } else if let Some(thickness) = args.thickness {
            InputMode::Thickness(thickness)
        } else {
            // Default to gauge mode with effective config gauge
            InputMode::Gauge {
                table: table_choice.clone(),
                key: effective_config.default_gauge.clone(),
            }
        };

        let data = Inputs {
            width_in: width,
            length_in: length,
            qty: args.qty,
            mode: mode.clone(),
            density_lb_ft3: density,
        };

        let result =
            compute_each_total_psf(&data, &tables).context("sheet weight calculation failed")?;

        if !args.json {
            println!(
                "Each (lb): {:.3}\nTotal (lb): {:.3}\npsf: {:.3}",
                result.each_lb, result.total_lb, result.psf
            );

            if let InputMode::Gauge { ref table, .. } = mode {
                if *table != resolved_table {
                    println!("(normalized table -> '{table}')");
                }
            }
            if let (Some(requested), Some(used_key)) =
                (args.gauge.as_deref(), result.used_key.as_deref())
            {
                if requested != used_key {
                    println!("(normalized gauge key -> '{used_key}')");
                }
            }
        }

        Some((data, result))
    } else {
        None
    };

    // --- Coil calculation (only when coil-related args are provided) ---
    let coil_result = if has_coil_args {
        let coil_thickness = if let Some(ct) = args.coil_thickness {
            ct
        } else if let Some((_, ref sr)) = sheet_result {
            sr.psf * 12.0 / density
        } else {
            bail!("--coil-thickness is required when no sheet calculation is performed");
        };

        let coil_width = if let Some(cw) = args.coil_width {
            cw
        } else if let Some((ref sd, _)) = sheet_result {
            sd.width_in
        } else {
            bail!("--coil-width is required when no sheet calculation is performed");
        };

        let coil_weight = args.coil_weight.unwrap_or(0.0);

        let coil_inputs = CoilInputs {
            coil_width_in: coil_width,
            coil_thickness_in: coil_thickness,
            coil_id_in: args.coil_id.unwrap_or(0.0),
            coil_weight_lb: coil_weight,
            density_lb_ft3: density,
        };

        let result = compute_coil(&coil_inputs).context("coil calculation failed")?;
        if !args.json {
            println!("Linear ft: {:.3}", result.coil_footage_ft);
            println!("PIW (lb/in): {:.3}", result.coil_piw_lb_per_in);
            if let Some(od) = result.coil_od_in {
                println!("Coil OD (in): {:.3}", od);
            }
        }
        Some(result)
    } else {
        None
    };

    // --- Scrap calculation (only when scrap-related args are provided) ---
    let scrap_result = if has_scrap_args {
        let scrap_actual = args
            .scrap_actual
            .ok_or_else(|| anyhow::anyhow!("--scrap-actual is required for scrap calculations"))?;
        let scrap_ending = args
            .scrap_ending
            .ok_or_else(|| anyhow::anyhow!("--scrap-ending is required for scrap calculations"))?;
        let scrap_base_cost = args.scrap_base_cost.ok_or_else(|| {
            anyhow::anyhow!("--scrap-base-cost is required for scrap calculations")
        })?;
        let scrap_processing_cost = args.scrap_processing_cost.ok_or_else(|| {
            anyhow::anyhow!("--scrap-processing-cost is required for scrap calculations")
        })?;

        let result = compute_scrap(
            scrap_actual,
            scrap_ending,
            scrap_base_cost,
            scrap_processing_cost,
        )
        .context("scrap calculation failed")?;

        if !args.json {
            println!("Scrap (lb): {:.3}", result.scrap_lb);
            println!("Total cost: ${:.2}", result.total_cost);
            println!("Price per lb: ${:.4}", result.price_per_lb);
            println!("Scrap charge per lb: ${:.4}", result.scrap_charge_per_lb);
            println!("Pickup: {}", if result.is_pickup { "Yes" } else { "No" });
        }
        Some(result)
    } else {
        None
    };

    // --- Cost calculation (only when sheet results are available) ---
    let cost_result = if let Some((ref data, ref result)) = sheet_result {
        let quote = compute_costs(
            &CostInputs {
                mode: args.price_mode,
                price_value: args.price,
                markup_pct: args.markup,
                tax_pct: args.tax,
                setup_fee: args.setup_fee,
                minimum_order: args.min_order,
            },
            data.qty,
            result.each_lb,
            result.area_ft2_each,
        )
        .context("cost calculation failed")?;

        if !args.json {
            println!(
                "Each before tax: $ {}\nEach after tax:  $ {}\nTotal before tax: $ {}\nTotal after tax:  $ {}",
                format_currency(quote.each_before_tax),
                format_currency(quote.each_after_tax),
                format_currency(quote.total_before_tax),
                format_currency(quote.total_after_tax),
            );
            if quote.minimum_applied {
                println!("(minimum order applied)");
            }
        }
        Some(quote)
    } else {
        None
    };

    // --- JSON output mode ---
    if args.json {
        let mut json_output = BTreeMap::new();
        if let Some((_, ref sr)) = sheet_result {
            json_output.insert(
                "sheet",
                serde_json::to_value(sr).context("failed to serialize sheet result")?,
            );
        }
        if let Some(ref cr) = coil_result {
            json_output.insert(
                "coil",
                serde_json::to_value(cr).context("failed to serialize coil result")?,
            );
        }
        if let Some(ref scr) = scrap_result {
            json_output.insert(
                "scrap",
                serde_json::to_value(scr).context("failed to serialize scrap result")?,
            );
        }
        if let Some(ref cost) = cost_result {
            json_output.insert(
                "costs",
                serde_json::to_value(cost).context("failed to serialize cost result")?,
            );
        }
        let json_str = serde_json::to_string_pretty(&json_output)
            .context("failed to serialize results to JSON")?;
        println!("{json_str}");
    }

    Ok(())
}

fn format_currency(value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let raw = format!("{:.2}", value.abs());
    let mut parts = raw.split('.');
    let whole = parts.next().unwrap_or("0");
    let fraction = parts.next().unwrap_or("00");

    let mut grouped_reversed = String::new();
    for (index, ch) in whole.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped_reversed.push(',');
        }
        grouped_reversed.push(ch);
    }

    let grouped = grouped_reversed.chars().rev().collect::<String>();
    format!("{sign}{grouped}.{fraction}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- format_currency tests ----

    #[test]
    fn format_currency_zero() {
        assert_eq!(format_currency(0.0), "0.00");
    }

    #[test]
    fn format_currency_small() {
        assert_eq!(format_currency(1.5), "1.50");
    }

    #[test]
    fn format_currency_thousands() {
        assert_eq!(format_currency(1234.56), "1,234.56");
    }

    #[test]
    fn format_currency_millions() {
        assert_eq!(format_currency(1234567.89), "1,234,567.89");
    }

    #[test]
    fn format_currency_negative() {
        assert_eq!(format_currency(-42.10), "-42.10");
    }

    #[test]
    fn format_currency_large_negative() {
        assert_eq!(format_currency(-1234567.89), "-1,234,567.89");
    }

    #[test]
    fn format_currency_negative_small() {
        assert_eq!(format_currency(-1234.56), "-1,234.56");
    }

    #[test]
    fn format_currency_negative_zero() {
        // -0.0 should format as "0.00" (no sign for negative zero)
        assert_eq!(format_currency(-0.0), "0.00");
    }

    #[test]
    fn format_currency_large_number() {
        assert_eq!(format_currency(999_999_999.99), "999,999,999.99");
    }

    #[test]
    fn format_currency_hundreds() {
        assert_eq!(format_currency(999.99), "999.99");
    }

    #[test]
    fn format_currency_exact_thousands() {
        assert_eq!(format_currency(1000.00), "1,000.00");
    }

    #[test]
    fn format_currency_very_small() {
        assert_eq!(format_currency(0.01), "0.01");
    }

    #[test]
    fn format_currency_rounds_to_two_decimal_places() {
        // format! rounds the value to 2 decimal places
        assert_eq!(format_currency(1.999), "2.00");
    }

    #[test]
    fn format_currency_single_digit() {
        assert_eq!(format_currency(5.0), "5.00");
    }

    #[test]
    fn format_currency_billions() {
        assert_eq!(format_currency(1_234_567_890.12), "1,234,567,890.12");
    }

    // ---- table option resolution tests ----

    /// When --table is not provided, effective_config.default_table is used.
    #[test]
    fn table_none_uses_config_default_table() {
        use steelcal_core::config;
        use steelcal_core::gauges::builtin_gauge_tables;

        let tables = builtin_gauge_tables();
        // Build a config map that sets default_table to "STAINLESS"
        let config_map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "default_table": "STAINLESS"
            }))
            .unwrap();
        let effective = config::effective_config(&config_map, &tables);

        // Simulate args.table being None (--table not provided on CLI)
        let args_table: Option<String> = None;
        let resolved = args_table.unwrap_or_else(|| effective.default_table.clone());
        assert_eq!(resolved, "STAINLESS");
    }

    /// When --table is provided, it overrides the config's default_table.
    #[test]
    fn table_some_overrides_config_default_table() {
        use steelcal_core::config;
        use steelcal_core::gauges::builtin_gauge_tables;

        let tables = builtin_gauge_tables();
        let config_map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "default_table": "STAINLESS"
            }))
            .unwrap();
        let effective = config::effective_config(&config_map, &tables);

        // Simulate args.table being Some (--table provided on CLI)
        let args_table: Option<String> = Some("GALV/JK/BOND".to_string());
        let resolved = args_table.unwrap_or_else(|| effective.default_table.clone());
        assert_eq!(resolved, "GALV/JK/BOND");
    }

    /// When --table is not provided and no config default_table is set,
    /// DEFAULT_TABLE_NAME is used via effective_config's fallback.
    #[test]
    fn table_none_no_config_uses_builtin_default() {
        use steelcal_core::config;
        use steelcal_core::gauges::{builtin_gauge_tables, DEFAULT_TABLE_NAME};

        let tables = builtin_gauge_tables();
        // Empty config map - no default_table set
        let config_map = serde_json::Map::new();
        let effective = config::effective_config(&config_map, &tables);

        let args_table: Option<String> = None;
        let resolved = args_table.unwrap_or_else(|| effective.default_table.clone());
        assert_eq!(resolved, DEFAULT_TABLE_NAME);
    }
}
