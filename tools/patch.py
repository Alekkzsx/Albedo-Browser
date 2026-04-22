#!/usr/bin/env python3
import os
import re
import sys
from pathlib import Path


HUNK_RE = re.compile(
    r"^@@ -(?P<old_start>\d+)(?:,(?P<old_count>\d+))? \+(?P<new_start>\d+)(?:,(?P<new_count>\d+))? @@"
)


class PatchError(Exception):
    pass


def strip_path(path: str, strip_components: int) -> str:
    parts = [part for part in path.split("/") if part not in ("", ".")]
    if strip_components <= 0 or len(parts) <= 1:
        return str(Path(*parts))
    if strip_components >= len(parts):
        return str(Path(*parts))
    return str(Path(*parts[strip_components:]))


def parse_args(argv: list[str]) -> int:
    strip_components = 0
    force = False
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "-f":
            force = True
        elif arg.startswith("-p"):
            value = arg[2:] or (argv[i + 1] if i + 1 < len(argv) else None)
            if value is None:
                raise PatchError("missing value for -p")
            strip_components = int(value)
            if arg == "-p":
                i += 1
        else:
            raise PatchError(f"unsupported argument: {arg}")
        i += 1
    return strip_components


def read_patch(stdin: str) -> list[dict]:
    lines = stdin.splitlines(keepends=True)
    files = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if not line.startswith("diff --git "):
            i += 1
            continue

        parts = line.strip().split()
        old_path = parts[2]
        new_path = parts[3]
        entry = {"old": old_path, "new": new_path, "hunks": []}
        i += 1

        while i < len(lines):
            line = lines[i]
            if line.startswith("diff --git "):
                break
            if line.startswith("--- "):
                entry["old"] = line[4:].strip()
                i += 1
                if i < len(lines) and lines[i].startswith("+++ "):
                    entry["new"] = lines[i][4:].strip()
                    i += 1
                continue
            if line.startswith("@@ "):
                match = HUNK_RE.match(line)
                if not match:
                    raise PatchError(f"invalid hunk header: {line.rstrip()}")
                hunk = {
                    "header": line,
                    "old_start": int(match.group("old_start")),
                    "lines": [],
                }
                i += 1
                while i < len(lines):
                    next_line = lines[i]
                    if next_line.startswith(("diff --git ", "@@ ", "--- ")):
                        break
                    if next_line.startswith(("\\ No newline at end of file",)):
                        i += 1
                        continue
                    if next_line[:1] not in {" ", "+", "-"}:
                        raise PatchError(f"invalid hunk line: {next_line.rstrip()}")
                    hunk["lines"].append(next_line)
                    i += 1
                entry["hunks"].append(hunk)
                continue
            i += 1

        files.append(entry)
    return files


def apply_hunks(content: str, hunks: list[dict], file_path: str) -> str:
    src_lines = content.splitlines(keepends=True)
    out_lines: list[str] = []
    src_index = 0

    for hunk in hunks:
        target_index = max(hunk["old_start"] - 1, 0)
        while src_index < target_index:
            out_lines.append(src_lines[src_index])
            src_index += 1

        for line in hunk["lines"]:
            prefix = line[0]
            payload = line[1:]
            if prefix == " ":
                if src_index >= len(src_lines) or src_lines[src_index] != payload:
                    raise PatchError(
                        f"context mismatch in {file_path} near {hunk['header'].rstrip()}"
                    )
                out_lines.append(src_lines[src_index])
                src_index += 1
            elif prefix == "-":
                if src_index >= len(src_lines) or src_lines[src_index] != payload:
                    raise PatchError(
                        f"delete mismatch in {file_path} near {hunk['header'].rstrip()}"
                    )
                src_index += 1
            elif prefix == "+":
                out_lines.append(payload)
            else:
                raise PatchError(f"unsupported hunk prefix {prefix!r}")

    out_lines.extend(src_lines[src_index:])
    return "".join(out_lines)


def main() -> int:
    try:
        strip_components = parse_args(sys.argv[1:])
        patch_text = sys.stdin.read()
        files = read_patch(patch_text)
        for entry in files:
            old_path = entry["old"].removeprefix("a/")
            new_path = entry["new"].removeprefix("b/")
            rel_path = strip_path(new_path, strip_components)
            file_path = Path.cwd() / rel_path
            if not file_path.exists():
                rel_path = strip_path(old_path, strip_components)
                file_path = Path.cwd() / rel_path
            original = file_path.read_text(encoding="utf-8", errors="surrogateescape")
            updated = apply_hunks(original, entry["hunks"], str(file_path))
            file_path.write_text(updated, encoding="utf-8", errors="surrogateescape")
        return 0
    except Exception as exc:
        print(f"patch.py: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
