from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import pnix_hy as pnix  # noqa: E402


lossless = pnix.roundtrip_host_value({"answer": 42, "enabled": True})
lossy = pnix.roundtrip_host_value((1, 2, 3))

print("lossless host roundtrip:", lossless)
print("explicit lossy host roundtrip:", lossy)

assert lossless["equal"] is True
assert lossy["loss_status"] == "lossy"

# These records describe value crossing. They do not turn Python strings into
# protocol type witnesses; structural type authority remains in pnix-meta.
