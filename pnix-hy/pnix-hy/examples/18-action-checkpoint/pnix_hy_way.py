"""pnix-hy의 방식 — action checkpoint: 한 pnix step을 판정표로 고정한다.

check_action은 새 evaluator를 만들지 않는다. 기존 gate/safe_eval/mirror/explain/witness를
얇게 묶어 accepted / held / rejected verdict를 만든다. rollback은 파일 백업이 아니라 해시 참조다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


accepted = ph.check_action("let a = 1; in a + 2")
print("accepted:", accepted["status"], "| effects:", accepted["effects"])
print("  ir_hash:", accepted["ir_hash"][:16], "| value_hash:", accepted["value_hash"][:16])
print("  witness:", accepted["witness_id"])
assert accepted["status"] == "accepted"

# 파일 읽기 계열 action은 기본으로 실행하지 않고 held가 된다.
held = ph.check_action('builtins.pathExists "/etc/passwd"')
print("\nheld:", held["status"], "| effects:", held["effects"])
assert held["status"] == "held"
assert held["effects"] == ["file-read"]

# 명시 권한을 주면 같은 action이 통과한다.
granted = ph.check_action('builtins.pathExists "/etc/passwd"', granted=("file-read",))
print("granted:", granted["status"], "| value_hash:", granted["value_hash"][:16])
assert granted["status"] == "accepted"

rejected = ph.check_action("1 +")
print("\nrejected:", rejected["status"], "| phase:", rejected["phase"])
assert rejected["status"] == "rejected"

verified = ph.verify_action(
    "1 + 2",
    intent="agent proposed a tiny arithmetic step",
    before_snapshot={"workspace": "clean"},
)
print("\nverify:", verified["status"])
print("  action_id:", verified["action"]["action_id"])
print("  rollback_ref:", verified["action"]["rollback_ref"])
print("  witness:", verified["witness_id"])

print("\n결론: 한 행동을 값·효과·증거·rollback hash ref가 붙은 verdict로 고정한다.")
