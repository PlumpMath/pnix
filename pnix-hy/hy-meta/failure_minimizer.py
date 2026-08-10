#!/usr/bin/env python3.11
"""Deterministically minimize Hy kernel failure probes."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from bootstrap import KERNEL_PATH, bootstrap_stage3_chain


def make_evaluator():
    _stage2, _stage2_prime, stage3 = bootstrap_stage3_chain()
    kernel = stage3.load_hy_file(KERNEL_PATH, "hy_meta_failure_minimizer.kernel")

    def evaluate(source: str) -> None:
        kernel.eval_source(source, None, "<hy-meta:minimized>")

    return evaluate


def still_fails(evaluate, source: str, expected: str | None, contains: str | None) -> bool:
    try:
        evaluate(source)
    except Exception as exc:
        if expected and exc.__class__.__name__ != expected:
            return False
        if contains and contains not in str(exc):
            return False
        return True
    return False


def minimize_lines(lines: list[str], evaluate, expected: str | None, contains: str | None) -> list[str]:
    current = lines[:]
    chunk = max(1, len(current) // 2)
    while chunk:
        changed = False
        index = 0
        while index < len(current):
            candidate = current[:index] + current[index + chunk :]
            if candidate and still_fails(evaluate, "".join(candidate), expected, contains):
                current = candidate
                changed = True
            else:
                index += chunk
        if not changed:
            chunk //= 2
    return current


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", nargs="?", default="", help="Hy source string")
    parser.add_argument("-f", "--file", help="read Hy source from a file")
    parser.add_argument("--expect", help="required exception class name")
    parser.add_argument("--contains", help="required substring in the exception")
    args = parser.parse_args(argv)

    source = Path(args.file).read_text() if args.file else args.source
    if not source:
        source = sys.stdin.read()
    evaluate = make_evaluator()
    if not still_fails(evaluate, source, args.expect, args.contains):
        print("input does not reproduce the requested failure", file=sys.stderr)
        return 2
    minimized = "".join(minimize_lines(source.splitlines(keepends=True), evaluate, args.expect, args.contains))
    print(minimized, end="" if minimized.endswith("\n") else "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
