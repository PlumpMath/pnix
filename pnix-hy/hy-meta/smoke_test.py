#!/usr/bin/env python3
"""Smoke test entry point for the Hy meta-circular bootstrap."""

from bootstrap import main


if __name__ == "__main__":
    for command in (
        ["chain-check"],
        ["kernel-check"],
        ["direct-kernel-bridge-check"],
        ["kernel-import-check"],
        ["prime-check"],
        ["stage3-check"],
        ["mirror-check"],
        ["stage7-check"],
        ["self-host-check"],
        ["bootstrap-fixedpoint-check"],
        ["diverse-double-compile-check"],
        ["independent-mini-backend-check"],
        ["no-fallback-check"],
        ["parity-ledger-check"],
        ["stage8-check"],
        ["stage9-check"],
        ["stage10-check"],
        ["stage11-check"],
        ["stage12-check"],
        ["stage13-check"],
        ["stage14-check"],
        ["stage14-import-check"],
        ["stage14-edn-check"],
        ["stage15-check"],
        ["stagen-check"],
        ["stage16-check"],
        ["version-ast-coverage-check"],
        ["source-position-check"],
        ["ast-forward-compat-check"],
        ["macro-require-parity-check"],
        ["reader-boundary-check"],
        ["front-end-boundary-check"],
        ["compatibility-boundary-check"],
        ["cli-io-check"],
        ["hyc-check"],
        ["repl-check"],
        ["startup-output-check"],
        ["run", "(+ 20 22)"],
        ["run", "-c", "(+ 20 22)"],
        ["run", "-f", "hy-meta/examples/shebang.hy"],
        ["py", "(+ 20 22)"],
        ["py", "-c", "(+ 20 22)"],
        ["hy2py", "-c", "(+ 20 22)"],
        ["hy2py", "-f", "hy-meta/examples/shebang.hy"],
        ["kernel-run", "(+ 20 22)"],
        ["kernel-run", "-c", "(+ 20 22)"],
        ["kernel-py", "(+ 20 22)"],
        ["stage7-kernel-run", "(+ 20 22)"],
        ["stage7-kernel-py", "(+ 20 22)"],
        ["native-subset-check"],
    ):
        status = main(command)
        if status:
            raise SystemExit(status)
