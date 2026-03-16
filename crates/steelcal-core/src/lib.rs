pub mod config;
pub mod errors;
pub mod gauges;
pub mod history;

use std::f64::consts::PI;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::errors::SteelCalError;
use crate::gauges::{get_psf, normalize_table_name, GaugeTables};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_COPYRIGHT: &str = "Copyright (c) Harbor Pipe & Steel Inc.";

pub const APP_TITLE: &str = "SIMPLE STEEL CALCULATOR";
pub const APP_DATA_DIRNAME: &str = "SimpleSteelCalculator";

pub const CONFIG_FILENAME: &str = "steel_calc_config.json";
pub const HISTORY_FILENAME: &str = "history.log";

pub const DENSITY_LB_PER_FT3_DEFAULT: f64 = 490.0;
pub const UI_FONT_SIZE_DEFAULT: i64 = 12;
pub const UI_HEADING_DELTA_DEFAULT: i64 = 3;
pub const UI_TK_SCALING_DEFAULT: f64 = 0.0;

// ---------------------------------------------------------------------------
// InputMode — replaces use_psf / use_gauge booleans
// ---------------------------------------------------------------------------

/// Determines how the PSF (lb/ft²) value is obtained for a sheet calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputMode {
    /// User provides PSF directly.
    Psf(f64),
    /// Lookup PSF from a gauge table.
    Gauge { table: String, key: String },
    /// Derive PSF from a raw thickness (inches).
    Thickness(f64),
}

// ---------------------------------------------------------------------------
// PriceMode — replaces mode: String
// ---------------------------------------------------------------------------

/// Pricing calculation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum PriceMode {
    /// Price per pound.
    #[value(name = "per-lb")]
    PerLb,
    /// Price per square foot.
    #[value(name = "per-ft2")]
    PerFt2,
    /// Price per sheet.
    #[value(name = "per-sheet")]
    PerSheet,
}

impl fmt::Display for PriceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PerLb => write!(f, "per lb"),
            Self::PerFt2 => write!(f, "per ft²"),
            Self::PerSheet => write!(f, "per sheet"),
        }
    }
}

// ---------------------------------------------------------------------------
// Inputs / results
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inputs {
    pub width_in: f64,
    pub length_in: f64,
    pub qty: i32,
    pub mode: InputMode,
    pub density_lb_ft3: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetResult {
    pub each_lb: f64,
    pub total_lb: f64,
    pub psf: f64,
    pub used_key: Option<String>,
    pub area_ft2_each: f64,
    pub area_ft2_total: f64,
}

impl fmt::Display for SheetResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Each (lb): {:.3}\nTotal (lb): {:.3}\nPSF (lb/ft²): {:.4}\nArea each (ft²): {:.4}\nArea total (ft²): {:.4}",
            self.each_lb, self.total_lb, self.psf, self.area_ft2_each, self.area_ft2_total
        )?;
        if let Some(key) = &self.used_key {
            write!(f, "\nUsed key: {key}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostInputs {
    pub mode: PriceMode,
    pub price_value: f64,
    pub markup_pct: f64,
    pub tax_pct: f64,
    pub setup_fee: f64,
    pub minimum_order: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostOutputs {
    pub each_before_tax: f64,
    pub each_after_tax: f64,
    pub total_before_tax: f64,
    pub total_after_tax: f64,
    pub minimum_applied: bool,
}

impl fmt::Display for CostOutputs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Each before tax: ${:.2}\nEach after tax: ${:.2}\nTotal before tax: ${:.2}\nTotal after tax: ${:.2}",
            self.each_before_tax, self.each_after_tax, self.total_before_tax, self.total_after_tax
        )?;
        if self.minimum_applied {
            write!(f, "\n(minimum order applied)")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScrapResult {
    pub scrap_lb: f64,
    pub total_cost: f64,
    pub price_per_lb: f64,
    pub scrap_charge_per_lb: f64,
    pub is_pickup: bool,
}

impl fmt::Display for ScrapResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Scrap (lb): {:.3}\nTotal cost: ${:.2}\nPrice per lb: ${:.4}\nScrap charge per lb: ${:.4}\nPickup: {}",
            self.scrap_lb, self.total_cost, self.price_per_lb, self.scrap_charge_per_lb,
            if self.is_pickup { "Yes" } else { "No" }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoilInputs {
    pub coil_width_in: f64,
    pub coil_thickness_in: f64,
    pub coil_id_in: f64,
    pub coil_weight_lb: f64,
    pub density_lb_ft3: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoilResult {
    pub coil_length_in: f64,
    pub coil_footage_ft: f64,
    pub coil_piw_lb_per_in: f64,
    pub coil_od_in: Option<f64>,
}

impl fmt::Display for CoilResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Coil length (in): {:.3}\nCoil footage (ft): {:.3}\nPIW (lb/in): {:.3}",
            self.coil_length_in, self.coil_footage_ft, self.coil_piw_lb_per_in
        )?;
        if let Some(od) = self.coil_od_in {
            write!(f, "\nCoil OD (in): {:.3}", od)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

#[must_use]
pub fn round_up(value: f64, ndigits: u32) -> f64 {
    let factor = 10_f64.powi(ndigits as i32);
    (value.max(0.0) * factor).ceil() / factor
}

#[must_use]
pub fn area_ft2(width_in: f64, length_in: f64) -> f64 {
    (width_in / 12.0) * (length_in / 12.0)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

pub fn validate_inputs(data: &Inputs) -> Result<(), SteelCalError> {
    if data.width_in <= 0.0 || data.length_in <= 0.0 {
        return Err(SteelCalError::validation("Width/Length must be > 0"));
    }
    if data.qty < 0 {
        return Err(SteelCalError::validation("Quantity must be \u{2265} 0"));
    }

    match &data.mode {
        InputMode::Psf(value) => {
            if *value < 0.0 {
                return Err(SteelCalError::validation(
                    "lb/ft\u{00b2} must be \u{2265} 0",
                ));
            }
        }
        InputMode::Gauge { key, .. } => {
            if key.trim().is_empty() {
                return Err(SteelCalError::validation(
                    "Gauge/Size is required when using gauge mode",
                ));
            }
        }
        InputMode::Thickness(t) => {
            if *t <= 0.0 {
                return Err(SteelCalError::validation("Thickness must be > 0"));
            }
        }
    }

    if data.density_lb_ft3 <= 0.0 {
        return Err(SteelCalError::validation("Density must be > 0"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Sheet calculation
// ---------------------------------------------------------------------------

pub fn compute_each_total_psf(
    data: &Inputs,
    tables: &GaugeTables,
) -> Result<SheetResult, SteelCalError> {
    validate_inputs(data)?;

    let area_each = area_ft2(data.width_in, data.length_in);

    let (psf, used_key) = match &data.mode {
        InputMode::Psf(value) => (*value, None),
        InputMode::Gauge { table, key } => {
            let canonical_table = normalize_table_name(table);
            let lookup = get_psf(tables, &canonical_table, key);
            match lookup.psf {
                Some(psf) => (psf, lookup.used_key),
                None if !lookup.suggestions.is_empty() => {
                    let suggestions = lookup
                        .suggestions
                        .iter()
                        .map(|value| format!("'{value}'"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(SteelCalError::lookup(format!(
                        "Size '{key}' not in {canonical_table} table. Did you mean one of: {suggestions}?"
                    )));
                }
                None => {
                    return Err(SteelCalError::lookup(format!(
                        "Size '{key}' not in {canonical_table} table."
                    )));
                }
            }
        }
        InputMode::Thickness(t) => (data.density_lb_ft3 * (*t / 12.0), None),
    };

    let each_lb = round_up(psf * area_each, 3);
    let total_lb = round_up(each_lb * data.qty as f64, 3);

    Ok(SheetResult {
        each_lb,
        total_lb,
        psf,
        used_key,
        area_ft2_each: area_each,
        area_ft2_total: area_each * data.qty as f64,
    })
}

// ---------------------------------------------------------------------------
// Cost calculation
// ---------------------------------------------------------------------------

pub fn compute_costs(
    cost: &CostInputs,
    qty: i32,
    each_lb: f64,
    area_ft2_each: f64,
) -> Result<CostOutputs, SteelCalError> {
    if qty < 0 {
        return Err(SteelCalError::validation("Quantity must be \u{2265} 0"));
    }
    if cost.price_value < 0.0 {
        return Err(SteelCalError::validation("Unit price must be \u{2265} 0"));
    }
    if cost.setup_fee < 0.0 || cost.minimum_order < 0.0 {
        return Err(SteelCalError::validation(
            "Setup fee and minimum must be \u{2265} 0",
        ));
    }

    let mut base_each = match cost.mode {
        PriceMode::PerLb => cost.price_value * each_lb,
        PriceMode::PerFt2 => cost.price_value * area_ft2_each,
        PriceMode::PerSheet => cost.price_value,
    };

    base_each *= 1.0 + cost.markup_pct.max(0.0) / 100.0;

    let subtotal_before_min = base_each * qty as f64 + cost.setup_fee;
    let mut minimum_applied = false;
    let mut effective_before_tax = subtotal_before_min;

    if cost.minimum_order > 0.0 && subtotal_before_min < cost.minimum_order {
        effective_before_tax = cost.minimum_order;
        minimum_applied = true;
    }

    let each_before_tax = if qty > 0 {
        effective_before_tax / qty as f64
    } else {
        0.0
    };
    let total_after_tax = effective_before_tax * (1.0 + cost.tax_pct.max(0.0) / 100.0);
    let each_after_tax = if qty > 0 {
        total_after_tax / qty as f64
    } else {
        0.0
    };

    Ok(CostOutputs {
        each_before_tax: round_scaled(each_before_tax, 2),
        each_after_tax: round_scaled(each_after_tax, 2),
        total_before_tax: round_scaled(effective_before_tax, 2),
        total_after_tax: round_scaled(total_after_tax, 2),
        minimum_applied,
    })
}

// ---------------------------------------------------------------------------
// Scrap calculation
// ---------------------------------------------------------------------------

pub fn compute_scrap(
    actual_weight: f64,
    ending_weight: f64,
    base_cost_per_lb: f64,
    processing_cost_per_lb: f64,
) -> Result<ScrapResult, SteelCalError> {
    if actual_weight < 0.0 || ending_weight <= 0.0 {
        return Err(SteelCalError::validation(
            "Actual weight must be \u{2265} 0 and ending weight must be > 0.",
        ));
    }
    if base_cost_per_lb < 0.0 || processing_cost_per_lb < 0.0 {
        return Err(SteelCalError::validation(
            "Base cost and processing cost must be \u{2265} 0.",
        ));
    }

    let scrap_lb = actual_weight - ending_weight;
    let total_cost = actual_weight * (base_cost_per_lb + processing_cost_per_lb);
    let price_per_lb = if ending_weight > 0.0 {
        total_cost / ending_weight
    } else {
        0.0
    };
    let scrap_charge_per_lb = if ending_weight > 0.0 {
        -scrap_lb * (base_cost_per_lb + processing_cost_per_lb) / ending_weight
    } else {
        0.0
    };

    Ok(ScrapResult {
        scrap_lb,
        total_cost: round_scaled(total_cost, 2),
        price_per_lb: round_scaled(price_per_lb, 4),
        scrap_charge_per_lb: round_scaled(scrap_charge_per_lb, 4),
        is_pickup: scrap_lb < 0.0,
    })
}

// ---------------------------------------------------------------------------
// Coil calculation
// ---------------------------------------------------------------------------

pub fn compute_coil(inputs: &CoilInputs) -> Result<CoilResult, SteelCalError> {
    if inputs.coil_thickness_in <= 0.0 {
        return Err(SteelCalError::validation("Coil thickness required."));
    }
    if inputs.coil_weight_lb < 0.0 {
        return Err(SteelCalError::validation("Coil weight must be >= 0."));
    }
    if inputs.coil_id_in < 0.0 {
        return Err(SteelCalError::validation("Coil ID must be >= 0."));
    }

    let mut coil_length_in = 0.0;
    if inputs.coil_weight_lb > 0.0 {
        if inputs.coil_width_in <= 0.0 {
            return Err(SteelCalError::validation(
                "Coil width required for weight-based calculation.",
            ));
        }

        let psf_derived = inputs.density_lb_ft3 * inputs.coil_thickness_in / 12.0;
        if psf_derived <= 0.0 {
            return Err(SteelCalError::validation("Derived PSF must be positive."));
        }

        coil_length_in = (inputs.coil_weight_lb * 144.0) / (psf_derived * inputs.coil_width_in);
    }

    let coil_footage_ft = if coil_length_in > 0.0 {
        coil_length_in / 12.0
    } else {
        0.0
    };

    let coil_piw_lb_per_in = if inputs.coil_width_in > 0.0 && inputs.coil_weight_lb > 0.0 {
        inputs.coil_weight_lb / inputs.coil_width_in
    } else {
        0.0
    };

    let coil_od_in = if coil_length_in > 0.0 && inputs.coil_id_in > 0.0 {
        Some(
            (inputs.coil_id_in.powi(2) + (4.0 * inputs.coil_thickness_in * coil_length_in) / PI)
                .sqrt(),
        )
    } else {
        None
    };

    Ok(CoilResult {
        coil_length_in,
        coil_footage_ft,
        coil_piw_lb_per_in,
        coil_od_in,
    })
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

pub fn parse_optional_float_text(
    text: &str,
    field_name: &str,
) -> Result<Option<f64>, SteelCalError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let value = trimmed
        .parse::<f64>()
        .map_err(|_| SteelCalError::validation(format!("{field_name} must be a number.")))?;

    if value.is_nan() || value.is_infinite() {
        return Err(SteelCalError::validation(format!(
            "{field_name} must be a finite number."
        )));
    }

    Ok(Some(value))
}

pub fn parse_float_text(
    text: &str,
    field_name: &str,
    default: Option<f64>,
) -> Result<f64, SteelCalError> {
    match parse_optional_float_text(text, field_name)? {
        Some(value) => Ok(value),
        None => {
            default.ok_or_else(|| SteelCalError::validation(format!("{field_name} is required.")))
        }
    }
}

pub fn parse_int_text(
    text: &str,
    field_name: &str,
    default: Option<i32>,
) -> Result<i32, SteelCalError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return default
            .ok_or_else(|| SteelCalError::validation(format!("{field_name} is required.")));
    }

    trimmed
        .parse::<i32>()
        .map_err(|_| SteelCalError::validation(format!("{field_name} must be an integer.")))
}

// ---------------------------------------------------------------------------
// Self-tests (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "selftest")]
pub fn run_self_tests() -> Result<(), SteelCalError> {
    use crate::config::normalize_config;
    use crate::gauges::{builtin_gauge_tables, DEFAULT_TABLE_NAME};
    use serde_json::json;

    let tables = builtin_gauge_tables();

    ensure(
        approx_eq(area_ft2(48.0, 96.0), 32.0, 1e-9),
        "area_ft2 parity failed",
    )?;
    ensure(
        approx_eq(round_up(2.61051, 3), 2.611, 1e-9),
        "round_up parity failed",
    )?;

    let hr = tables
        .get(DEFAULT_TABLE_NAME)
        .ok_or_else(|| SteelCalError::data("Missing HR/HRPO/CR table"))?;
    let galv = tables
        .get("GALV/JK/BOND")
        .ok_or_else(|| SteelCalError::data("Missing GALV/JK/BOND table"))?;

    ensure(
        approx_eq(hr.get("16").unwrap_or_default(), 2.5, 1e-9),
        "HR gauge parity failed",
    )?;
    ensure(
        approx_eq(galv.get("16").unwrap_or_default(), 2.65625, 1e-9),
        "GALV gauge parity failed",
    )?;

    ensure(
        approx_eq(round_up(2.5 * area_ft2(48.0, 96.0), 3), 80.0, 1e-9),
        "Sheet weight parity failed",
    )?;

    let quote = compute_costs(
        &CostInputs {
            mode: PriceMode::PerLb,
            price_value: 1.0,
            markup_pct: 0.0,
            tax_pct: 10.0,
            setup_fee: 0.0,
            minimum_order: 0.0,
        },
        1,
        80.0,
        32.0,
    )?;
    ensure(
        approx_eq(quote.total_after_tax, 88.0, 1e-6),
        "per-lb quote parity failed",
    )?;

    let quote = compute_costs(
        &CostInputs {
            mode: PriceMode::PerFt2,
            price_value: 2.0,
            markup_pct: 0.0,
            tax_pct: 0.0,
            setup_fee: 0.0,
            minimum_order: 0.0,
        },
        3,
        80.0,
        32.0,
    )?;
    ensure(
        approx_eq(quote.each_before_tax, 64.0, 1e-6),
        "per-ft^2 each parity failed",
    )?;
    ensure(
        approx_eq(quote.total_before_tax, 192.0, 1e-6),
        "per-ft^2 total parity failed",
    )?;

    let quote = compute_costs(
        &CostInputs {
            mode: PriceMode::PerSheet,
            price_value: 95.0,
            markup_pct: 0.0,
            tax_pct: 10.0,
            setup_fee: 0.0,
            minimum_order: 0.0,
        },
        10,
        80.0,
        32.0,
    )?;
    ensure(
        approx_eq(quote.total_before_tax, 950.0, 1e-6),
        "per-sheet total parity failed",
    )?;
    ensure(
        approx_eq(quote.total_after_tax, 1045.0, 1e-6),
        "per-sheet tax parity failed",
    )?;

    let quote = compute_costs(
        &CostInputs {
            mode: PriceMode::PerLb,
            price_value: 1.0,
            markup_pct: 25.0,
            tax_pct: 0.0,
            setup_fee: 0.0,
            minimum_order: 0.0,
        },
        1,
        80.0,
        32.0,
    )?;
    ensure(
        approx_eq(quote.each_before_tax, 100.0, 1e-6),
        "markup parity failed",
    )?;

    let quote = compute_costs(
        &CostInputs {
            mode: PriceMode::PerLb,
            price_value: 0.125,
            markup_pct: 0.0,
            tax_pct: 0.0,
            setup_fee: 0.0,
            minimum_order: 150.0,
        },
        1,
        80.0,
        32.0,
    )?;
    ensure(quote.minimum_applied, "minimum-order flag parity failed")?;
    ensure(
        approx_eq(quote.total_before_tax, 150.0, 1e-6),
        "minimum-order total parity failed",
    )?;

    let invalid_gauge = compute_each_total_psf(
        &Inputs {
            width_in: 48.0,
            length_in: 96.0,
            qty: 1,
            mode: InputMode::Gauge {
                table: DEFAULT_TABLE_NAME.to_string(),
                key: "2/7".to_string(),
            },
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        },
        &tables,
    );
    ensure(invalid_gauge.is_err(), "missing gauge parity failed")?;
    ensure(
        invalid_gauge
            .err()
            .map(|error| error.to_string().contains("Did you mean"))
            .unwrap_or(false),
        "missing gauge suggestions parity failed",
    )?;

    let quote = compute_costs(
        &CostInputs {
            mode: PriceMode::PerSheet,
            price_value: 10.0,
            markup_pct: 0.0,
            tax_pct: 0.0,
            setup_fee: 20.0,
            minimum_order: 100.0,
        },
        0,
        0.0,
        0.0,
    )?;
    ensure(
        approx_eq(quote.total_before_tax, 100.0, 1e-9)
            && approx_eq(quote.each_before_tax, 0.0, 1e-9),
        "qty-zero parity failed",
    )?;

    let decimal_key = gauges::key_to_numeric("0.1875");
    ensure(decimal_key.kind == 1, "decimal gauge kind parity failed")?;
    ensure(
        approx_eq(decimal_key.value, 0.1875, 1e-9),
        "decimal gauge value parity failed",
    )?;

    let suggestions = get_psf(&tables, DEFAULT_TABLE_NAME, "999");
    ensure(
        suggestions.suggestions.len() == 3,
        "top-three suggestion parity failed",
    )?;

    ensure(
        normalize_table_name("HR/HRPO/CR/EG") == DEFAULT_TABLE_NAME,
        "table alias parity failed",
    )?;

    let raw_config = json!({
        "default_table": "HR/HRPO/CR/EG",
        "default_gauge": "16",
        "density_lb_ft3": 490.0,
        "ui_font_size": true
    });
    let normalized = normalize_config(
        raw_config
            .as_object()
            .ok_or_else(|| SteelCalError::data("Expected object config fixture"))?,
        &tables,
    );
    ensure(
        normalized
            .get("default_table")
            .and_then(|value| value.as_str())
            .map(|value| value == DEFAULT_TABLE_NAME)
            .unwrap_or(false),
        "config default_table parity failed",
    )?;
    ensure(
        normalized
            .get("default_gauge")
            .and_then(|value| value.as_str())
            .map(|value| value == "16")
            .unwrap_or(false),
        "config default_gauge parity failed",
    )?;
    ensure(
        !normalized.contains_key("ui_font_size"),
        "config invalid int parity failed",
    )?;

    ensure(
        approx_eq(parse_float_text("1.25", "Width", None)?, 1.25, 1e-9),
        "float parsing parity failed",
    )?;
    ensure(
        parse_float_text("1-", "Width", None).is_err(),
        "malformed float parity failed",
    )?;
    ensure(
        compute_scrap(-1.0, 1.0, 0.0, 0.0).is_err(),
        "scrap validation parity failed",
    )?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[must_use]
fn round_scaled(value: f64, digits: u32) -> f64 {
    let factor = 10_f64.powi(digits as i32);
    ((value.abs() * factor + 0.5).floor() / factor).copysign(value)
}

#[cfg(any(feature = "selftest", test))]
fn approx_eq(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() < tolerance
}

#[cfg(feature = "selftest")]
fn ensure(condition: bool, message: &str) -> Result<(), SteelCalError> {
    if condition {
        Ok(())
    } else {
        Err(SteelCalError::data(message))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauges::{builtin_gauge_tables, canonical_gauge_key, DEFAULT_TABLE_NAME};

    // ---- Self-test (only when feature enabled) ----

    #[cfg(feature = "selftest")]
    #[test]
    fn python_parity_self_tests_pass() {
        run_self_tests().unwrap();
    }

    // ---- round_scaled boundary tests ----

    #[test]
    fn round_scaled_half_up_positive() {
        assert_eq!(round_scaled(2.505, 2), 2.51);
    }

    #[test]
    fn round_scaled_half_down_positive() {
        assert_eq!(round_scaled(2.495, 2), 2.50);
    }

    #[test]
    fn round_scaled_half_away_negative() {
        assert_eq!(round_scaled(-2.505, 2), -2.51);
    }

    #[test]
    fn round_scaled_half_down_negative() {
        assert_eq!(round_scaled(-2.495, 2), -2.50);
    }

    #[test]
    fn round_scaled_exact_value() {
        assert_eq!(round_scaled(1.0, 2), 1.0);
        assert_eq!(round_scaled(0.0, 2), 0.0);
    }

    #[test]
    fn round_scaled_zero_digits() {
        assert_eq!(round_scaled(2.5, 0), 3.0);
        assert_eq!(round_scaled(-2.5, 0), -3.0);
        assert_eq!(round_scaled(2.4, 0), 2.0);
    }

    #[test]
    fn round_scaled_three_digits() {
        assert_eq!(round_scaled(1.2345, 3), 1.235);
        assert_eq!(round_scaled(1.2344, 3), 1.234);
        assert_eq!(round_scaled(-1.2345, 3), -1.235);
    }

    #[test]
    fn round_scaled_no_positive_bias() {
        assert_eq!(round_scaled(0.125, 2), 0.13);
        assert_eq!(round_scaled(-0.125, 2), -0.13);
        assert_eq!(round_scaled(0.135, 2), 0.14);
        assert_eq!(round_scaled(-0.135, 2), -0.14);
    }

    #[test]
    fn round_scaled_large_value() {
        assert_eq!(round_scaled(99999.995, 2), 100000.0);
        assert_eq!(round_scaled(123456.785, 2), 123456.79);
    }

    #[test]
    fn round_scaled_small_value() {
        assert_eq!(round_scaled(0.005, 2), 0.01);
        assert_eq!(round_scaled(-0.005, 2), -0.01);
    }

    // ---- canonical_gauge_key / get_psf tolerance tests ----

    #[test]
    fn canonical_gauge_key_uses_tolerance_not_epsilon() {
        let tables = builtin_gauge_tables();
        let result = canonical_gauge_key(&tables, DEFAULT_TABLE_NAME, "16.0");
        assert_eq!(result, Some("16".to_string()));
    }

    #[test]
    fn canonical_gauge_key_rejects_non_integer_decimal() {
        let tables = builtin_gauge_tables();
        let result = canonical_gauge_key(&tables, DEFAULT_TABLE_NAME, "16.5");
        assert_eq!(result, None);
    }

    #[test]
    fn get_psf_integer_lookup_via_tolerance() {
        let tables = builtin_gauge_tables();
        let result = get_psf(&tables, DEFAULT_TABLE_NAME, "16.0");
        assert_eq!(result.psf, Some(2.5));
        assert_eq!(result.used_key, Some("16".to_string()));
    }

    #[test]
    fn get_psf_non_integer_decimal_no_match() {
        let tables = builtin_gauge_tables();
        let result = get_psf(&tables, DEFAULT_TABLE_NAME, "16.1");
        assert!(result.psf.is_none());
        assert!(!result.suggestions.is_empty());
    }

    // ---- InputMode enum tests ----

    #[test]
    fn input_mode_gauge_produces_correct_lookup() {
        let tables = builtin_gauge_tables();
        let data = Inputs {
            width_in: 48.0,
            length_in: 96.0,
            qty: 1,
            mode: InputMode::Gauge {
                table: DEFAULT_TABLE_NAME.to_string(),
                key: "16".to_string(),
            },
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        let result = compute_each_total_psf(&data, &tables).unwrap();
        assert!(approx_eq(result.psf, 2.5, 1e-9));
        assert!(approx_eq(result.each_lb, 80.0, 1e-9));
    }

    #[test]
    fn input_mode_psf_uses_provided_value() {
        let tables = builtin_gauge_tables();
        let data = Inputs {
            width_in: 48.0,
            length_in: 96.0,
            qty: 1,
            mode: InputMode::Psf(3.5),
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        let result = compute_each_total_psf(&data, &tables).unwrap();
        assert!(approx_eq(result.psf, 3.5, 1e-9));
    }

    #[test]
    fn input_mode_thickness_derives_psf() {
        let tables = builtin_gauge_tables();
        let thickness = 0.5;
        let expected_psf = DENSITY_LB_PER_FT3_DEFAULT * (thickness / 12.0);
        let data = Inputs {
            width_in: 48.0,
            length_in: 96.0,
            qty: 1,
            mode: InputMode::Thickness(thickness),
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        let result = compute_each_total_psf(&data, &tables).unwrap();
        assert!(approx_eq(result.psf, expected_psf, 1e-9));
    }

    #[test]
    fn input_mode_gauge_validates_empty_key() {
        let data = Inputs {
            width_in: 48.0,
            length_in: 96.0,
            qty: 1,
            mode: InputMode::Gauge {
                table: DEFAULT_TABLE_NAME.to_string(),
                key: "  ".to_string(),
            },
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        assert!(validate_inputs(&data).is_err());
    }

    #[test]
    fn input_mode_psf_validates_negative() {
        let data = Inputs {
            width_in: 48.0,
            length_in: 96.0,
            qty: 1,
            mode: InputMode::Psf(-1.0),
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        assert!(validate_inputs(&data).is_err());
    }

    #[test]
    fn input_mode_thickness_validates_zero() {
        let data = Inputs {
            width_in: 48.0,
            length_in: 96.0,
            qty: 1,
            mode: InputMode::Thickness(0.0),
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        assert!(validate_inputs(&data).is_err());
    }

    // ---- PriceMode enum tests ----

    #[test]
    fn price_mode_per_lb_works() {
        let cost = CostInputs {
            mode: PriceMode::PerLb,
            price_value: 1.0,
            markup_pct: 0.0,
            tax_pct: 0.0,
            setup_fee: 0.0,
            minimum_order: 0.0,
        };
        let result = compute_costs(&cost, 1, 80.0, 32.0).unwrap();
        assert!(approx_eq(result.total_before_tax, 80.0, 1e-6));
    }

    #[test]
    fn price_mode_per_ft2_works() {
        let cost = CostInputs {
            mode: PriceMode::PerFt2,
            price_value: 2.0,
            markup_pct: 0.0,
            tax_pct: 0.0,
            setup_fee: 0.0,
            minimum_order: 0.0,
        };
        let result = compute_costs(&cost, 1, 80.0, 32.0).unwrap();
        assert!(approx_eq(result.total_before_tax, 64.0, 1e-6));
    }

    #[test]
    fn price_mode_per_sheet_works() {
        let cost = CostInputs {
            mode: PriceMode::PerSheet,
            price_value: 95.0,
            markup_pct: 0.0,
            tax_pct: 0.0,
            setup_fee: 0.0,
            minimum_order: 0.0,
        };
        let result = compute_costs(&cost, 10, 80.0, 32.0).unwrap();
        assert!(approx_eq(result.total_before_tax, 950.0, 1e-6));
    }

    // ---- Display tests ----

    #[test]
    fn sheet_result_display() {
        let result = SheetResult {
            each_lb: 80.0,
            total_lb: 800.0,
            psf: 2.5,
            used_key: Some("16".to_string()),
            area_ft2_each: 32.0,
            area_ft2_total: 320.0,
        };
        let output = format!("{result}");
        assert!(output.contains("Each (lb): 80.000"));
        assert!(output.contains("Total (lb): 800.000"));
        assert!(output.contains("PSF (lb/ft²): 2.5000"));
        assert!(output.contains("Used key: 16"));
    }

    #[test]
    fn sheet_result_display_no_key() {
        let result = SheetResult {
            each_lb: 80.0,
            total_lb: 800.0,
            psf: 2.5,
            used_key: None,
            area_ft2_each: 32.0,
            area_ft2_total: 320.0,
        };
        let output = format!("{result}");
        assert!(!output.contains("Used key"));
    }

    #[test]
    fn cost_outputs_display() {
        let result = CostOutputs {
            each_before_tax: 80.0,
            each_after_tax: 88.0,
            total_before_tax: 800.0,
            total_after_tax: 880.0,
            minimum_applied: false,
        };
        let output = format!("{result}");
        assert!(output.contains("Each before tax: $80.00"));
        assert!(output.contains("Total after tax: $880.00"));
        assert!(!output.contains("minimum order"));
    }

    #[test]
    fn cost_outputs_display_with_minimum() {
        let result = CostOutputs {
            each_before_tax: 150.0,
            each_after_tax: 150.0,
            total_before_tax: 150.0,
            total_after_tax: 150.0,
            minimum_applied: true,
        };
        let output = format!("{result}");
        assert!(output.contains("(minimum order applied)"));
    }

    #[test]
    fn scrap_result_display() {
        let result = ScrapResult {
            scrap_lb: 10.0,
            total_cost: 100.0,
            price_per_lb: 2.0,
            scrap_charge_per_lb: -0.5,
            is_pickup: false,
        };
        let output = format!("{result}");
        assert!(output.contains("Scrap (lb): 10.000"));
        assert!(output.contains("Total cost: $100.00"));
        assert!(output.contains("Pickup: No"));
    }

    #[test]
    fn scrap_result_display_pickup() {
        let result = ScrapResult {
            scrap_lb: -5.0,
            total_cost: 50.0,
            price_per_lb: 1.0,
            scrap_charge_per_lb: 0.5,
            is_pickup: true,
        };
        let output = format!("{result}");
        assert!(output.contains("Pickup: Yes"));
    }

    #[test]
    fn coil_result_display() {
        let result = CoilResult {
            coil_length_in: 1200.0,
            coil_footage_ft: 100.0,
            coil_piw_lb_per_in: 25.0,
            coil_od_in: Some(36.5),
        };
        let output = format!("{result}");
        assert!(output.contains("Coil footage (ft): 100.000"));
        assert!(output.contains("PIW (lb/in): 25.000"));
        assert!(output.contains("Coil OD (in): 36.500"));
    }

    #[test]
    fn coil_result_display_no_od() {
        let result = CoilResult {
            coil_length_in: 0.0,
            coil_footage_ft: 0.0,
            coil_piw_lb_per_in: 0.0,
            coil_od_in: None,
        };
        let output = format!("{result}");
        assert!(!output.contains("Coil OD"));
    }

    // ---- SteelCalError user_message tests ----

    #[test]
    fn error_user_message_validation() {
        let err = SteelCalError::validation("Width must be > 0");
        assert_eq!(err.user_message(), "Width must be > 0");
    }

    #[test]
    fn error_user_message_lookup() {
        let err = SteelCalError::lookup("Gauge not found");
        assert_eq!(err.user_message(), "Gauge not found");
    }

    #[test]
    fn error_user_message_config() {
        let err = SteelCalError::config("Bad config");
        assert_eq!(err.user_message(), "Configuration error: Bad config");
    }

    #[test]
    fn error_user_message_data() {
        let err = SteelCalError::data("Corrupt data");
        assert_eq!(err.user_message(), "Data error: Corrupt data");
    }

    // ---- parse_float_text NaN/Infinity rejection tests ----

    #[test]
    fn parse_float_text_rejects_nan() {
        let result = parse_float_text("NaN", "Width", None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SteelCalError::Validation(_)),
            "Expected Validation error, got: {err:?}"
        );
    }

    #[test]
    fn parse_float_text_rejects_infinity() {
        let result = parse_float_text("Infinity", "Width", None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SteelCalError::Validation(_)));
    }

    #[test]
    fn parse_float_text_rejects_negative_infinity() {
        let result = parse_float_text("-Infinity", "Width", None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SteelCalError::Validation(_)));
    }

    #[test]
    fn parse_float_text_rejects_inf() {
        let result = parse_float_text("inf", "Width", None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SteelCalError::Validation(_)));
    }

    #[test]
    fn parse_optional_float_text_rejects_nan() {
        let result = parse_optional_float_text("NaN", "Width");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SteelCalError::Validation(_)));
    }

    #[test]
    fn parse_optional_float_text_rejects_infinity() {
        let result = parse_optional_float_text("Infinity", "Width");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SteelCalError::Validation(_)));
    }

    #[test]
    fn parse_optional_float_text_rejects_neg_infinity() {
        let result = parse_optional_float_text("-Infinity", "Width");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SteelCalError::Validation(_)));
    }

    #[test]
    fn parse_optional_float_text_rejects_inf() {
        let result = parse_optional_float_text("inf", "Width");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SteelCalError::Validation(_)));
    }

    #[test]
    fn parse_float_text_accepts_valid_numbers() {
        assert_eq!(parse_float_text("1.25", "Width", None).unwrap(), 1.25);
        assert_eq!(parse_float_text("-3.5", "Width", None).unwrap(), -3.5);
        assert_eq!(parse_float_text("0", "Width", None).unwrap(), 0.0);
        assert_eq!(parse_float_text("  42  ", "Width", None).unwrap(), 42.0);
    }

    #[test]
    fn parse_optional_float_text_empty_returns_none() {
        assert_eq!(parse_optional_float_text("", "Width").unwrap(), None);
        assert_eq!(parse_optional_float_text("  ", "Width").unwrap(), None);
    }

    #[test]
    fn parse_float_text_empty_with_default() {
        assert_eq!(parse_float_text("", "Width", Some(5.0)).unwrap(), 5.0);
    }

    #[test]
    fn parse_float_text_empty_without_default() {
        let result = parse_float_text("", "Width", None);
        assert!(result.is_err());
    }

    // ---- Case-insensitive table alias tests ----

    #[test]
    fn normalize_table_name_case_insensitive() {
        assert_eq!(normalize_table_name("hr/hrpo/cr"), "HR/HRPO/CR");
        assert_eq!(normalize_table_name("Hr/Hrpo/Cr"), "HR/HRPO/CR");
        assert_eq!(normalize_table_name("HR/HRPO/CR"), "HR/HRPO/CR");
    }

    #[test]
    fn normalize_table_name_alias_case_insensitive() {
        assert_eq!(normalize_table_name("hr/hrpo/cr/eg"), "HR/HRPO/CR");
        assert_eq!(normalize_table_name("HR/HRPO/CR/EG"), "HR/HRPO/CR");
        assert_eq!(normalize_table_name("Hr/Hrpo/Cr/Eg"), "HR/HRPO/CR");
    }

    #[test]
    fn normalize_table_name_floor_plate_case_insensitive() {
        assert_eq!(normalize_table_name("hr floor plate"), "HR FLOOR PLATE");
        assert_eq!(normalize_table_name("HR Floor Plate"), "HR FLOOR PLATE");
        assert_eq!(normalize_table_name("HDP (Mill Plate)"), "HR FLOOR PLATE");
        assert_eq!(normalize_table_name("hdp (mill plate)"), "HR FLOOR PLATE");
    }

    #[test]
    fn get_psf_case_insensitive_table() {
        let tables = builtin_gauge_tables();
        let result = get_psf(&tables, "hr/hrpo/cr", "16");
        assert_eq!(result.psf, Some(2.5));
    }

    #[test]
    fn get_psf_case_insensitive_hr_floor_plate() {
        let tables = builtin_gauge_tables();
        let result = get_psf(&tables, "hr floor plate", "1/4");
        assert_eq!(result.psf, Some(11.26));
    }

    // ---- APP_VERSION / APP_COPYRIGHT tests ----

    #[test]
    fn app_version_matches_cargo_pkg_version() {
        assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn app_copyright_exists() {
        assert!(!APP_COPYRIGHT.is_empty());
        assert!(APP_COPYRIGHT.contains("Harbor Pipe & Steel Inc."));
    }

    // ---- PriceMode display ----

    #[test]
    fn price_mode_display() {
        assert_eq!(PriceMode::PerLb.to_string(), "per lb");
        assert_eq!(PriceMode::PerFt2.to_string(), "per ft²");
        assert_eq!(PriceMode::PerSheet.to_string(), "per sheet");
    }

    // ====================================================================
    // Python-baseline parity tests
    // ====================================================================
    //
    // Each test below verifies a specific calculation path against
    // expected values computed from the Python reference implementation.

    // ---- Sheet-by-gauge parity ----

    #[test]
    fn parity_sheet_by_gauge() {
        // HR/HRPO/CR gauge 16, width 48, length 120, qty 10
        // Python baseline: each_lb=100.0, total_lb=1000.0, psf=2.5
        let tables = builtin_gauge_tables();
        let data = Inputs {
            width_in: 48.0,
            length_in: 120.0,
            qty: 10,
            mode: InputMode::Gauge {
                table: DEFAULT_TABLE_NAME.to_string(),
                key: "16".to_string(),
            },
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        let result = compute_each_total_psf(&data, &tables).unwrap();

        assert!(
            approx_eq(result.psf, 2.5, 1e-9),
            "PSF mismatch: got {}, expected 2.5",
            result.psf,
        );
        assert!(
            approx_eq(result.each_lb, 100.0, 1e-9),
            "each_lb mismatch: got {}, expected 100.0",
            result.each_lb,
        );
        assert!(
            approx_eq(result.total_lb, 1000.0, 1e-9),
            "total_lb mismatch: got {}, expected 1000.0",
            result.total_lb,
        );
        assert_eq!(result.used_key, Some("16".to_string()));
        assert!(
            approx_eq(result.area_ft2_each, 40.0, 1e-9),
            "area_ft2_each mismatch: got {}, expected 40.0",
            result.area_ft2_each,
        );
        assert!(
            approx_eq(result.area_ft2_total, 400.0, 1e-9),
            "area_ft2_total mismatch: got {}, expected 400.0",
            result.area_ft2_total,
        );
    }

    // ---- Sheet-by-PSF parity ----

    #[test]
    fn parity_sheet_by_psf() {
        // PSF 3.5, width 36, length 96, qty 5
        // Python baseline: each_lb=84.0, total_lb=420.0, psf=3.5
        let tables = builtin_gauge_tables();
        let data = Inputs {
            width_in: 36.0,
            length_in: 96.0,
            qty: 5,
            mode: InputMode::Psf(3.5),
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        let result = compute_each_total_psf(&data, &tables).unwrap();

        assert!(
            approx_eq(result.psf, 3.5, 1e-9),
            "PSF mismatch: got {}, expected 3.5",
            result.psf,
        );
        assert!(
            approx_eq(result.each_lb, 84.0, 1e-9),
            "each_lb mismatch: got {}, expected 84.0",
            result.each_lb,
        );
        assert!(
            approx_eq(result.total_lb, 420.0, 1e-9),
            "total_lb mismatch: got {}, expected 420.0",
            result.total_lb,
        );
        assert_eq!(result.used_key, None);
        assert!(
            approx_eq(result.area_ft2_each, 24.0, 1e-9),
            "area_ft2_each mismatch: got {}, expected 24.0",
            result.area_ft2_each,
        );
        assert!(
            approx_eq(result.area_ft2_total, 120.0, 1e-9),
            "area_ft2_total mismatch: got {}, expected 120.0",
            result.area_ft2_total,
        );
    }

    // ---- Sheet-by-thickness parity ----

    #[test]
    fn parity_sheet_by_thickness() {
        // thickness 0.0625, width 48, length 120, qty 10
        // Python baseline: psf=2.552083333..., each_lb=102.084, total_lb=1020.84
        let tables = builtin_gauge_tables();
        let data = Inputs {
            width_in: 48.0,
            length_in: 120.0,
            qty: 10,
            mode: InputMode::Thickness(0.0625),
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        let result = compute_each_total_psf(&data, &tables).unwrap();

        let expected_psf = DENSITY_LB_PER_FT3_DEFAULT * (0.0625 / 12.0);
        assert!(
            approx_eq(result.psf, expected_psf, 1e-9),
            "PSF mismatch: got {}, expected {}",
            result.psf,
            expected_psf,
        );
        assert!(
            approx_eq(result.each_lb, 102.084, 1e-9),
            "each_lb mismatch: got {}, expected 102.084",
            result.each_lb,
        );
        assert!(
            approx_eq(result.total_lb, 1020.84, 1e-9),
            "total_lb mismatch: got {}, expected 1020.84",
            result.total_lb,
        );
        assert_eq!(result.used_key, None);
        assert!(
            approx_eq(result.area_ft2_each, 40.0, 1e-9),
            "area_ft2_each mismatch: got {}, expected 40.0",
            result.area_ft2_each,
        );
        assert!(
            approx_eq(result.area_ft2_total, 400.0, 1e-9),
            "area_ft2_total mismatch: got {}, expected 400.0",
            result.area_ft2_total,
        );
    }

    // ---- Coil parity ----

    #[test]
    fn parity_coil() {
        // Coil: width 48, thickness 0.06, ID 20, weight 2000, density 490
        // Python baseline: footage≈204.0816, PIW≈41.6667, OD≈24.2299
        let inputs = CoilInputs {
            coil_width_in: 48.0,
            coil_thickness_in: 0.06,
            coil_id_in: 20.0,
            coil_weight_lb: 2000.0,
            density_lb_ft3: DENSITY_LB_PER_FT3_DEFAULT,
        };
        let result = compute_coil(&inputs).unwrap();

        assert!(
            approx_eq(result.coil_footage_ft, 204.0816326530612, 1e-6),
            "footage mismatch: got {}, expected ≈204.0816",
            result.coil_footage_ft,
        );
        assert!(
            approx_eq(result.coil_piw_lb_per_in, 41.66666666666666, 1e-6),
            "PIW mismatch: got {}, expected ≈41.6667",
            result.coil_piw_lb_per_in,
        );
        let od = result
            .coil_od_in
            .expect("OD should be computed when ID > 0");
        assert!(
            approx_eq(od, 24.22990424319821, 1e-4),
            "OD mismatch: got {}, expected ≈24.2299",
            od,
        );

        // Also verify length in inches
        assert!(
            approx_eq(result.coil_length_in, 2448.979591836735, 1e-6),
            "coil_length_in mismatch: got {}, expected ≈2448.9796",
            result.coil_length_in,
        );
    }

    // ---- Scrap parity ----

    #[test]
    fn parity_scrap() {
        // Scrap (no pickup): actual 5000, ending 4800, base_cost 0.35, proc_cost 0.05
        // Python baseline: scrap_lb=200.0, total_cost≈2000.0, price_per_lb≈0.4167,
        //                  scrap_charge≈-0.0167, is_pickup=false
        let result = compute_scrap(5000.0, 4800.0, 0.35, 0.05).unwrap();

        assert!(
            approx_eq(result.scrap_lb, 200.0, 1e-9),
            "scrap_lb mismatch: got {}, expected 200.0",
            result.scrap_lb,
        );
        assert!(
            approx_eq(result.total_cost, 2000.0, 1e-2),
            "total_cost mismatch: got {}, expected 2000.0",
            result.total_cost,
        );
        assert!(
            approx_eq(result.price_per_lb, 0.4167, 1e-4),
            "price_per_lb mismatch: got {}, expected ≈0.4167",
            result.price_per_lb,
        );
        assert!(
            approx_eq(result.scrap_charge_per_lb, -0.0167, 1e-4),
            "scrap_charge_per_lb mismatch: got {}, expected ≈-0.0167",
            result.scrap_charge_per_lb,
        );
        assert!(
            !result.is_pickup,
            "Expected is_pickup=false for positive scrap",
        );
    }

    #[test]
    fn parity_scrap_pickup() {
        // Scrap with pickup: actual 4500, ending 5000, base_cost 0.35, proc_cost 0.05
        // Python baseline: scrap_lb=-500.0, total_cost≈1800.0, price_per_lb≈0.36,
        //                  scrap_charge≈0.04, is_pickup=true
        let result = compute_scrap(4500.0, 5000.0, 0.35, 0.05).unwrap();

        assert!(
            approx_eq(result.scrap_lb, -500.0, 1e-9),
            "scrap_lb mismatch: got {}, expected -500.0",
            result.scrap_lb,
        );
        assert!(
            approx_eq(result.total_cost, 1800.0, 1e-2),
            "total_cost mismatch: got {}, expected 1800.0",
            result.total_cost,
        );
        assert!(
            approx_eq(result.price_per_lb, 0.36, 1e-4),
            "price_per_lb mismatch: got {}, expected 0.36",
            result.price_per_lb,
        );
        assert!(
            approx_eq(result.scrap_charge_per_lb, 0.04, 1e-4),
            "scrap_charge_per_lb mismatch: got {}, expected 0.04",
            result.scrap_charge_per_lb,
        );
        assert!(
            result.is_pickup,
            "Expected is_pickup=true when actual < ending",
        );
    }
}
