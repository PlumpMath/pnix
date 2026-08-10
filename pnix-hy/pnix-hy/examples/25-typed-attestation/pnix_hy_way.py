"""pnix-hy 방식: predicate-typed attestation (proposal 0024).

`typed_witness(predicate_uri, payload)`는 증거에 **predicate 타입 URI**를 부여하고 payload를 그 타입에
묶는다(in-toto/SLSA 스타일). 그래서
- 증거가 무엇에 대한 것인지(action/eval/interop/realisation…) URI로 명시되고,
- `is_known_predicate`로 알려진 형식인지 판별하며,
- `migrate_predicate`로 deprecated URI를 최신으로 이관한다.
공유 witness envelope는 그대로 — predicate/payload는 payload 레벨에 얹힌다(불변식 유지).
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
from pnix_hy import gate

# 알려진 predicate URI
eval_uri = gate.PREDICATE_TYPES["eval"]
print("eval predicate:", eval_uri)
assert gate.is_known_predicate(eval_uri) is True
assert gate.is_known_predicate("https://example.com/made-up") is False

# 타입이 부여된 witness — payload가 predicate에 묶이고, 안정 sha256이 찍힌다
w = gate.typed_witness(eval_uri, {"in_hash": "h1", "out_hash": "h2", "status": "lossless"})
print("typed witness id:", w["witness_id"][:16], "…")
print("payload._predicate_type:", w["payload"]["_predicate_type"])
assert w["payload"]["_predicate_type"] == eval_uri
assert gate.predicate_of(w) == eval_uri            # 증거→predicate 역조회

# deprecated URI는 최신으로 이관
old = next(iter(gate.DEPRECATED_PREDICATES))
migrated = gate.migrate_predicate(old)
print(f"migrate: {old.rsplit('/',2)[-2:]} → {migrated.rsplit('/',2)[-2:]}")
assert migrated == gate.DEPRECATED_PREDICATES[old] and gate.is_known_predicate(migrated)

assert gate.typed_witness_report()["ready"]
print("→ 증거에 predicate 타입을 부여: 무엇에 대한 것인지·유효한지·최신인지 판별 가능.")
