from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import pandas as pd

TABLE_ALIASES = {
    "HR/HRPO/CR/EG": "HR/HRPO/CR",
    "HR Floor Plate": "HR FLOOR PLATE",
    "HDP (Mill Plate)": "HR FLOOR PLATE",
}

NAME_MAP = {
    "CRS": "HR/HRPO/CR",
    "HRS": "HR/HRPO/CR",
    "GALVS": "GALV/JK/BOND",
    "ALUM": "ALUMINIZED",
    "AL1": "ALUMINUM",
    "HDP": "HR FLOOR PLATE",
    "HRP": "HOT ROLLED PLATE",
    "STAIN": "STAINLESS",
}

IGNORED_TYPES = {"A-40", "A-60", "BOND"}


def normalize_table_name(table_name: str) -> str:
    trimmed = str(table_name).strip()
    return TABLE_ALIASES.get(trimmed, trimmed)


def parse_fraction_to_float(value: str) -> float | None:
    trimmed = value.strip()
    if "/" not in trimmed:
        return None
    try:
        numerator, denominator = trimmed.split("/", 1)
        return float(numerator) / float(denominator)
    except Exception:
        return None


def key_sort(value: str) -> tuple[int, float, str]:
    trimmed = value.strip().lower().replace(" inch", "")
    try:
        parsed = float(trimmed)
        if parsed == int(parsed):
            return (0, parsed, value)
        return (1, parsed, value)
    except ValueError:
        pass

    fraction = parse_fraction_to_float(trimmed)
    if fraction is not None:
        return (1, fraction, value)

    return (2, float("inf"), value)


def normalize_key(value: Any) -> str:
    text = str(value).strip()
    try:
        integer = int(float(text))
        if float(text) == float(integer):
            return str(integer)
    except Exception:
        pass
    return text


def convert_tables(input_path: Path) -> dict[str, dict[str, float]]:
    frame = pd.read_excel(input_path, sheet_name=0)
    required = {"Material_Type", "Gauge", "Unit_Weight"}
    if not required.issubset(set(frame.columns)):
        raise ValueError(f"Workbook must contain columns: {sorted(required)}")

    material_frame = frame.loc[
        frame["Material_Type"].notna(), ["Material_Type", "Gauge", "Unit_Weight"]
    ].copy()
    material_frame["Material_Type"] = material_frame["Material_Type"].astype(str).str.strip()

    tables: dict[str, dict[str, float]] = {}
    for label in sorted(material_frame["Material_Type"].unique()):
        if label in IGNORED_TYPES:
            continue

        normalized_name = normalize_table_name(NAME_MAP.get(label, label))
        subset = material_frame.loc[
            material_frame["Material_Type"] == label, ["Gauge", "Unit_Weight"]
        ].dropna()
        if subset.empty:
            continue

        entries = tables.setdefault(normalized_name, {})
        for _, row in subset.iterrows():
            entries[normalize_key(row["Gauge"])] = float(row["Unit_Weight"])

    normalized_tables: dict[str, dict[str, float]] = {}
    for table_name, entries in tables.items():
        ordered_keys = sorted(entries.keys(), key=key_sort)
        normalized_tables[table_name] = {
            key: entries[key] for key in ordered_keys
        }

    return normalized_tables


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Convert SteelCal workbook gauge tables into normalized JSON."
    )
    parser.add_argument(
        "--input",
        default="lbs_ft_table.xlsx",
        help="Path to the workbook to convert.",
    )
    parser.add_argument(
        "--output",
        default="assets/gauge_tables.override.json",
        help="Path to the normalized JSON output.",
    )
    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    converted = convert_tables(input_path)
    output_path.write_text(json.dumps(converted, indent=2), encoding="utf-8")
    print(f"Wrote {len(converted)} tables to {output_path}")


if __name__ == "__main__":
    main()
