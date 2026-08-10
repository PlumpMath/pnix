"""pnix-hy 방식: 감쇠·중단·회수 가능한 능력 핸들 (proposal 0020/0016, SES 스타일).

`grant_capability(*effects)`는 effect-class 집합에 대한 **런타임 회수 가능** 핸들을 준다.
- `attenuate(*drop)` : 권한을 **뺀** 자식 핸들(최소권한; 상위로 못 올라감),
- `suspend()`/`resume()` : 일시 정지/복구,
- `revoke()` : 영구 회수(이후 resume 불가).
현재 유효 권한은 `effective()`.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
import pnix_hy.interop as iop

# 전권 부여
full = iop.grant_capability("read", "write", "exec")
print("grant(read,write,exec):", sorted(full.effective()))
assert full.effective() == {"read", "write", "exec"}

# 최소권한: exec를 뺀 자식 핸들 (플러그인엔 이것만 준다)
least = full.attenuate("exec")
print("attenuate('exec') → 자식:", sorted(least.effective()))
assert least.effective() == {"read", "write"}

# 자식은 상위 권한으로 escalate 못 함 (감쇠만 가능)
escalated = least.attenuate("read")            # 더 줄이기만 됨
print("자식을 더 감쇠:", sorted(escalated.effective()))
assert escalated.effective() == {"write"}

# 일시 중단/복구
h = iop.grant_capability("read", "write")
h.suspend(); print("suspend →", sorted(h.effective()))
assert h.effective() == set()
h.resume();  print("resume  →", sorted(h.effective()))
assert h.effective() == {"read", "write"}

# 영구 회수: 이후 resume도 막힘
h.revoke(); print("revoke  →", sorted(h.effective()))
assert h.effective() == set()
try:
    h.resume()
    raise AssertionError("revoke 후 resume이 통과하면 안 됨")
except iop.InteropError as e:
    print("revoke 후 resume 차단:", type(e).__name__)

assert iop.interop_hardening_report()["ready"]
print("→ 최소권한(감쇠) + 중단/복구 + 영구 회수 = 넘겨도 전권이 아니다.")
