use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::SteelCalError;

pub const DEFAULT_TABLE_NAME: &str = "HR/HRPO/CR";
pub const DEFAULT_GAUGE_KEY: &str = "16";

pub const TABLE_ALIASES: &[(&str, &str)] = &[
    ("HR/HRPO/CR/EG", DEFAULT_TABLE_NAME),
    ("HR Floor Plate", "HR FLOOR PLATE"),
    ("HDP (Mill Plate)", "HR FLOOR PLATE"),
];

pub const IGNORED_XLSX_MATERIAL_TYPES: &[&str] = &["A-40", "A-60", "BOND"];

pub const XLSX_NAME_MAP: &[(&str, &str)] = &[
    ("CRS", "HR/HRPO/CR"),
    ("HRS", "HR/HRPO/CR"),
    ("GALVS", "GALV/JK/BOND"),
    ("ALUM", "ALUMINIZED"),
    ("AL1", "ALUMINUM"),
    ("HDP", "HR FLOOR PLATE"),
    ("HRP", "HOT ROLLED PLATE"),
    ("STAIN", "STAINLESS"),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyNumeric {
    pub kind: u8,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaugeEntry {
    pub key: String,
    pub psf: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GaugeTable {
    pub entries: Vec<GaugeEntry>,
}

pub type GaugeTables = BTreeMap<String, GaugeTable>;

#[derive(Debug, Clone, PartialEq)]
pub struct LookupResult {
    pub psf: Option<f64>,
    pub used_key: Option<String>,
    pub suggestions: Vec<String>,
}

impl GaugeTable {
    #[must_use]
    pub fn new(entries: &[(&str, f64)]) -> Self {
        let mut table_entries = entries
            .iter()
            .map(|(key, psf)| GaugeEntry {
                key: (*key).to_string(),
                psf: *psf,
            })
            .collect::<Vec<_>>();
        sort_entries(&mut table_entries);
        Self {
            entries: table_entries,
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<f64> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.psf)
    }

    #[must_use]
    pub fn first_key(&self) -> Option<&str> {
        self.entries.first().map(|entry| entry.key.as_str())
    }

    #[must_use]
    pub fn entries(&self) -> &[GaugeEntry] {
        &self.entries
    }
}

#[must_use]
pub fn parse_fraction_to_float(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if !trimmed.contains('/') {
        return None;
    }

    // Handle mixed fractions like "1-1/2", "1-3/4", "2-1/4"
    if let Some(dash_pos) = trimmed.find('-') {
        let whole_part = trimmed[..dash_pos].trim();
        let frac_part = trimmed[dash_pos + 1..].trim();

        // Only treat as mixed fraction if whole part is a valid number
        // and fraction part contains a '/'
        if frac_part.contains('/') {
            if let Ok(whole) = whole_part.parse::<f64>() {
                let mut frac_parts = frac_part.splitn(2, '/');
                let numerator = frac_parts.next()?.trim().parse::<f64>().ok()?;
                let denominator = frac_parts.next()?.trim().parse::<f64>().ok()?;
                if denominator == 0.0 {
                    return None;
                }
                return Some(whole + numerator / denominator);
            }
        }
    }

    // Simple fraction like "1/2", "3/4"
    let mut parts = trimmed.splitn(2, '/');
    let numerator = parts.next()?.trim().parse::<f64>().ok()?;
    let denominator = parts.next()?.trim().parse::<f64>().ok()?;
    if denominator == 0.0 {
        return None;
    }

    Some(numerator / denominator)
}

#[must_use]
pub fn key_to_numeric(key: &str) -> KeyNumeric {
    let normalized = key.trim().to_lowercase().replace(" inch", "");
    if let Ok(value) = normalized.parse::<f64>() {
        if value == value.trunc() {
            return KeyNumeric { kind: 0, value };
        }
        return KeyNumeric { kind: 1, value };
    }

    if let Some(value) = parse_fraction_to_float(&normalized) {
        return KeyNumeric { kind: 1, value };
    }

    KeyNumeric {
        kind: 2,
        value: f64::INFINITY,
    }
}

pub fn sort_entries(entries: &mut [GaugeEntry]) {
    entries.sort_by(|left, right| compare_keys(&left.key, &right.key));
}

#[must_use]
pub fn compare_keys(left: &str, right: &str) -> Ordering {
    let left_numeric = key_to_numeric(left);
    let right_numeric = key_to_numeric(right);

    left_numeric
        .kind
        .cmp(&right_numeric.kind)
        .then_with(|| {
            left_numeric
                .value
                .partial_cmp(&right_numeric.value)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.cmp(right))
}

#[must_use]
pub fn normalize_table_name(table_name: &str) -> String {
    let trimmed = table_name.trim();
    let upper = trimmed.to_uppercase();

    // Check aliases (case-insensitive)
    if let Some((_, target)) = TABLE_ALIASES
        .iter()
        .find(|(source, _)| source.to_uppercase() == upper)
    {
        return (*target).to_string();
    }

    // Check if the input matches a known builtin table name (case-insensitive)
    // Return the canonical (uppercase) form from the table data.
    let known_tables = [
        DEFAULT_TABLE_NAME,
        "GALV/JK/BOND",
        "ALUMINIZED",
        "ALUMINUM",
        "HR FLOOR PLATE",
        "HOT ROLLED PLATE",
        "STAINLESS",
    ];

    for &canonical in &known_tables {
        if canonical.to_uppercase() == upper {
            return canonical.to_string();
        }
    }

    // Fallback: return as-is with original casing (for unknown tables that
    // may have been loaded from overrides at runtime).
    trimmed.to_string()
}

#[must_use]
pub fn normalize_source_material_name(material_type: &str) -> String {
    let trimmed = material_type.trim();
    XLSX_NAME_MAP
        .iter()
        .find(|(source, _)| *source == trimmed)
        .map(|(_, target)| normalize_table_name(target))
        .unwrap_or_else(|| normalize_table_name(trimmed))
}

#[must_use]
pub fn canonical_gauge_key(
    tables: &GaugeTables,
    table_name: &str,
    gauge_key: &str,
) -> Option<String> {
    let canonical_table = normalize_table_name(table_name);
    let table = tables.get(&canonical_table)?;

    let key = gauge_key.trim();
    if table.get(key).is_some() {
        return Some(key.to_string());
    }

    let parsed = key.parse::<f64>().ok()?;
    if (parsed - parsed.trunc()).abs() >= 1e-9 {
        return None;
    }

    let alt_key = (parsed as i64).to_string();
    table.get(&alt_key).map(|_| alt_key)
}

#[must_use]
pub fn get_psf(table_map: &GaugeTables, table_name: &str, key_raw: &str) -> LookupResult {
    let canonical_table = normalize_table_name(table_name);
    let Some(table) = table_map.get(&canonical_table) else {
        return LookupResult {
            psf: None,
            used_key: None,
            suggestions: Vec::new(),
        };
    };

    let key = key_raw.trim();
    if let Some(psf) = table.get(key) {
        return LookupResult {
            psf: Some(psf),
            used_key: Some(key.to_string()),
            suggestions: Vec::new(),
        };
    }

    if let Ok(parsed) = key.parse::<f64>() {
        if (parsed - parsed.trunc()).abs() < 1e-9 {
            let alt_key = (parsed as i64).to_string();
            if let Some(psf) = table.get(&alt_key) {
                return LookupResult {
                    psf: Some(psf),
                    used_key: Some(alt_key),
                    suggestions: Vec::new(),
                };
            }
        }
    }

    if table.entries().is_empty() {
        return LookupResult {
            psf: None,
            used_key: None,
            suggestions: Vec::new(),
        };
    }

    let target_numeric = key_to_numeric(key);
    let average_psf =
        table.entries().iter().map(|entry| entry.psf).sum::<f64>() / table.entries().len() as f64;

    let mut distances = table
        .entries()
        .iter()
        .map(|entry| {
            let entry_numeric = key_to_numeric(&entry.key);
            let distance = if target_numeric.kind == entry_numeric.kind && target_numeric.kind != 2
            {
                (target_numeric.value - entry_numeric.value).abs()
            } else {
                (entry.psf - average_psf).abs()
            };
            (distance, entry.key.clone())
        })
        .collect::<Vec<_>>();

    distances.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.1.cmp(&right.1))
    });

    LookupResult {
        psf: None,
        used_key: None,
        suggestions: distances.into_iter().take(3).map(|(_, key)| key).collect(),
    }
}

#[must_use]
pub fn builtin_gauge_tables() -> GaugeTables {
    let mut tables = GaugeTables::new();
    tables.insert(DEFAULT_TABLE_NAME.to_string(), GaugeTable::new(HR_HRPO_CR));
    tables.insert("GALV/JK/BOND".to_string(), GaugeTable::new(GALV_JK_BOND));
    tables.insert("ALUMINIZED".to_string(), GaugeTable::new(ALUMINIZED));
    tables.insert("ALUMINUM".to_string(), GaugeTable::new(ALUMINUM));
    tables.insert(
        "HR FLOOR PLATE".to_string(),
        GaugeTable::new(HR_FLOOR_PLATE),
    );
    tables.insert(
        "HOT ROLLED PLATE".to_string(),
        GaugeTable::new(HOT_ROLLED_PLATE),
    );
    tables.insert("STAINLESS".to_string(), GaugeTable::new(STAINLESS));
    tables
}

/// Load override gauge tables from a JSON file at the given path.
///
/// The JSON format is: `{ "TABLE_NAME": { "gauge_key": psf_value, ... }, ... }`
///
/// If the file does not exist, returns an empty `GaugeTables` (no error).
/// If the file exists but contains invalid JSON, returns a `SteelCalError`.
pub fn load_override_tables(path: &Path) -> Result<GaugeTables, SteelCalError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GaugeTables::new());
        }
        Err(err) => return Err(SteelCalError::Io(err)),
    };

    let raw: BTreeMap<String, BTreeMap<String, f64>> = serde_json::from_str(&contents)?;

    let mut tables = GaugeTables::new();
    for (table_name, entries_map) in raw {
        let entries: Vec<(&str, f64)> = entries_map.iter().map(|(k, &v)| (k.as_str(), v)).collect();
        tables.insert(table_name, GaugeTable::new(&entries));
    }

    Ok(tables)
}

/// Merge override tables into builtin tables using entry-level merge semantics.
///
/// - Override entries update existing keys in a builtin table (override wins on conflict).
/// - Builtin entries with no matching override key are preserved.
/// - Override tables that have no builtin equivalent are added as new tables.
pub fn merge_tables(builtins: &mut GaugeTables, overrides: &GaugeTables) {
    for (table_name, override_table) in overrides {
        if let Some(builtin_table) = builtins.get_mut(table_name) {
            // Entry-level merge: update or insert each override entry
            for override_entry in override_table.entries() {
                if let Some(existing) = builtin_table
                    .entries
                    .iter_mut()
                    .find(|e| e.key == override_entry.key)
                {
                    existing.psf = override_entry.psf;
                } else {
                    builtin_table.entries.push(GaugeEntry {
                        key: override_entry.key.clone(),
                        psf: override_entry.psf,
                    });
                }
            }
            // Re-sort after merge to maintain numeric ordering
            sort_entries(&mut builtin_table.entries);
        } else {
            // New table from override — add it wholesale
            builtins.insert(table_name.clone(), override_table.clone());
        }
    }
}

const HR_HRPO_CR: &[(&str, f64)] = &[
    ("4", 9.375),
    ("5", 8.75),
    ("7", 7.5),
    ("8", 6.875),
    ("9", 6.25),
    ("10", 5.625),
    ("11", 5.0),
    ("12", 4.375),
    ("13", 3.75),
    ("14", 3.125),
    ("15", 2.8125),
    ("16", 2.5),
    ("17", 2.25),
    ("18", 2.0),
    ("19", 1.75),
    ("20", 1.5),
    ("21", 1.375),
    ("22", 1.25),
    ("23", 1.125),
    ("24", 1.0),
    ("25", 0.875),
    ("26", 0.75),
    ("27", 0.6875),
    ("28", 0.625),
    ("29", 0.5625),
    ("30", 0.5),
    ("3/16", 7.66),
    ("1/4", 10.21),
    ("9/32", 11.924813),
    ("5/16", 12.76),
    ("11/32", 13.476672),
    ("23/64", 14.29344),
    ("3/8", 15.31),
    ("7/16", 17.87),
    ("29/64", 18.581472),
    ("15/32", 18.867341),
    ("1/2", 20.4192),
    ("9/16", 22.97),
    ("5/8", 25.52),
    ("11/16", 28.08),
    ("3/4", 30.63),
    ("13/16", 33.17),
    ("7/8", 35.73),
    ("1.00", 40.84),
    ("1.25", 51.05),
    ("1.375", 56.1),
    ("1.50", 61.26),
    ("1.75", 71.47),
    ("2.00", 81.68),
    ("2.50", 102.1),
    ("3.00", 122.52),
];

const GALV_JK_BOND: &[(&str, f64)] = &[
    ("8", 7.031),
    ("9", 6.25),
    ("10", 5.78125),
    ("11", 5.15625),
    ("12", 4.53125),
    ("13", 3.90625),
    ("14", 3.28125),
    ("15", 2.96875),
    ("16", 2.65625),
    ("17", 2.40625),
    ("18", 2.15625),
    ("19", 1.90625),
    ("20", 1.65625),
    ("22", 1.40625),
    ("23", 1.28125),
    ("24", 1.15625),
    ("25", 1.03125),
    ("26", 0.90625),
    ("28", 0.78125),
    ("30", 0.65625),
    ("32", 0.53085),
];

// NOTE: The "ALUMINUM" entry in gauge_tables.override.json actually contains
// ALUMINIZED data (the XLSX name map AL1 → ALUMINUM was swapped with ALUM →
// ALUMINIZED in the source data). The override's "ALUMINUM" values match
// ALUMINIZED PSF values. We reconcile by including gauge 13 (3.827) from that
// override data. The override's "ALUMINIZED" entry (11: 1.746, 18: 0.713)
// appears to contain misattributed data and is intentionally not applied.
const ALUMINIZED: &[(&str, f64)] = &[
    ("12", 4.452),
    ("13", 3.827),
    ("14", 3.202),
    ("16", 2.577),
    ("18", 2.077),
    ("20", 1.577),
    ("22", 1.327),
    ("24", 1.077),
    ("26", 0.827),
    ("28", 0.702),
];

const ALUMINUM: &[(&str, f64)] = &[
    ("3", 3.22),
    ("4", 2.87),
    ("5", 2.55),
    ("6", 2.27),
    ("7", 2.03),
    ("8", 1.80),
    ("9", 1.61),
    ("10", 1.43),
    ("11", 1.27),
    ("12", 1.13),
    ("13", 1.01),
    ("14", 0.90),
    ("15", 0.80),
    ("16", 0.71),
    ("17", 0.64),
    ("18", 0.57),
    ("19", 0.50),
    ("20", 0.45),
    ("21", 0.40),
    ("22", 0.36),
    ("23", 0.32),
    ("24", 0.28),
    ("25", 0.25),
    ("26", 0.22),
    ("27", 0.20),
    ("28", 0.18),
    ("29", 0.16),
    ("30", 0.14),
    ("31", 0.13),
    ("32", 0.11),
    ("33", 0.10),
    ("34", 0.09),
    ("35", 0.08),
];

const HR_FLOOR_PLATE: &[(&str, f64)] = &[
    ("12", 5.25),
    ("14", 3.75),
    ("16", 2.95),
    ("1/8", 6.16),
    ("3/16", 8.71),
    ("1/4", 11.26),
    ("5/16", 13.81),
    ("3/8", 16.37),
    ("1/2", 21.47),
    ("3/4", 31.68),
];

const HOT_ROLLED_PLATE: &[(&str, f64)] = &[
    ("3/16", 7.66),
    ("1/4", 10.21),
    ("9/32", 11.924813),
    ("5/16", 12.76),
    ("11/32", 13.476672),
    ("23/64", 14.29344),
    ("3/8", 15.31),
    ("7/16", 17.87),
    ("29/64", 18.581472),
    ("15/32", 18.867341),
    ("1/2", 20.4192),
    ("9/16", 22.97),
    ("5/8", 25.52),
    ("3/4", 30.63),
    ("13/16", 33.17),
    ("7/8", 35.73),
    ("1", 40.84),
    ("1-1/4", 51.05),
    ("1-3/8", 56.1),
    ("1-1/2", 61.26),
    ("1-3/4", 71.47),
    ("2", 81.68),
    ("2-1/2", 102.1),
    ("3", 122.52),
];

const STAINLESS: &[(&str, f64)] = &[
    ("12", 4.427),
    ("14", 3.154),
    ("16", 2.499),
    ("18", 2.016),
    ("20", 1.65),
    ("22", 1.30),
    ("24", 1.03),
    ("26", 0.82),
    ("28", 0.65),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f64, right: f64, tolerance: f64) -> bool {
        (left - right).abs() < tolerance
    }

    // ---- Mixed fraction parsing tests ----

    #[test]
    fn parse_fraction_to_float_simple_fraction() {
        assert!(approx_eq(
            parse_fraction_to_float("1/2").unwrap(),
            0.5,
            1e-9
        ));
        assert!(approx_eq(
            parse_fraction_to_float("3/4").unwrap(),
            0.75,
            1e-9
        ));
        assert!(approx_eq(
            parse_fraction_to_float("7/8").unwrap(),
            0.875,
            1e-9
        ));
    }

    #[test]
    fn parse_fraction_to_float_mixed_fractions() {
        assert!(approx_eq(
            parse_fraction_to_float("1-1/2").unwrap(),
            1.5,
            1e-9
        ));
        assert!(approx_eq(
            parse_fraction_to_float("1-1/4").unwrap(),
            1.25,
            1e-9
        ));
        assert!(approx_eq(
            parse_fraction_to_float("1-3/4").unwrap(),
            1.75,
            1e-9
        ));
        assert!(approx_eq(
            parse_fraction_to_float("1-3/8").unwrap(),
            1.375,
            1e-9
        ));
        assert!(approx_eq(
            parse_fraction_to_float("2-1/2").unwrap(),
            2.5,
            1e-9
        ));
    }

    #[test]
    fn parse_fraction_to_float_zero_denominator() {
        assert!(parse_fraction_to_float("1/0").is_none());
        assert!(parse_fraction_to_float("1-1/0").is_none());
    }

    #[test]
    fn parse_fraction_to_float_no_fraction() {
        assert!(parse_fraction_to_float("16").is_none());
        assert!(parse_fraction_to_float("1.5").is_none());
    }

    #[test]
    fn key_to_numeric_mixed_fraction() {
        let result = key_to_numeric("1-1/2");
        assert_eq!(result.kind, 1);
        assert!(approx_eq(result.value, 1.5, 1e-9));
    }

    #[test]
    fn key_to_numeric_mixed_fraction_1_3_4() {
        let result = key_to_numeric("1-3/4");
        assert_eq!(result.kind, 1);
        assert!(approx_eq(result.value, 1.75, 1e-9));
    }

    #[test]
    fn key_to_numeric_mixed_fraction_2_1_2() {
        let result = key_to_numeric("2-1/2");
        assert_eq!(result.kind, 1);
        assert!(approx_eq(result.value, 2.5, 1e-9));
    }

    #[test]
    fn key_to_numeric_simple_integer() {
        let result = key_to_numeric("16");
        assert_eq!(result.kind, 0);
        assert!(approx_eq(result.value, 16.0, 1e-9));
    }

    #[test]
    fn key_to_numeric_simple_fraction() {
        let result = key_to_numeric("3/8");
        assert_eq!(result.kind, 1);
        assert!(approx_eq(result.value, 0.375, 1e-9));
    }

    // ---- Mixed fraction sorting tests ----

    #[test]
    fn mixed_fractions_sort_numerically() {
        // Verify that mixed fraction keys sort numerically, not as strings
        let mut entries = vec![
            GaugeEntry {
                key: "2-1/2".to_string(),
                psf: 102.1,
            },
            GaugeEntry {
                key: "1-1/4".to_string(),
                psf: 51.05,
            },
            GaugeEntry {
                key: "1-3/4".to_string(),
                psf: 71.47,
            },
            GaugeEntry {
                key: "1-1/2".to_string(),
                psf: 61.26,
            },
            GaugeEntry {
                key: "3/4".to_string(),
                psf: 30.63,
            },
            GaugeEntry {
                key: "1".to_string(),
                psf: 40.84,
            },
        ];

        sort_entries(&mut entries);

        let sorted_keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        // Integers (kind=0) sort first, then fractions (kind=1) by numeric value.
        // "1" is an integer (kind=0), all others are fractions (kind=1).
        assert_eq!(
            sorted_keys,
            vec!["1", "3/4", "1-1/4", "1-1/2", "1-3/4", "2-1/2"]
        );

        // Verify that within kind=1, the fractional/mixed values are in numeric order
        let frac_keys: Vec<&str> = sorted_keys[1..].to_vec();
        assert_eq!(frac_keys, vec!["3/4", "1-1/4", "1-1/2", "1-3/4", "2-1/2"]);
    }

    // ---- HOT ROLLED PLATE table tests ----

    #[test]
    fn hot_rolled_plate_table_exists() {
        let tables = builtin_gauge_tables();
        assert!(
            tables.contains_key("HOT ROLLED PLATE"),
            "HOT ROLLED PLATE table must exist in builtins"
        );
    }

    #[test]
    fn hot_rolled_plate_has_all_override_entries() {
        let tables = builtin_gauge_tables();
        let table = tables.get("HOT ROLLED PLATE").unwrap();

        // All entries from the override JSON
        let expected: &[(&str, f64)] = &[
            ("1", 40.84),
            ("1-1/2", 61.26),
            ("1-1/4", 51.05),
            ("1-3/4", 71.47),
            ("1-3/8", 56.1),
            ("1/2", 20.4192),
            ("1/4", 10.21),
            ("11/32", 13.476672),
            ("13/16", 33.17),
            ("15/32", 18.867341),
            ("2", 81.68),
            ("2-1/2", 102.1),
            ("23/64", 14.29344),
            ("29/64", 18.581472),
            ("3", 122.52),
            ("3/16", 7.66),
            ("3/4", 30.63),
            ("3/8", 15.31),
            ("5/16", 12.76),
            ("5/8", 25.52),
            ("7/16", 17.87),
            ("7/8", 35.73),
            ("9/16", 22.97),
            ("9/32", 11.924813),
        ];

        for (key, expected_psf) in expected {
            let actual = table.get(key);
            assert!(actual.is_some(), "HOT ROLLED PLATE missing key '{key}'");
            assert!(
                approx_eq(actual.unwrap(), *expected_psf, 1e-6),
                "HOT ROLLED PLATE key '{key}': expected {expected_psf}, got {}",
                actual.unwrap()
            );
        }

        assert_eq!(
            table.entries().len(),
            expected.len(),
            "HOT ROLLED PLATE entry count mismatch"
        );
    }

    #[test]
    fn hot_rolled_plate_lookup_mixed_fraction_key() {
        let tables = builtin_gauge_tables();
        let result = get_psf(&tables, "HOT ROLLED PLATE", "1-1/2");
        assert_eq!(result.psf, Some(61.26));
        assert_eq!(result.used_key.as_deref(), Some("1-1/2"));
    }

    #[test]
    fn hot_rolled_plate_lookup_simple_fraction_key() {
        let tables = builtin_gauge_tables();
        let result = get_psf(&tables, "HOT ROLLED PLATE", "3/8");
        assert_eq!(result.psf, Some(15.31));
    }

    #[test]
    fn hot_rolled_plate_lookup_integer_key() {
        let tables = builtin_gauge_tables();
        let result = get_psf(&tables, "HOT ROLLED PLATE", "1");
        assert_eq!(result.psf, Some(40.84));
    }

    // ---- Per-table reconciliation tests ----

    /// Helper: load the override JSON and return it as a map of table_name → (key → psf)
    fn load_override_data(
    ) -> std::collections::HashMap<String, std::collections::HashMap<String, f64>> {
        let json_str = include_str!("../../../assets/gauge_tables.override.json");
        serde_json::from_str(json_str).unwrap()
    }

    #[test]
    fn reconcile_hr_hrpo_cr() {
        let tables = builtin_gauge_tables();
        let overrides = load_override_data();
        let override_data = overrides.get("HR/HRPO/CR").unwrap();
        let builtin = tables.get("HR/HRPO/CR").unwrap();

        for (key, &expected_psf) in override_data {
            let actual = builtin.get(key);
            assert!(
                actual.is_some(),
                "HR/HRPO/CR: builtin missing override key '{key}'"
            );
            assert!(
                approx_eq(actual.unwrap(), expected_psf, 1e-6),
                "HR/HRPO/CR key '{key}': expected {expected_psf}, got {}",
                actual.unwrap()
            );
        }
    }

    #[test]
    fn reconcile_galv_jk_bond() {
        let tables = builtin_gauge_tables();
        let overrides = load_override_data();
        let override_data = overrides.get("GALV/JK/BOND").unwrap();
        let builtin = tables.get("GALV/JK/BOND").unwrap();

        for (key, &expected_psf) in override_data {
            let actual = builtin.get(key);
            assert!(
                actual.is_some(),
                "GALV/JK/BOND: builtin missing override key '{key}'"
            );
            assert!(
                approx_eq(actual.unwrap(), expected_psf, 1e-6),
                "GALV/JK/BOND key '{key}': expected {expected_psf}, got {}",
                actual.unwrap()
            );
        }
    }

    #[test]
    fn reconcile_aluminized_with_corrected_override() {
        // The override "ALUMINUM" entry actually contains ALUMINIZED data
        // (see NOTE in ALUMINIZED const). We verify all its values match our builtin.
        let tables = builtin_gauge_tables();
        let overrides = load_override_data();
        let corrected_override = overrides.get("ALUMINUM").unwrap();
        let builtin = tables.get("ALUMINIZED").unwrap();

        for (key, &expected_psf) in corrected_override {
            let actual = builtin.get(key);
            assert!(
                actual.is_some(),
                "ALUMINIZED: builtin missing corrected-override key '{key}'"
            );
            assert!(
                approx_eq(actual.unwrap(), expected_psf, 1e-6),
                "ALUMINIZED key '{key}': expected {expected_psf}, got {}",
                actual.unwrap()
            );
        }
    }

    #[test]
    fn reconcile_aluminum_override_is_misattributed() {
        // The override "ALUMINUM" entry actually contains ALUMINIZED values.
        // Verify they do NOT match the real ALUMINUM builtin — confirming the
        // mislabeling issue has been identified and the ALUMINUM builtin is
        // intentionally left unchanged.
        let tables = builtin_gauge_tables();
        let overrides = load_override_data();
        let mislabeled = overrides.get("ALUMINUM").unwrap();
        let aluminum_builtin = tables.get("ALUMINUM").unwrap();

        // Check that at least some shared keys have different values,
        // confirming the override data is not appropriate for ALUMINUM.
        let mut mismatches = 0;
        for (key, &override_psf) in mislabeled {
            if let Some(builtin_psf) = aluminum_builtin.get(key) {
                if !approx_eq(builtin_psf, override_psf, 1e-6) {
                    mismatches += 1;
                }
            }
        }
        assert!(
            mismatches > 0,
            "Override 'ALUMINUM' data should NOT match real ALUMINUM builtin \
             (it contains ALUMINIZED data)"
        );
    }

    #[test]
    fn reconcile_stainless() {
        let tables = builtin_gauge_tables();
        let overrides = load_override_data();
        let override_data = overrides.get("STAINLESS").unwrap();
        let builtin = tables.get("STAINLESS").unwrap();

        for (key, &expected_psf) in override_data {
            let actual = builtin.get(key);
            assert!(
                actual.is_some(),
                "STAINLESS: builtin missing override key '{key}'"
            );
            assert!(
                approx_eq(actual.unwrap(), expected_psf, 1e-6),
                "STAINLESS key '{key}': expected {expected_psf}, got {}",
                actual.unwrap()
            );
        }
    }

    #[test]
    fn reconcile_hr_floor_plate() {
        let tables = builtin_gauge_tables();
        let overrides = load_override_data();
        let override_data = overrides.get("HR FLOOR PLATE").unwrap();
        let builtin = tables.get("HR FLOOR PLATE").unwrap();

        for (key, &expected_psf) in override_data {
            let actual = builtin.get(key);
            assert!(
                actual.is_some(),
                "HR FLOOR PLATE: builtin missing override key '{key}'"
            );
            assert!(
                approx_eq(actual.unwrap(), expected_psf, 1e-6),
                "HR FLOOR PLATE key '{key}': expected {expected_psf}, got {}",
                actual.unwrap()
            );
        }
    }

    #[test]
    fn reconcile_hot_rolled_plate() {
        let tables = builtin_gauge_tables();
        let overrides = load_override_data();
        let override_data = overrides.get("HOT ROLLED PLATE").unwrap();
        let builtin = tables.get("HOT ROLLED PLATE").unwrap();

        for (key, &expected_psf) in override_data {
            let actual = builtin.get(key);
            assert!(
                actual.is_some(),
                "HOT ROLLED PLATE: builtin missing override key '{key}'"
            );
            assert!(
                approx_eq(actual.unwrap(), expected_psf, 1e-6),
                "HOT ROLLED PLATE key '{key}': expected {expected_psf}, got {}",
                actual.unwrap()
            );
        }

        // Also verify all override entries are present (entry count >= override count)
        assert!(
            builtin.entries().len() >= override_data.len(),
            "HOT ROLLED PLATE should have at least as many entries as override"
        );
    }

    // ---- Verify ALUMINIZED override (labeled "ALUMINIZED") is not applied ----

    #[test]
    fn aluminized_override_entries_not_blindly_applied() {
        // The override "ALUMINIZED" has gauge 11: 1.746 and gauge 18: 0.713.
        // These values appear misattributed. Verify our builtin ALUMINIZED
        // does NOT have gauge 18 = 0.713 (we keep 2.077 from the corrected data).
        let tables = builtin_gauge_tables();
        let builtin = tables.get("ALUMINIZED").unwrap();

        // Gauge 18 should be 2.077 (from corrected ALUMINIZED data), not 0.713
        let psf_18 = builtin.get("18").unwrap();
        assert!(
            approx_eq(psf_18, 2.077, 1e-6),
            "ALUMINIZED gauge 18 should be 2.077 (corrected), not 0.713 (misattributed override)"
        );

        // Gauge 11 is NOT in our ALUMINIZED builtin (the 1.746 value from the
        // misattributed override is not applied)
        assert!(
            builtin.get("11").is_none(),
            "ALUMINIZED should NOT have gauge 11 from the misattributed override"
        );
    }

    // ---- Override loading tests ----

    #[test]
    fn load_override_tables_valid_json() {
        let dir = std::env::temp_dir().join("steelcal_test_valid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_override.json");
        let json = r#"{
            "TEST TABLE": {
                "10": 5.5,
                "12": 4.25
            }
        }"#;
        std::fs::write(&path, json).unwrap();

        let tables = load_override_tables(&path).unwrap();
        assert!(tables.contains_key("TEST TABLE"));
        let table = tables.get("TEST TABLE").unwrap();
        assert!(approx_eq(table.get("10").unwrap(), 5.5, 1e-9));
        assert!(approx_eq(table.get("12").unwrap(), 4.25, 1e-9));

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn load_override_tables_missing_file_returns_empty() {
        let path = std::env::temp_dir().join("steelcal_nonexistent_override.json");
        // Ensure the file does not exist
        std::fs::remove_file(&path).ok();

        let tables = load_override_tables(&path).unwrap();
        assert!(tables.is_empty(), "Missing file should return empty tables");
    }

    #[test]
    fn load_override_tables_invalid_json_returns_error() {
        let dir = std::env::temp_dir().join("steelcal_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("invalid_override.json");
        std::fs::write(&path, "{ not valid json !!").unwrap();

        let result = load_override_tables(&path);
        assert!(result.is_err(), "Invalid JSON should return an error");
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::errors::SteelCalError::Json(_)),
            "Error should be SteelCalError::Json variant, got: {err:?}"
        );

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn load_override_tables_empty_json_object() {
        let dir = std::env::temp_dir().join("steelcal_test_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty_override.json");
        std::fs::write(&path, "{}").unwrap();

        let tables = load_override_tables(&path).unwrap();
        assert!(
            tables.is_empty(),
            "Empty JSON object should produce empty tables"
        );

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn load_override_tables_reads_real_asset() {
        // Load the actual assets/gauge_tables.override.json
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/gauge_tables.override.json");
        let tables = load_override_tables(&path).unwrap();
        assert!(tables.contains_key("HR/HRPO/CR"));
        assert!(tables.contains_key("HOT ROLLED PLATE"));
        assert!(tables.contains_key("STAINLESS"));
    }

    // ---- Merge tests ----

    #[test]
    fn merge_tables_preserves_unmatched_builtin_keys() {
        let mut builtins = GaugeTables::new();
        builtins.insert(
            "TABLE_A".to_string(),
            GaugeTable::new(&[("10", 5.0), ("12", 4.0), ("14", 3.0)]),
        );

        let mut overrides = GaugeTables::new();
        overrides.insert("TABLE_A".to_string(), GaugeTable::new(&[("12", 4.5)]));

        merge_tables(&mut builtins, &overrides);

        let merged = builtins.get("TABLE_A").unwrap();
        // Key "10" should still be 5.0 (unmatched builtin preserved)
        assert!(approx_eq(merged.get("10").unwrap(), 5.0, 1e-9));
        // Key "12" should be 4.5 (override wins)
        assert!(approx_eq(merged.get("12").unwrap(), 4.5, 1e-9));
        // Key "14" should still be 3.0 (unmatched builtin preserved)
        assert!(approx_eq(merged.get("14").unwrap(), 3.0, 1e-9));
    }

    #[test]
    fn merge_tables_updates_existing_entries_override_wins() {
        let mut builtins = GaugeTables::new();
        builtins.insert(
            "TABLE_A".to_string(),
            GaugeTable::new(&[("10", 5.0), ("12", 4.0)]),
        );

        let mut overrides = GaugeTables::new();
        overrides.insert(
            "TABLE_A".to_string(),
            GaugeTable::new(&[("10", 5.5), ("12", 4.5)]),
        );

        merge_tables(&mut builtins, &overrides);

        let merged = builtins.get("TABLE_A").unwrap();
        assert!(approx_eq(merged.get("10").unwrap(), 5.5, 1e-9));
        assert!(approx_eq(merged.get("12").unwrap(), 4.5, 1e-9));
    }

    #[test]
    fn merge_tables_adds_new_entries_from_override() {
        let mut builtins = GaugeTables::new();
        builtins.insert("TABLE_A".to_string(), GaugeTable::new(&[("10", 5.0)]));

        let mut overrides = GaugeTables::new();
        overrides.insert(
            "TABLE_A".to_string(),
            GaugeTable::new(&[("12", 4.5), ("14", 3.5)]),
        );

        merge_tables(&mut builtins, &overrides);

        let merged = builtins.get("TABLE_A").unwrap();
        assert_eq!(merged.entries().len(), 3);
        assert!(approx_eq(merged.get("10").unwrap(), 5.0, 1e-9));
        assert!(approx_eq(merged.get("12").unwrap(), 4.5, 1e-9));
        assert!(approx_eq(merged.get("14").unwrap(), 3.5, 1e-9));
    }

    #[test]
    fn merge_tables_adds_new_table_from_override() {
        let mut builtins = GaugeTables::new();
        builtins.insert("TABLE_A".to_string(), GaugeTable::new(&[("10", 5.0)]));

        let mut overrides = GaugeTables::new();
        overrides.insert(
            "NEW_TABLE".to_string(),
            GaugeTable::new(&[("8", 7.0), ("10", 5.5)]),
        );

        merge_tables(&mut builtins, &overrides);

        // Original table preserved
        assert!(builtins.contains_key("TABLE_A"));
        assert!(approx_eq(
            builtins.get("TABLE_A").unwrap().get("10").unwrap(),
            5.0,
            1e-9
        ));

        // New table added from override
        assert!(builtins.contains_key("NEW_TABLE"));
        let new_table = builtins.get("NEW_TABLE").unwrap();
        assert!(approx_eq(new_table.get("8").unwrap(), 7.0, 1e-9));
        assert!(approx_eq(new_table.get("10").unwrap(), 5.5, 1e-9));
    }

    #[test]
    fn merge_tables_empty_override_does_nothing() {
        let mut builtins = GaugeTables::new();
        builtins.insert(
            "TABLE_A".to_string(),
            GaugeTable::new(&[("10", 5.0), ("12", 4.0)]),
        );

        let overrides = GaugeTables::new();
        merge_tables(&mut builtins, &overrides);

        let table = builtins.get("TABLE_A").unwrap();
        assert_eq!(table.entries().len(), 2);
        assert!(approx_eq(table.get("10").unwrap(), 5.0, 1e-9));
        assert!(approx_eq(table.get("12").unwrap(), 4.0, 1e-9));
    }

    #[test]
    fn merge_tables_entries_remain_sorted_after_merge() {
        let mut builtins = GaugeTables::new();
        builtins.insert(
            "TABLE_A".to_string(),
            GaugeTable::new(&[("10", 5.0), ("14", 3.0)]),
        );

        let mut overrides = GaugeTables::new();
        overrides.insert("TABLE_A".to_string(), GaugeTable::new(&[("12", 4.0)]));

        merge_tables(&mut builtins, &overrides);

        let merged = builtins.get("TABLE_A").unwrap();
        let keys: Vec<&str> = merged.entries().iter().map(|e| e.key.as_str()).collect();
        // Integer gauge keys should sort numerically: 10, 12, 14
        assert_eq!(keys, vec!["10", "12", "14"]);
    }

    #[test]
    fn merge_tables_hr_hrpo_cr_preserves_51_entries() {
        // Validates VAL-DATA-008: HR/HRPO/CR's 51 builtin entries are not
        // lost when the 22-entry override is applied.
        let mut builtins = builtin_gauge_tables();
        let orig_count = builtins.get("HR/HRPO/CR").unwrap().entries().len();

        let override_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/gauge_tables.override.json");
        let overrides = load_override_tables(&override_path).unwrap();

        merge_tables(&mut builtins, &overrides);

        let merged = builtins.get("HR/HRPO/CR").unwrap();
        // The merged table should have at least as many entries as the builtin
        // (all unmatched builtin keys are preserved)
        assert!(
            merged.entries().len() >= orig_count,
            "Merged HR/HRPO/CR should have at least {orig_count} entries, got {}",
            merged.entries().len()
        );
        // The original had 51 entries, override has a subset of those
        assert_eq!(
            merged.entries().len(),
            orig_count,
            "HR/HRPO/CR entry count should be unchanged (override is a subset of builtin)"
        );
    }
}
