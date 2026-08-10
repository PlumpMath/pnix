# 17 · unified explain — 값·순수성·안전성·진단·단면을 한 번에

## 쉽게 말하면 (비유)
**종합 건강검진 리포트**. 값·순수성·안전성·단면(mirror)·오류를 각각 재는 게 아니라 **한 장으로**
받는다.
```py
e = ph.explain_pnix("let a = 1; in a + 2")
e["ok"], e["purity"]["pure"], e["safe_eval"]["value"]   # True, True, 3
```
직관: 한 호출로 **값+순수성+안전성+진단** 통합 → 디버깅/서버 응답.

## 무엇을
한 소스에 대해 **purity + safe_eval(값) + mirror(단면) + diagnostic**을 하나의 레코드로 설명.

## plain의 한계 (`limit_python.py`)
Python에선 값(eval)·순수성·안전한계·진단·실행단면을 각각 다른 도구(ast/dis/inspect/try·except/
직접 만든 자원제한)로 조립해야 하고, 하나의 일관된 설명으로 묶이지 않는다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `explain_pnix(src, granted=...)` → `{ok, phase, purity, safe_eval, mirror, diagnostic}` 통합.
  순수 소스는 값/순수성/안전성이 한 번에, impure 소스는 `purity.pure=False` + `impure_uses`로.

## 어디에 쓰나
- DSL/설정 실행의 **통합 설명 패널**(값·안전성·부작용·오류를 한 응답으로)
- 감사/디버깅: 한 호출로 계산의 모든 관점 확보
- 서버 API: 사용자 입력에 대해 "결과 + 왜 거부됐는지"를 일관되게 반환

## 실행
```sh
python pnix-hy/examples/17-unified-explain/limit_python.py
python pnix-hy/examples/17-unified-explain/pnix_hy_way.py
```
