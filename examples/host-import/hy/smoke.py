#!/usr/bin/env python3
"""최소 host-main import: pnix_hy 를 import 하고 eval_file 호출."""
from __future__ import annotations

from pathlib import Path

import pnix_hy as ph


def main() -> None:
    px = Path(__file__).resolve().parent.parent / "hello.px"
    print(ph.eval_file(str(px)))


if __name__ == "__main__":
    main()
