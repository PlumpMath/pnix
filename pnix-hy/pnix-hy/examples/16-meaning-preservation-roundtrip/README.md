# 16 · meaning-preservation roundtrip — 번역의 의미 보존 증명

> Hy 1.3.0 proof Python 필요 (`nix develop` / `PNIX_HY_PYTHON`).

## 쉽게 말하면 (비유)
**역번역 검수(back-translation)**. 번역(Hy→pnix) 후 양쪽을 평가해 **뜻(값)이 같은지** 확인하고,
손실 정도를 `lossless / lossy-ok / held / rejected` 등급으로 매긴다.
```py
pm.hy_to_pnix_value_roundtrip("(+ 1 2)")["meaning_preserved"]   # True (값 3 == 3)
```
직관: **번역이 의미를 보존했는지 값으로 증명** + 표준 상태 어휘로 분류.

## 무엇을
표현을 번역(Hy→pnix)했을 때 **의미(값)가 보존되는지 증명**하고, 왕복 결과를 **공유 상태 어휘**로 분류.

## plain의 한계 (`limit_python.py`)
Python엔 "번역 전후가 같은 의미인가"를 검사하고 상태(lossless/lossy/…)로 분류하는 표준이 없다 →
직접 양쪽을 eval해 비교하고 어휘도 스스로 정해야 한다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `pnix_mirror.hy_to_pnix_value_roundtrip(hy)` — Hy→합성 pnix, 양쪽 평가 후 값 일치
  (`meaning_preserved`) 판정.
- `roundtrip_status(src)` + `ROUNDTRIP_STATUS_VOCAB` = `('lossless','lossy-ok','held','rejected')` —
  여러 프로젝션 왕복을 **한 상태 어휘**로 분류.

## 어디에 쓰나
- 언어/표현 번역·마이그레이션에서 **의미 보존을 자동 증명**(회귀검출)
- 손실 허용 정책: 결과를 lossless/lossy-ok/held/rejected로 분류해 처리 분기
- 최적화/재작성이 값을 바꾸지 않았음을 파이프라인에서 검증

## 실행
```sh
nix develop
python pnix-hy/examples/16-meaning-preservation-roundtrip/pnix_hy_way.py
```
