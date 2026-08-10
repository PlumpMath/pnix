from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import pnix_hy as pnix  # noqa: E402


meta = pnix.load_meta_api()
projection = meta.pnix_meta_circular_projection("2 * 3 + 4")

print("lanes:", projection["lanes"])
print("converged:", projection["converged"])

assert projection["converged"] is True
assert set(projection["lanes"].values()) == {10}

# Proof/service modules are not imported as prerequisites for this mechanism.
assert "action" not in meta.__dict__
assert "deploy" not in meta.__dict__
