from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import pnix_hy as pnix  # noqa: E402


answer = pnix.eval_source("let x = 20; in x + 22")
record = pnix.eval_source('{ answer = 42; status = "held"; }')

print("answer:", answer)
print("guest held-shaped value:", record)

assert answer == 42
assert isinstance(record, dict)
assert record["status"] == "held"
