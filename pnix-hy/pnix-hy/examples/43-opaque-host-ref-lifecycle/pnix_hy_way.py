"""pnix-hy 방식: opaque 호스트 참조 생명주기 (make_opaque_ref 등).

host 객체를 SES식 opaque 참조로 감싸 공개 메서드만 허용하고, 빌림에
스코프를 두고, 표면을 동결할 수 있다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph


class Greeter:
    def hello(self, name):
        return f"hello {name}"

    def _secret(self):
        return "should not be reachable"


ref = ph.make_opaque_ref(Greeter())
allowed = ph.opaque_allowed_methods(ref)
print(f"opaque_allowed_methods: {allowed}")
assert "hello" in allowed and "_secret" not in allowed

value, record = ph.opaque_call_method(ref, "hello", ["world"])
print(f"opaque_call_method: value={value!r} effect_class={record.effect_class!r}")
assert value == "hello world"

info = ph.inspect_opaque(ref)
print(f"inspect_opaque: type={info['type']} callable={info['callable']}")
assert info["type"] == "Greeter"

before = ph.opaque_lifecycle()["lends_active"]
with ph.lend_opaque(ph.make_opaque_ref(Greeter())):
    during = ph.opaque_lifecycle()["lends_active"]
after = ph.opaque_lifecycle()["lends_active"]
print(f"lend scope: before={before} during={during} after={after}")
assert during == before + 1 and after == before

hardened = ph.harden_opaque(ph.make_opaque_ref(Greeter()))
print(f"harden_opaque: surface_sha256={hardened['surface_sha256'][:16]}...")
assert len(hardened["surface_sha256"]) == 64

invariants = ph.declare_opaque_invariants(ph.make_opaque_ref(Greeter()), ["hello"])
print(f"declare_opaque_invariants: frozen_attrs={invariants['frozen_attrs']}")
assert invariants["frozen_attrs"] == ["hello"]

print("→ _secret은 안 보이고, 빌림은 스코프가 있고, 표면은 동결·불변 선언까지 된다.")
