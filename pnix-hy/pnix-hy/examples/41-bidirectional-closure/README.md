# 41. 양방향 투영 닫힘 — pnix↔Hy 왕복이 정말 닫히는가

## 무엇을
세 함수로 pnix↔Hy 왕복의 **닫힘(closure)**을 본다:

- `synthesize_pnix_from_hy` — Hy 소스에서 pnix 소스를 합성(값 대신 **소스
  텍스트** 수준의 역-투영; `08-hy-reader-embed-pnix`/`07`의 반대 방향)
- `pnix_projection_closure` — pnix → Hy → pnix 왕복(involution): 값 보존 +
  두 번째 pnix 소스가 **비교 가능**하고 **닫힘**
- `hy_projection_closure` — 반대 방향, Hy → pnix → Hy 왕복

## 왜
`16-meaning-preservation-roundtrip`이 "의미가 보존되는가"의 상태어휘를
다룬다면, 이건 그보다 좁고 구체적인 질문이다 — "왕복한 **소스 자체**가
안정적으로 닫히는가(더 왕복해도 안 변하는가)". plain Python에는 애초에
두 언어 사이를 오가는 소스 합성/왕복 개념이 없다.

## 무엇을 게이트하나
| 함수 | 확인 |
|---|---|
| `synthesize_pnix_from_hy` | Hy→pnix 소스 합성 + `synthesizable`/`gaps` |
| `pnix_projection_closure` | pnix→Hy→pnix, `comparable`+`closed` |
| `hy_projection_closure` | Hy→pnix→Hy, `comparable`+`closed` |

## 한 줄
> `1 + 2`(pnix)와 `(+ 1 2)`(Hy) 양쪽에서 출발해도, 서로를 거쳐 되돌아온
> 소스가 다시 비교 가능하고 닫혀 있다 — 왕복이 발산하지 않는다.

## 경계
- 값 보존은 `16`이 이미 다룬다. 여기는 **소스 표현의 왕복 안정성** 자체에
  집중한다.
