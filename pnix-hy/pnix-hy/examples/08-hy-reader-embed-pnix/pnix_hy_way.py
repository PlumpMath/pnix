"""pnix-hy의 방식 — Hy의 reader macro `#px`로 pnix를 '읽는 시점'에 임베드한다.

Hy 리더에 `#px` 매크로를 달아, `#px "<pnix>"`를 읽는 순간 `(pnix-eval "<pnix>")` 폼으로 승격한다.
그 다음 pnix-hy가 임베드된 pnix에 '의미'를 준다 — 평가하고, Hy 폼으로 투영한다. (Hy의 리더 기계를
쓰는 것이지 pnix에 reader macro를 만드는 것이 아니다.)

* Hy 1.3.0 proof Python 필요 (`nix develop` 또는 PNIX_HY_PYTHON).
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


# `#px "1 + 2"` 가 read-time에 (pnix-eval "1 + 2") 로 승격되고, pnix-hy가 값/폼을 준다.
r = ph.hy_reader_embed_pnix('(+ 10 #px "1 + 2")')
emb = r["embeddings"][0]
print("임베드된 pnix:", repr(emb["pnix_source"]))
print("pnix 값:", emb["pnix_value"], "| Hy 폼으로 투영:", emb["hy_form"])
assert emb["pnix_source"] == "1 + 2" and emb["pnix_value"] == 3 and emb["hy_form"] == "(+ 1 2)"

# 여러 개 임베드도 된다.
r2 = ph.hy_reader_embed_pnix('[#px "1 + 2" #px "{ a = 1; }"]')
print("멀티 임베드:", [(e["pnix_source"], e["pnix_value"]) for e in r2["embeddings"]])
assert r2["embedding_count"] == 2

print("\n결론: read-time에 다른 언어(pnix)를 1급 폼으로 임베드하고 의미를 부여한다.")
