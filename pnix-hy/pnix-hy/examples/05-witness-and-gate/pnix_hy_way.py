"""pnix-hy의 방식 — 내용해시 witness + effect capability gate.

make_witness는 (kind, payload)에서 결정적 내용해시 영수증을 만든다(키 순서 무관). gate_check는
계산이 요구하는 effect를 분류하고, 허가된 권한일 때만 통과시킨다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


# 1) witness: 같은 내용이면 키 순서가 달라도 같은 해시(결정적, 검증 가능한 영수증).
w1 = ph.make_witness("eval", {"value": 21, "source": "x"})
w2 = ph.make_witness("eval", {"source": "x", "value": 21})
print("witness id:", w1["witness_id"], "| 결정적(키순서 무관):", w1["sha256"] == w2["sha256"])
print("sha256 길이:", len(w1["sha256"]))
assert w1["sha256"] == w2["sha256"] and len(w1["sha256"]) == 64

# 2) capability gate: 순수 계산은 무권한 통과, 부작용은 필요 effect가 있어야 통과.
pure = ph.gate_check("let a = 1; in a + a")
print("순수 계산 게이트:", pure["allowed"], "| 필요 effect:", pure["required_effects"])
assert pure["allowed"] is True and pure["required_effects"] == []

denied = ph.gate_check('builtins.readFile "/etc/passwd"')
granted = ph.gate_check('builtins.readFile "/etc/passwd"', granted=("file-read",))
print("readFile(무권한):", denied["allowed"], denied["required_effects"],
      "| (file-read 허가):", granted["allowed"])
assert denied["allowed"] is False and denied["required_effects"] == ["file-read"]
assert granted["allowed"] is True

# exec/network 등도 각자의 effect로 분류된다.
print("exec effect:", ph.gate_check("builtins.exec")["required_effects"],
      "| getFlake effect:", ph.gate_check("builtins.getFlake")["required_effects"])

print("\n결론: 모든 계산이 검증 가능한 영수증을 남기고, 권한이 있어야만 부작용을 수행한다.")
