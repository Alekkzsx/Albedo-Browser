#!/usr/bin/env python3
import json
import math
import pathlib
import sys


def load_json(path: pathlib.Path):
    with path.open("r", encoding="utf-8") as f:
        return json.load(f)


def fail(msg: str):
    print(f"[FAIL] {msg}")
    return False


def ok(msg: str):
    print(f"[ OK ] {msg}")
    return True


def main():
    root = pathlib.Path(__file__).resolve().parents[1]
    super_path = root / "ace_html_super_benchmark_results.json"
    mem_path = root / "ace_html_memory_profile_results.json"
    thresholds_path = root / "tools" / "ace_html_quality_thresholds.json"

    if len(sys.argv) >= 2:
        super_path = pathlib.Path(sys.argv[1]).resolve()
    if len(sys.argv) >= 3:
        mem_path = pathlib.Path(sys.argv[2]).resolve()
    if len(sys.argv) >= 4:
        thresholds_path = pathlib.Path(sys.argv[3]).resolve()

    super_report = load_json(super_path)
    mem_report = load_json(mem_path)
    thresholds = load_json(thresholds_path)

    ace = super_report.get("ace_html", {})
    required = ace.get("requiredTree", {})
    html5lib = ace.get("html5libTreeFull", {})
    tokenizer = ace.get("tokenizerSubset", {})
    perf = ace.get("performance", {})
    alloc = mem_report.get("allocatorStats", {})

    checks = []
    checks.append(
        (
            required.get("passRate", 0.0) >= thresholds["min_required_tree_pass_rate"],
            f"requiredTree passRate={required.get('passRate', 0.0):.2f} (min {thresholds['min_required_tree_pass_rate']:.2f})",
        )
    )
    checks.append(
        (
            html5lib.get("passRate", 0.0) >= thresholds["min_html5lib_tree_pass_rate"],
            f"html5libTreeFull passRate={html5lib.get('passRate', 0.0):.2f} (min {thresholds['min_html5lib_tree_pass_rate']:.2f})",
        )
    )
    checks.append(
        (
            tokenizer.get("passRate", 0.0) >= thresholds["min_tokenizer_pass_rate"],
            f"tokenizerSubset passRate={tokenizer.get('passRate', 0.0):.2f} (min {thresholds['min_tokenizer_pass_rate']:.2f})",
        )
    )
    checks.append(
        (
            perf.get("throughputMbps", 0.0) >= thresholds["min_throughput_mbps"],
            f"throughputMbps={perf.get('throughputMbps', 0.0):.3f} (min {thresholds['min_throughput_mbps']:.3f})",
        )
    )
    checks.append(
        (
            perf.get("avgMs", math.inf) <= thresholds["max_avg_ms"],
            f"avgMs={perf.get('avgMs', math.inf):.3f} (max {thresholds['max_avg_ms']:.3f})",
        )
    )
    checks.append(
        (
            alloc.get("avgAllocatedBytesPerCase", math.inf)
            <= thresholds["max_allocated_bytes_per_case"],
            f"avgAllocatedBytesPerCase={alloc.get('avgAllocatedBytesPerCase', math.inf):.1f} (max {thresholds['max_allocated_bytes_per_case']:.1f})",
        )
    )

    all_ok = True
    for passed, message in checks:
        if passed:
            ok(message)
        else:
            all_ok = False
            fail(message)

    if not all_ok:
        print("\nQuality gate failed.")
        sys.exit(1)

    print("\nQuality gate passed.")


if __name__ == "__main__":
    main()
