# 05 · witness & gate — 검증 가능한 영수증과 권한 게이트

## 쉽게 말하면 (비유)
계산마다 **위조 불가 영수증(witness)**을 발급하고, **출입증(gate)**이 없으면 문 앞에서 막는다.
```py
ph.make_witness("eval", {"value": 21, "source": "x"})["sha256"]   # 키 순서 무관, 결정적
ph.gate_check('builtins.readFile "/x"')["allowed"]                # False (file-read 권한 필요)
```
직관: **재현 가능한 증거** + **권한 있어야만 부작용** → 감사/보안.

## 무엇을
모든 중요한 연산에 **결정적 내용해시 witness**를 남기고, 부작용을 **capability gate**로 통제한다.

## plain의 한계 (`limit_python.py`)
`eval`은 "이 입력→이 출력"의 검증 가능한 영수증을 남기지 않고, 키 순서가 다른 같은 의미의
데이터를 안정적으로 동일 취급하지 못하며, "이 계산은 file-read가 필요"라는 권한 게이트가 없다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `make_witness(kind, payload)` — 결정적 sha256 영수증(키 순서 무관, 재현 가능).
- `gate_check(src, granted=...)` — 계산이 요구하는 effect(`file-read`/`subprocess`/`network`/…)를
  분류하고, 허가된 권한일 때만 `allowed=True`.

## 어디에 쓰나
- 감사(auditable)/규정준수 로그: 연산마다 안정적 증거 해시
- 신뢰 경계에서 부작용을 **명시적으로 허가**해야 통과시키는 실행 정책
- 재현 검증: 두 실행이 같은 witness면 같은 입력/환경/결과임을 증명

## 실행
```sh
python pnix-hy/examples/05-witness-and-gate/limit_python.py
python pnix-hy/examples/05-witness-and-gate/pnix_hy_way.py
```
