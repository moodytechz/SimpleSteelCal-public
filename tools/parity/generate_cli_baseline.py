from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PYTHON_APP = ROOT / "UlimateSteelCal_20250823_0054.py"
OUTPUT = ROOT / "fixtures" / "parity" / "cli-baseline.json"

CASES = [
    {
        "name": "sheet-gauge",
        "args": [
            "--width", "48",
            "--length", "96",
            "--qty", "10",
            "--gauge", "16",
            "--table", "HR/HRPO/CR",
            "--price-mode", "per lb",
            "--price", "0.6",
            "--markup", "15",
            "--tax", "6",
            "--setup-fee", "25",
        ],
    },
    {
        "name": "sheet-psf",
        "args": [
            "--width", "48",
            "--length", "96",
            "--qty", "3",
            "--psf", "2.5",
            "--price-mode", "per ft\u00b2",
            "--price", "2.0",
        ],
    },
    {
        "name": "sheet-thickness",
        "args": [
            "--width", "48",
            "--length", "96",
            "--qty", "1",
            "--thickness", "0.25",
            "--density", "490",
        ],
    },
    {
        "name": "coil",
        "args": [
            "--width", "48",
            "--length", "96",
            "--qty", "1",
            "--gauge", "16",
            "--table", "HR/HRPO/CR",
            "--coil-width", "48",
            "--coil-thickness", "0.06",
            "--coil-id", "20",
            "--coil-weight", "2000",
        ],
    },
]


def run_case(args: list[str]) -> dict[str, object]:
    process = subprocess.run(
        [sys.executable, str(PYTHON_APP), *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    return {
        "args": args,
        "returncode": process.returncode,
        "stdout": process.stdout.strip(),
        "stderr": process.stderr.strip(),
    }


def main() -> None:
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    results = [{"name": case["name"], **run_case(case["args"])} for case in CASES]
    OUTPUT.write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(f"Wrote {len(results)} parity cases to {OUTPUT}")


if __name__ == "__main__":
    main()
