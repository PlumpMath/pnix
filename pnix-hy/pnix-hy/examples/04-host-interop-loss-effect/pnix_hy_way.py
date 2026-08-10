"""pnix-hy의 방식 — 모든 경계 넘김에 loss/effect/capability를 명시 기록한다.

from_host / to_host는 값을 변환하면서 InteropRecord(손실상태·effect클래스·필요권한·witness)를
함께 낸다. host 콜러블은 pnix 항으로 '직접' 들어오지 않고 opaque ref가 되며, 호출은 host-call
권한으로 게이트된다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


# 1) 변환 손실이 '표시'된다: tuple -> pnix list 는 lossy로 기록.
_, rec = ph.from_host((1, 2, 3))
print("tuple 변환 loss:", rec.loss_status, "| effect:", rec.effect_class)
assert rec.loss_status == "lossy"

# 왕복 fidelity를 한 곳에서 본다.
rt = ph.roundtrip_host_value((1, 2, 3))
print("roundtrip:", "loss=", rt["loss_status"], "equal=", rt["equal"])
assert rt["loss_status"] == "lossy" and rt["equal"] is False

# 2) host 콜러블은 opaque ref + 'host-call 권한 필요'로 기록된다.
ref, crec = ph.from_host(len)
print("콜러블 변환:", "opaque?", ph.is_interop_error(ref) is False and "__" in str(ref),
      "| 필요권한:", crec.capability_required)
assert crec.capability_required == "host-call"

# 3) 호출은 권한 게이트를 통과해야 한다.
ok = ph.try_call_host(len, ([1, 2, 3],))                 # host-call 허가(기본)
denied = ph.try_call_host(len, ([1, 2, 3],), granted=())  # 권한 없음
print("호출(허가):", ok["success"], ok["value"], "| 호출(무권한):", denied["success"], denied["error"]["kind"])
assert ok["success"] is True and ok["value"] == 3
assert denied["success"] is False and denied["error"]["kind"] == "denied"

print("\n결론: 경계 넘김의 손실/부작용/권한이 값에 기록돼 '조용한 위험'이 사라진다.")
