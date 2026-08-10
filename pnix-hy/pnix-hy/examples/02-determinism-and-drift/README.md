# 02 · determinism & drift — 정본 해시와 차이 분류

## 쉽게 말하면 (비유)
`hash()`는 만날 때마다 새로 붙이는 **임시 스티커**(실행마다 값이 바뀜). IR 해시는 코드의
**지문(fingerprint)** — 언제 찍어도 같다. drift는 "어디가 달라졌나"를 알려주는 진단서.
```py
ph.ir_of("let a=1; in a+2")["ir_sha256"] == ph.ir_of("let  a=1 ;  in a+2")["ir_sha256"]  # True
```
직관: 포맷이 달라도 같은 뜻이면 같은 지문 → 재현·캐시·회귀검출.

## 무엇을
코드/값에 **재현 가능한 정본(canonical) 내용주소 해시**를 부여하고, 차이를 **drift로 분류**한다.

## plain의 한계 (`limit_python.py`)
`hash()`는 `PYTHONHASHSEED`로 실행마다 바뀌고(무염 불가), 코드의 정본 해시가 표준에 없으며,
"무엇이 어떻게 달라졌는지" 분류하는 표준 도구가 없다 → **재현성/감사에 불리.**

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `ir_of(src)["ir_sha256"]` — 정규화 IR의 안정 해시. 같은 소스 → 항상 같은 해시(결정성),
  공백/포맷 차이는 정본으로 수렴.
- `classify_drift(src)` — pnix→Hy 프로젝션의 차이를 안정적 카테고리(`no-hy-operator` /
  `no-projection-construct` / …)로 분류.

## 어디에 쓰나
- 내용주소 **캐시 키**(같은 입력=같은 해시), **재현 가능 빌드/평가**
- 회귀/변경 감지: 결과가 달라졌을 때 "메타데이터 vs 의미 vs 구성" 중 무엇이 바뀐지 분류
- 감사(auditable) 로그: 실행마다 안정적인 증거 해시

## 실행
```sh
python pnix-hy/examples/02-determinism-and-drift/limit_python.py
python pnix-hy/examples/02-determinism-and-drift/pnix_hy_way.py
```
