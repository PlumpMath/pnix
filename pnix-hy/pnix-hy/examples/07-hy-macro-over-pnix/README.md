# 07 · Hy macro / quasiquote over pnix — 두 meta-circular 잇기

> Hy 1.3.0 proof Python 필요 — `nix develop` 안에서, 또는 `PNIX_HY_PYTHON`을 설정하고 실행.

## 쉽게 말하면 (비유)
**통역사**가 상대 언어의 관용구(매크로)를 내 문장에 얹어 준다. pnix엔 관용구가 없지만, Hy의
관용구를 pnix 문장 위에 적용해 쓸 수 있다 — pnix는 그대로(비동형) 두면서.
```py
ph.hy_macro_over_pnix("1 + 2")["pnix_of_expansion"]   # (if true then (1 + 2) else null)
ph.hy_quasiquote_over_pnix("`(sum ~a ~b)", {"a": "1 + 2", "b": "10"})  # (sum 3 10)
```
직관: 두 meta-circular(파이썬 생태계 ↔ pnix)의 **언어기능을 연결**한다.

## 무엇을
pnix를 **비동형(non-homoiconic)으로 유지**한 채, Hy의 실제 매크로/quasiquote를 **pnix 코드/값 위에**
적용한다. (pnix에 매크로를 만드는 것이 아니다.)

## plain의 한계 (`limit_python.py`)
Python엔 1급 매크로가 없고(함수는 인자를 선평가), pnix는 설계상 quote/macro가 없다 →
**한쪽 언어만으로는** "pnix 코드 위에 매크로 적용"이 불가능.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `hy_macro_over_pnix(src)` — pnix식 → Hy 폼 투영 → Hy 매크로(`when`) 적용 → 확장(→`if`),
  확장을 다시 pnix로 재합성.
- `hy_quasiquote_over_pnix(template, holes)` — pnix **값**을 Hy quasiquote 구멍에 주입해 폼 생성.
- `quasiquote_specialize_correspondence(...)` — "quasiquote=수동 staging vs specialize_pnix=자동
  staging" 대응을 실행가능하게 검증.

## 어디에 쓰나
- 순수 설정/DSL(pnix) 위에 **호스트 매크로 기반 코드생성**을 얹고 싶을 때
- 두 meta-circular(Python-ecosystem ↔ pnix) 사이 언어기능 연구/브리징
- staging(정적 골격 + 동적 구멍) 구조를 두 언어에서 대응시켜 관찰

## 실행
```sh
nix develop
python pnix-hy/examples/07-hy-macro-over-pnix/limit_python.py
python pnix-hy/examples/07-hy-macro-over-pnix/pnix_hy_way.py
```
