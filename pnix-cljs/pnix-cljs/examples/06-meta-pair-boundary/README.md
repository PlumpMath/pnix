# 06 — meta 쌍 경계

## 무엇을

모노레포 이중 축:

| 절반 | 역할 |
|------|------|
| **pnix-cljs** (이 패키지) | pnix 런타임: parse/eval, Node export |
| **cljs-meta** (`pnix-cljs/cljs-meta/`) | 호스트 언어 meta / fixed-point 메커니즘 |

예제는 product 쪽만 보여 준다. meta 게이트는 `cljs-meta` README / bin 을 본다.

## 관련

- 모노레포 `README.md` — 다섯 호스트 쌍 표
- `pnix-cljs/cljs-meta/README.md`
