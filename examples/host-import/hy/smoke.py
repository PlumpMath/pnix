#!/usr/bin/env python3
"""Minimal host-main import: import pnix_hy + eval_file."""
from __future__ import annotations

from pathlib import Path

import pnix_hy as ph


def main() -> None:
    px = Path(__file__).resolve().parent.parent / "hello.px"
    print(ph.eval_file(str(px)))


if __name__ == "__main__":
    main()
