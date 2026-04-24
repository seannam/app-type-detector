#!/usr/bin/env python3
"""
Reusable, automatable test that runs app-type-detector against every project
listed in ~/Developer/__versioning_projects/versioning_scan.json and compares
the detector's output against the expected values defined in
scripts/detector-scan-expectations.json.

Modes:
  (default)   Run as a test. Exit 0 if no project fails its expectations, 1
              otherwise. Prints a table + summary.
  --report    Print the table only, always exit 0.
  --update    Overwrite detector-scan-expectations.json's project_overrides
              with whatever the detector currently returns (turning the
              current state into the new baseline). Useful after a deliberate
              rule change.
  --csv       Emit machine-readable CSV rather than a formatted table.
  --limit N   Only evaluate the first N projects (smoke test).
  --jobs N    Run detector calls in parallel (default: 1).

Exit codes:
  0  Every non-skipped project met its expectations (or had no expectations).
  1  One or more projects failed.
  2  Setup error (binary missing, scan JSON missing, etc.).
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any, Iterable

REPO_ROOT = Path(__file__).resolve().parent.parent
HOME = Path.home()
SCAN_JSON_DEFAULT = HOME / "Developer/__versioning_projects/versioning_scan.json"
EXPECTATIONS_DEFAULT = REPO_ROOT / "scripts/detector-scan-expectations.json"
BINARY_DEFAULT = REPO_ROOT / "app/target/release/app-type-detector"

FAIL, PASS, SKIP, NOEXPECT, NA = "FAIL", "PASS", "SKIP", "-----", "N/A"


def build_detector() -> Path:
    print("building app-type-detector-cli (release) ...", file=sys.stderr)
    subprocess.run(
        ["cargo", "build", "--release", "-p", "app-type-detector-cli", "--quiet"],
        cwd=REPO_ROOT / "app",
        check=True,
    )
    if not BINARY_DEFAULT.exists():
        sys.exit(f"built binary not found at {BINARY_DEFAULT}")
    return BINARY_DEFAULT


def run_detector(bin_path: Path, project_path: Path) -> dict:
    try:
        result = subprocess.run(
            [str(bin_path), "detect", str(project_path), "--format", "json"],
            capture_output=True,
            text=True,
            timeout=30,
        )
    except subprocess.TimeoutExpired:
        return {"__error__": "timeout"}
    if result.returncode != 0:
        tail = result.stderr.strip().splitlines()[-1][:80] if result.stderr else "err"
        return {"__error__": tail}
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as e:
        return {"__error__": f"bad json: {e}"}


def primary_language(report: dict) -> str | None:
    langs = report.get("tech_stack", {}).get("languages", {}) or {}
    return langs.get("primary")


def primary_framework(report: dict) -> str | None:
    ts = report.get("tech_stack", {})
    for top, key in [
        ("web", "backend_frameworks"),
        ("web", "frontend_frameworks"),
        ("game", "engines"),
        ("desktop", "shells"),
        ("mobile", "ui_frameworks"),
    ]:
        node = ts.get(top) or {}
        arr = node.get(key) if isinstance(node, dict) else None
        if arr:
            return arr[0]
    frameworks = ts.get("frameworks") or []
    return frameworks[0] if frameworks else None


def primary_app_type(report: dict) -> str | None:
    return report.get("app_type", {}).get("primary")


def expected_value(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(v) for v in value]
    return [str(value)]


def check_field(actual: str | None, expected: Any) -> bool:
    acceptable = expected_value(expected)
    if not acceptable:
        return True  # no expectation
    return actual is not None and actual in acceptable


def resolve_expectations(
    label: str, detected_stack: str, cfg: dict
) -> tuple[dict, bool]:
    """Return (expectations, is_skip)."""
    if label in cfg.get("skip_projects", []):
        return {}, True
    overrides = cfg.get("project_overrides", {}).get(label)
    if overrides is not None:
        return overrides, False
    stack_map = cfg.get("stack_expectations", {})
    return stack_map.get(detected_stack, {}), False


def evaluate(
    proj: dict, cfg: dict, bin_path: Path
) -> dict:
    label = proj["label"]
    path = Path(proj["path"])
    detected_stack = proj.get("detected_stack", "unknown")
    expectations, is_skip = resolve_expectations(label, detected_stack, cfg)

    row = {
        "label": label,
        "stack": detected_stack,
        "mode": proj.get("recommended_mode", "-"),
        "expected_app_type": ",".join(expected_value(expectations.get("app_type"))) or "-",
        "expected_language": ",".join(expected_value(expectations.get("language"))) or "-",
        "app_type": "-",
        "language": "-",
        "framework": "-",
        "status": SKIP if is_skip else NOEXPECT,
        "reasons": [],
    }

    if not path.exists():
        # Missing directory → not a detector regression, just "no code present".
        row["status"] = SKIP if is_skip else NA
        row["reasons"].append("directory missing")
        return row

    report = run_detector(bin_path, path)
    if "__error__" in report:
        row["status"] = FAIL if not is_skip else SKIP
        row["reasons"].append(f"detector error: {report['__error__']}")
        return row

    row["app_type"] = primary_app_type(report) or "-"
    row["language"] = primary_language(report) or "-"
    row["framework"] = primary_framework(report) or "-"

    rules_fired = (report.get("scorecard", {}) or {}).get("rules_fired", 0) or 0
    no_signal = (
        rules_fired == 0
        and primary_app_type(report) is None
        and primary_language(report) is None
    )
    if no_signal:
        row["status"] = SKIP if is_skip else NA
        row["reasons"].append("no code signal (rules_fired=0, no language, no app_type)")
        return row

    if is_skip or not expectations:
        return row

    ok_app = check_field(primary_app_type(report), expectations.get("app_type"))
    ok_lang = check_field(primary_language(report), expectations.get("language"))
    if ok_app and ok_lang:
        row["status"] = PASS
    else:
        row["status"] = FAIL
        if not ok_app:
            row["reasons"].append(
                f"app_type: expected {expectations.get('app_type')!r}, got {row['app_type']!r}"
            )
        if not ok_lang:
            row["reasons"].append(
                f"language: expected {expectations.get('language')!r}, got {row['language']!r}"
            )
    return row


def fmt_table(rows: list[dict]) -> str:
    headers_flat = [
        "directory", "stack", "mode",
        "exp.app_type", "exp.language",
        "app_type", "language", "framework", "status",
    ]
    values = [
        [
            r["label"], r["stack"], r["mode"],
            r["expected_app_type"], r["expected_language"],
            r["app_type"], r["language"], r["framework"], r["status"],
        ]
        for r in rows
    ]
    widths = [
        max(len(str(cell)) for cell in [h, *(row[i] for row in values)])
        for i, h in enumerate(headers_flat)
    ]

    def fmt_row(cells: Iterable[str]) -> str:
        return "| " + " | ".join(c.ljust(w) for c, w in zip(cells, widths)) + " |"

    # Two-row header: group 1 = directory metadata, group 2 = expected, group 3 = actual.
    groups = [
        ("", ["directory", "stack", "mode"]),
        ("expected", ["exp.app_type", "exp.language"]),
        ("actual",   ["app_type", "language", "framework"]),
        ("", ["status"]),
    ]
    top_cells, i = [], 0
    for title, subs in groups:
        span = sum(widths[i:i + len(subs)]) + (len(subs) - 1) * 3
        top_cells.append(title.center(span))
        i += len(subs)
    out = ["| " + " | ".join(top_cells) + " |", fmt_row(headers_flat),
           "|-" + "-|-".join("-" * w for w in widths) + "-|"]
    out.extend(fmt_row(row) for row in values)
    return "\n".join(out)


def update_overrides(rows: list[dict], cfg_path: Path) -> None:
    cfg = json.loads(cfg_path.read_text())
    overrides = cfg.get("project_overrides", {})
    for r in rows:
        if r["app_type"] == "-" and r["language"] == "-":
            continue
        entry: dict[str, Any] = {}
        if r["app_type"] != "-":
            entry["app_type"] = r["app_type"]
        if r["language"] != "-":
            entry["language"] = r["language"]
        overrides[r["label"]] = entry
    cfg["project_overrides"] = overrides
    cfg_path.write_text(json.dumps(cfg, indent=2) + "\n")
    print(f"wrote {len(overrides)} project overrides to {cfg_path}", file=sys.stderr)


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--scan-json", type=Path, default=SCAN_JSON_DEFAULT)
    p.add_argument("--expectations", type=Path, default=EXPECTATIONS_DEFAULT)
    p.add_argument("--report", action="store_true", help="Print table and always exit 0.")
    p.add_argument("--update", action="store_true", help="Overwrite project_overrides with current state.")
    p.add_argument("--csv", action="store_true", help="Emit CSV instead of a table.")
    p.add_argument("--json-out", type=Path, help="Write per-project JSON results to this path.")
    p.add_argument("--limit", type=int, default=0)
    p.add_argument("--jobs", type=int, default=1)
    args = p.parse_args()

    if not args.scan_json.exists():
        print(f"error: scan json not found at {args.scan_json}. "
              "run /version:scan first.", file=sys.stderr)
        return 2
    cfg = json.loads(args.expectations.read_text()) if args.expectations.exists() else {}
    scan = json.loads(args.scan_json.read_text())
    projects = scan["projects"]
    if args.limit > 0:
        projects = projects[: args.limit]

    bin_path = build_detector()

    rows: list[dict] = [None] * len(projects)  # type: ignore[list-item]
    if args.jobs > 1:
        with ThreadPoolExecutor(max_workers=args.jobs) as ex:
            fut_to_idx = {
                ex.submit(evaluate, proj, cfg, bin_path): i
                for i, proj in enumerate(projects)
            }
            for fut in as_completed(fut_to_idx):
                idx = fut_to_idx[fut]
                rows[idx] = fut.result()
                print(f"[{sum(r is not None for r in rows)}/{len(projects)}] "
                      f"{rows[idx]['label']} {rows[idx]['status']}", file=sys.stderr)
    else:
        for i, proj in enumerate(projects, 1):
            print(f"[{i}/{len(projects)}] {proj['label']} ...", file=sys.stderr)
            rows[i - 1] = evaluate(proj, cfg, bin_path)

    if args.csv:
        w = csv.writer(sys.stdout)
        w.writerow(["directory", "stack", "mode", "exp.app_type", "exp.language",
                    "app_type", "language", "framework", "status", "reasons"])
        for r in rows:
            w.writerow([r["label"], r["stack"], r["mode"],
                        r["expected_app_type"], r["expected_language"],
                        r["app_type"], r["language"], r["framework"], r["status"],
                        "; ".join(r["reasons"])])
    else:
        print(fmt_table(rows))

    if args.json_out:
        args.json_out.write_text(json.dumps(rows, indent=2) + "\n")

    if args.update:
        update_overrides(rows, args.expectations)

    totals = {PASS: 0, FAIL: 0, SKIP: 0, NOEXPECT: 0, NA: 0}
    for r in rows:
        totals[r["status"]] += 1
    print(f"\nSummary: {totals[PASS]} PASS · {totals[FAIL]} FAIL · "
          f"{totals[SKIP]} SKIP · {totals[NA]} N/A · "
          f"{totals[NOEXPECT]} no-expectation (of {len(rows)} total)",
          file=sys.stderr)

    failures = [r for r in rows if r["status"] == FAIL]
    if failures and not args.report:
        print("\nFailures:", file=sys.stderr)
        for r in failures:
            print(f"  - {r['label']} [{r['stack']}]", file=sys.stderr)
            for reason in r["reasons"]:
                print(f"      {reason}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
