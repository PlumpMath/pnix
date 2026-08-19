#!/usr/bin/env python3
"""Check assigned docs contain Hangul (U+AC00–U+D7A3). Run: python3 _hangul_check.py"""
from pathlib import Path

ROOT = Path("/Users/gp/pnix/pnix-clr")
files = [
    ROOT / "pnix-clr/docs/IMPLEMENTATION.md",
    ROOT / "pnix-clr/docs/TODO.md",
    ROOT / "pnix-clr/docs/BUGS.md",
    ROOT / "pnix-clr/docs/PLANS.md",
    ROOT / "clr-meta/README.md",
    ROOT / "clr-meta/RESIDUAL_SURFACE.md",
    ROOT / "clr-meta/CLR_BOOTSTRAP.md",
    ROOT / "clr-meta/SELF_REPRODUCTION_DESIGN.md",
    ROOT / "clr-meta/INDEPENDENT_MINI_INTERPRETER_DESIGN.md",
    ROOT / "clr-meta/STAGE3_DESIGN.md",
    ROOT / "clr-meta/STAGE4_DESIGN.md",
    ROOT / "clr-meta/STAGE5_DESIGN.md",
    ROOT / "clr-meta/STAGE6_DESIGN.md",
    ROOT / "clr-meta/STAGE7_DESIGN.md",
    ROOT / "clr-meta/STAGE8_DESIGN.md",
    ROOT / "clr-meta/STAGE9_DESIGN.md",
    ROOT / "clr-meta/STAGE10_DESIGN.md",
    ROOT / "clr-meta/STAGE11_DESIGN.md",
    ROOT / "clr-meta/STAGE12_DESIGN.md",
    ROOT / "clr-meta/STAGE13_DESIGN.md",
    ROOT / "clr-meta/STAGE14_DESIGN.md",
    ROOT / "clr-meta/STAGE15_DESIGN.md",
    ROOT / "clr-meta/STAGEN_DESIGN.md",
    ROOT / "clr-meta/STAGE15_N_ROADMAP.md",
    ROOT / "clr-meta/STATUS.md",
    ROOT / "clr-meta/todo.md",
    ROOT / "AGENTS.md",
]

ok, bad = [], []
for p in files:
    if not p.exists():
        bad.append((str(p), "MISSING"))
        continue
    text = p.read_text(encoding="utf-8")
    count = sum(1 for ch in text if 0xAC00 <= ord(ch) <= 0xD7A3)
    if count > 0:
        ok.append((str(p), count))
    else:
        bad.append((str(p), "NO_HANGUL"))

print("=== PASS (Hangul present) ===")
for path, n in ok:
    print(f"  {n:5d} chars  {path}")
print(f"\nPASS count: {len(ok)}")
print("\n=== FAIL ===")
if not bad:
    print("  (none)")
else:
    for path, reason in bad:
        print(f"  {reason:12s}  {path}")
print(f"\nFAIL count: {len(bad)}")
raise SystemExit(0 if not bad else 1)
