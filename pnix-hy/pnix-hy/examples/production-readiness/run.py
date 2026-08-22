from pathlib import Path
import sys


REPO = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(REPO))

import pnix_hy as pnix  # noqa: E402


direct = pnix.eval_file(HERE / "direct.px")
consumer = pnix.eval_file(HERE / "consumer.px")
self_hosted = pnix.eval_file(HERE / "self_interpreter.px")
called_double = pnix.call_file(HERE / "library.px", "double", [21])
called_map_json = pnix.call_file_json(
    HERE / "library.px", "mapDouble", "[[1,2,3]]"
)

# Host import returns native Python dictionaries, lists, and integers.
assert isinstance(direct, dict)
assert direct == {"mode": "direct-runtime", "value": 42}
assert consumer == {
    "answer": 42,
    "mapped": [2, 4, 6],
    "count": 4,
    "total": 10,
    "version": "pnix-library-seed-v1",
}
assert self_hosted == {"mode": "pnix-in-pnix", "value": 42}
assert called_double == 42
assert called_map_json == "[2,4,6]"

# PNIX and hy-meta agree through the live meta-circular projection API.
projection = pnix.load_meta_api().pnix_meta_circular_projection("2 * 3 + 4")
assert projection["converged"] is True
assert set(projection["lanes"].values()) == {10}

print("PASS pnix-hy production-readiness")
