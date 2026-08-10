# 01 · pure sandbox — 신뢰 가능한 평가

## 쉽게 말하면 (비유)
`eval()`은 낯선 사람에게 **집 열쇠를 통째로** 주는 것이다. `safe_eval`은 **유리벽 면회실** —
정해진 것만, 시간제한을 두고, 부작용 없이 만나게 한다.
```py
ph.safe_eval('builtins.getEnv "HOME"', pure_only=True)   # -> ok:False, limit_exceeded:"impure"
```
직관: 신뢰 못 할 코드를 "부작용 금지 + 자원 상한 + 권한 필요"로 가둬서 실행한다.

## 무엇을
신뢰할 수 없는 코드를 **부작용 없이, 자원 한계 안에서, 권한 제어와 함께** 평가한다.

## plain의 한계 (`limit_python.py`)
Python `eval()`/`exec()`는 (1) 부작용을 막지 못하고, (2) 무한 루프/자원 소모를 막지 못하며,
(3) 실행 전에 순수성을 정적으로 판정할 수 없다 → **신뢰할 수 없는 입력에 안전하지 않다.**

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `safe_eval(src)` — 스텝/시간/출력 한계 강제, 절대 걸리거나 예외로 새지 않음(구조화 판정 반환)
- `safe_eval(src, pure_only=True)` — 부작용은 `limit_exceeded="impure"`로 **실행 전 거부**
- `static_purity_check(src)` — 실행 '전에' 순수/부작용 정적 분류
- `gate_check(src, granted=...)` — 필요한 effect가 허가돼야만 통과 (capability gate)

## 어디에 쓰나
- 사용자 제출 로직 / 설정 DSL / 규칙식(rule expression)의 **안전한 평가**
- 신뢰 경계를 넘는 입력을 "부작용 없음 + 자원 상한 + 권한 명시"로 실행해야 하는 곳
- 감사 가능(auditable)·재현 가능해야 하는 계산 레이어

## 실행
```sh
python pnix-hy/examples/01-pure-sandbox/limit_python.py
python pnix-hy/examples/01-pure-sandbox/pnix_hy_way.py
```
