//! History model for tracking calculation sessions.
//!
//! Provides typed history entries, session storage, filtering, and stable
//! text export with a version header.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// HistoryEntryType — the kind of calculation recorded
// ---------------------------------------------------------------------------

/// Discriminates the type of calculation stored in a [`HistoryEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryEntryType {
    Sheet,
    Coil,
    Scrap,
    Pricing,
}

impl fmt::Display for HistoryEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sheet => write!(f, "Sheet"),
            Self::Coil => write!(f, "Coil"),
            Self::Scrap => write!(f, "Scrap"),
            Self::Pricing => write!(f, "Pricing"),
        }
    }
}

// ---------------------------------------------------------------------------
// HistoryEntry
// ---------------------------------------------------------------------------

/// A single history entry capturing one calculation's inputs and outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unix timestamp (seconds since epoch) when the entry was created.
    pub timestamp: u64,
    /// The kind of calculation.
    pub entry_type: HistoryEntryType,
    /// Serialised input values (structure depends on `entry_type`).
    pub inputs: serde_json::Value,
    /// Serialised output values (structure depends on `entry_type`).
    pub outputs: serde_json::Value,
}

impl HistoryEntry {
    /// Create a new history entry with the current wall-clock time.
    #[must_use]
    pub fn new(
        entry_type: HistoryEntryType,
        inputs: serde_json::Value,
        outputs: serde_json::Value,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            timestamp,
            entry_type,
            inputs,
            outputs,
        }
    }

    /// Create an entry with an explicit timestamp (useful for testing).
    #[must_use]
    pub fn with_timestamp(
        timestamp: u64,
        entry_type: HistoryEntryType,
        inputs: serde_json::Value,
        outputs: serde_json::Value,
    ) -> Self {
        Self {
            timestamp,
            entry_type,
            inputs,
            outputs,
        }
    }
}

impl fmt::Display for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (date, time) = format_timestamp(self.timestamp);
        write!(
            f,
            "[{date} {time}] {typ}\n  Inputs:  {inputs}\n  Outputs: {outputs}",
            typ = self.entry_type,
            inputs = self.inputs,
            outputs = self.outputs,
        )
    }
}

// ---------------------------------------------------------------------------
// SessionHistory
// ---------------------------------------------------------------------------

/// In-memory collection of [`HistoryEntry`] items for the current session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionHistory {
    entries: Vec<HistoryEntry>,
}

impl SessionHistory {
    /// Create an empty session history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new entry.
    pub fn add_entry(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
    }

    /// Return a slice of all entries (oldest first).
    #[must_use]
    pub fn get_entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Return entries whose type matches `entry_type`.
    #[must_use]
    pub fn filter_by_type(&self, entry_type: HistoryEntryType) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.entry_type == entry_type)
            .collect()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ---------------------------------------------------------------------------
// Text export
// ---------------------------------------------------------------------------

/// Export a slice of history entries to a stable, human-readable text format.
///
/// The output begins with a version header line, followed by a separator and
/// one block per entry.
#[must_use]
pub fn export_to_text(entries: &[HistoryEntry]) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut buf = String::new();
    buf.push_str(&format!("SteelCal History Export v{version}\n"));
    buf.push_str("========================================\n");

    if entries.is_empty() {
        buf.push_str("(no entries)\n");
        return buf;
    }

    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            buf.push_str("----------------------------------------\n");
        }
        let (date, time) = format_timestamp(entry.timestamp);
        buf.push_str(&format!(
            "Entry #{num}\n\
             Timestamp: {date} {time}\n\
             Type:      {typ}\n\
             Inputs:    {inputs}\n\
             Outputs:   {outputs}\n",
            num = i + 1,
            typ = entry.entry_type,
            inputs = entry.inputs,
            outputs = entry.outputs,
        ));
    }

    buf
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a Unix timestamp (seconds) into `("YYYY-MM-DD", "HH:MM:SS")` in
/// UTC.  We avoid pulling in `chrono` for this tiny utility.
#[must_use]
pub fn format_timestamp(ts: u64) -> (String, String) {
    // Days from UNIX epoch to each month start in a non-leap year (cumulative).
    const MONTH_DAYS: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

    let total_secs = ts;
    let secs_of_day = (total_secs % 86400) as u32;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    // Total days since 1970-01-01.
    let mut days = (total_secs / 86400) as u32;

    // Compute year.
    let mut year: u32 = 1970;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    // Compute month and day.
    let leap = is_leap(year);
    let mut month: u32 = 12;
    for m in (1..=12).rev() {
        let mut cum = MONTH_DAYS[(m - 1) as usize];
        if leap && m > 2 {
            cum += 1;
        }
        if days >= cum {
            month = m;
            days -= cum;
            break;
        }
    }
    let day = days + 1;

    (
        format!("{year:04}-{month:02}-{day:02}"),
        format!("{hours:02}:{minutes:02}:{seconds:02}"),
    )
}

fn is_leap(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- HistoryEntryType Display ----

    #[test]
    fn entry_type_display() {
        assert_eq!(HistoryEntryType::Sheet.to_string(), "Sheet");
        assert_eq!(HistoryEntryType::Coil.to_string(), "Coil");
        assert_eq!(HistoryEntryType::Scrap.to_string(), "Scrap");
        assert_eq!(HistoryEntryType::Pricing.to_string(), "Pricing");
    }

    // ---- HistoryEntry creation ----

    #[test]
    fn history_entry_new_captures_timestamp() {
        let entry = HistoryEntry::new(
            HistoryEntryType::Sheet,
            json!({"width": 48}),
            json!({"each_lb": 80.0}),
        );
        // Timestamp should be a recent Unix epoch (> 2020-01-01).
        assert!(entry.timestamp > 1_577_836_800);
        assert_eq!(entry.entry_type, HistoryEntryType::Sheet);
    }

    #[test]
    fn history_entry_with_explicit_timestamp() {
        let entry = HistoryEntry::with_timestamp(
            1_700_000_000,
            HistoryEntryType::Coil,
            json!({"width": 48}),
            json!({"footage": 200.0}),
        );
        assert_eq!(entry.timestamp, 1_700_000_000);
        assert_eq!(entry.entry_type, HistoryEntryType::Coil);
    }

    // ---- HistoryEntry Display ----

    #[test]
    fn history_entry_display_format() {
        let entry = HistoryEntry::with_timestamp(
            1_700_000_000,
            HistoryEntryType::Sheet,
            json!({"width": 48, "length": 120}),
            json!({"each_lb": 80.0}),
        );
        let output = format!("{entry}");
        assert!(output.contains("[2023-11-14 22:13:20] Sheet"));
        assert!(output.contains("Inputs:"));
        assert!(output.contains("Outputs:"));
    }

    // ---- SessionHistory ----

    #[test]
    fn session_history_add_and_get() {
        let mut history = SessionHistory::new();
        assert!(history.get_entries().is_empty());

        let e1 = HistoryEntry::with_timestamp(100, HistoryEntryType::Sheet, json!({}), json!({}));
        let e2 = HistoryEntry::with_timestamp(200, HistoryEntryType::Coil, json!({}), json!({}));
        history.add_entry(e1);
        history.add_entry(e2);

        assert_eq!(history.get_entries().len(), 2);
        assert_eq!(history.get_entries()[0].timestamp, 100);
        assert_eq!(history.get_entries()[1].timestamp, 200);
    }

    #[test]
    fn session_history_filter_by_type() {
        let mut history = SessionHistory::new();
        history.add_entry(HistoryEntry::with_timestamp(
            1,
            HistoryEntryType::Sheet,
            json!({}),
            json!({}),
        ));
        history.add_entry(HistoryEntry::with_timestamp(
            2,
            HistoryEntryType::Coil,
            json!({}),
            json!({}),
        ));
        history.add_entry(HistoryEntry::with_timestamp(
            3,
            HistoryEntryType::Sheet,
            json!({}),
            json!({}),
        ));
        history.add_entry(HistoryEntry::with_timestamp(
            4,
            HistoryEntryType::Scrap,
            json!({}),
            json!({}),
        ));
        history.add_entry(HistoryEntry::with_timestamp(
            5,
            HistoryEntryType::Pricing,
            json!({}),
            json!({}),
        ));

        let sheets = history.filter_by_type(HistoryEntryType::Sheet);
        assert_eq!(sheets.len(), 2);
        assert_eq!(sheets[0].timestamp, 1);
        assert_eq!(sheets[1].timestamp, 3);

        let coils = history.filter_by_type(HistoryEntryType::Coil);
        assert_eq!(coils.len(), 1);

        let scraps = history.filter_by_type(HistoryEntryType::Scrap);
        assert_eq!(scraps.len(), 1);

        let pricing = history.filter_by_type(HistoryEntryType::Pricing);
        assert_eq!(pricing.len(), 1);
    }

    #[test]
    fn session_history_filter_empty_result() {
        let mut history = SessionHistory::new();
        history.add_entry(HistoryEntry::with_timestamp(
            1,
            HistoryEntryType::Sheet,
            json!({}),
            json!({}),
        ));
        let coils = history.filter_by_type(HistoryEntryType::Coil);
        assert!(coils.is_empty());
    }

    #[test]
    fn session_history_clear() {
        let mut history = SessionHistory::new();
        history.add_entry(HistoryEntry::with_timestamp(
            1,
            HistoryEntryType::Sheet,
            json!({}),
            json!({}),
        ));
        history.add_entry(HistoryEntry::with_timestamp(
            2,
            HistoryEntryType::Coil,
            json!({}),
            json!({}),
        ));
        assert_eq!(history.get_entries().len(), 2);

        history.clear();
        assert!(history.get_entries().is_empty());
    }

    // ---- format_timestamp ----

    #[test]
    fn format_timestamp_epoch() {
        let (date, time) = format_timestamp(0);
        assert_eq!(date, "1970-01-01");
        assert_eq!(time, "00:00:00");
    }

    #[test]
    fn format_timestamp_known_date() {
        // 2023-11-14 22:13:20 UTC = 1700000000
        let (date, time) = format_timestamp(1_700_000_000);
        assert_eq!(date, "2023-11-14");
        assert_eq!(time, "22:13:20");
    }

    #[test]
    fn format_timestamp_leap_year() {
        // 2024-02-29 00:00:00 UTC = 1709164800
        let (date, time) = format_timestamp(1_709_164_800);
        assert_eq!(date, "2024-02-29");
        assert_eq!(time, "00:00:00");
    }

    #[test]
    fn format_timestamp_end_of_year() {
        // 2023-12-31 23:59:59 UTC = 1704067199
        let (date, time) = format_timestamp(1_704_067_199);
        assert_eq!(date, "2023-12-31");
        assert_eq!(time, "23:59:59");
    }

    // ---- export_to_text ----

    #[test]
    fn export_to_text_empty() {
        let output = export_to_text(&[]);
        assert!(output.starts_with("SteelCal History Export v"));
        assert!(output.contains("(no entries)"));
    }

    #[test]
    fn export_to_text_version_header() {
        let output = export_to_text(&[]);
        let first_line = output.lines().next().unwrap();
        assert!(
            first_line.starts_with("SteelCal History Export v"),
            "Expected version header, got: {first_line}"
        );
        // Version should match Cargo.toml workspace version.
        assert!(
            first_line.contains(env!("CARGO_PKG_VERSION")),
            "Version mismatch in header"
        );
    }

    // ---- Snapshot tests for export format ----

    #[test]
    fn export_to_text_snapshot_single_entry() {
        let entries = vec![HistoryEntry::with_timestamp(
            1_700_000_000,
            HistoryEntryType::Sheet,
            json!({"width": 48, "length": 120, "qty": 10, "gauge": "16"}),
            json!({"each_lb": 100.0, "total_lb": 1000.0, "psf": 2.5}),
        )];

        let output = export_to_text(&entries);
        let expected = format!(
            "\
SteelCal History Export v{version}\n\
========================================\n\
Entry #1\n\
Timestamp: 2023-11-14 22:13:20\n\
Type:      Sheet\n\
Inputs:    {{\"gauge\":\"16\",\"length\":120,\"qty\":10,\"width\":48}}\n\
Outputs:   {{\"each_lb\":100.0,\"psf\":2.5,\"total_lb\":1000.0}}\n",
            version = env!("CARGO_PKG_VERSION"),
        );

        assert_eq!(
            output, expected,
            "Snapshot mismatch:\n--- expected ---\n{expected}\n--- got ---\n{output}"
        );
    }

    #[test]
    fn export_to_text_snapshot_multiple_entries() {
        let entries = vec![
            HistoryEntry::with_timestamp(
                1_700_000_000,
                HistoryEntryType::Sheet,
                json!({"width": 48}),
                json!({"each_lb": 80.0}),
            ),
            HistoryEntry::with_timestamp(
                1_700_000_060,
                HistoryEntryType::Coil,
                json!({"width": 48, "thickness": 0.06}),
                json!({"footage": 204.08}),
            ),
            HistoryEntry::with_timestamp(
                1_700_000_120,
                HistoryEntryType::Scrap,
                json!({"actual": 5000, "ending": 4800}),
                json!({"scrap_lb": 200.0}),
            ),
            HistoryEntry::with_timestamp(
                1_700_000_180,
                HistoryEntryType::Pricing,
                json!({"mode": "per lb", "price": 1.0}),
                json!({"total_after_tax": 88.0}),
            ),
        ];

        let output = export_to_text(&entries);
        let expected = format!(
            "\
SteelCal History Export v{version}\n\
========================================\n\
Entry #1\n\
Timestamp: 2023-11-14 22:13:20\n\
Type:      Sheet\n\
Inputs:    {{\"width\":48}}\n\
Outputs:   {{\"each_lb\":80.0}}\n\
----------------------------------------\n\
Entry #2\n\
Timestamp: 2023-11-14 22:14:20\n\
Type:      Coil\n\
Inputs:    {{\"thickness\":0.06,\"width\":48}}\n\
Outputs:   {{\"footage\":204.08}}\n\
----------------------------------------\n\
Entry #3\n\
Timestamp: 2023-11-14 22:15:20\n\
Type:      Scrap\n\
Inputs:    {{\"actual\":5000,\"ending\":4800}}\n\
Outputs:   {{\"scrap_lb\":200.0}}\n\
----------------------------------------\n\
Entry #4\n\
Timestamp: 2023-11-14 22:16:20\n\
Type:      Pricing\n\
Inputs:    {{\"mode\":\"per lb\",\"price\":1.0}}\n\
Outputs:   {{\"total_after_tax\":88.0}}\n",
            version = env!("CARGO_PKG_VERSION"),
        );

        assert_eq!(
            output, expected,
            "Snapshot mismatch:\n--- expected ---\n{expected}\n--- got ---\n{output}"
        );
    }

    // ---- Serde round-trip ----

    #[test]
    fn history_entry_serde_roundtrip() {
        let entry = HistoryEntry::with_timestamp(
            1_700_000_000,
            HistoryEntryType::Sheet,
            json!({"width": 48}),
            json!({"each_lb": 80.0}),
        );
        let json_str = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json_str).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn session_history_serde_roundtrip() {
        let mut history = SessionHistory::new();
        history.add_entry(HistoryEntry::with_timestamp(
            1,
            HistoryEntryType::Sheet,
            json!({}),
            json!({}),
        ));
        history.add_entry(HistoryEntry::with_timestamp(
            2,
            HistoryEntryType::Coil,
            json!({"w": 48}),
            json!({"f": 200.0}),
        ));
        let json_str = serde_json::to_string(&history).unwrap();
        let deserialized: SessionHistory = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.get_entries().len(), 2);
        assert_eq!(deserialized.get_entries()[0].timestamp, 1);
        assert_eq!(
            deserialized.get_entries()[1].entry_type,
            HistoryEntryType::Coil
        );
    }

    // ---- HistoryEntryType serde ----

    #[test]
    fn entry_type_serde_roundtrip() {
        for typ in [
            HistoryEntryType::Sheet,
            HistoryEntryType::Coil,
            HistoryEntryType::Scrap,
            HistoryEntryType::Pricing,
        ] {
            let json_str = serde_json::to_string(&typ).unwrap();
            let deserialized: HistoryEntryType = serde_json::from_str(&json_str).unwrap();
            assert_eq!(typ, deserialized);
        }
    }
}
