# 04 · host interop — 손실·부작용·권한을 명시하는 경계

## 쉽게 말하면 (비유)
국경의 **세관/환전소**. 언어를 넘을 때 환율 손실·반입 금지·허가 도장을 **영수증에 적어** 준다.
plain은 말없이 통과시켜 나중에 사고가 난다.
```py
ph.from_host((1, 2, 3))[1].loss_status        # "lossy" (tuple -> list)
ph.from_host(len)[1].capability_required       # "host-call" (호출하려면 권한 필요)
```
직관: 경계 넘김의 **손실·부작용·권한이 값에 기록**되어 조용한 위험이 사라진다.

## 무엇을
Hy/Python ↔ pnix 값 변환을 **loss(손실)·effect(부작용)·capability(권한)** 기록과 함께 수행한다.

## plain의 한계 (`limit_python.py`)
Python에서 값 변환(tuple→list, set→?, bytes→?)은 **조용히 손실**되고, 콜러블을 경계 밖으로
넘겨도 "이건 부작용이 있으니 권한이 필요하다"는 표시가 값에 없다 → 경계 넘김이 조용히 위험하다.

## pnix-hy의 방식 (`pnix_hy_way.py`)
- `from_host(v)` / `to_host(v)` → 값 + `InteropRecord`(loss_status / effect_class /
  capability_required / witness). tuple→list는 `lossy`로 표시.
- `roundtrip_host_value(v)` — 왕복 fidelity를 한 곳에서 보고.
- host 콜러블은 pnix 항에 직접 들어가지 않고 **opaque ref**가 되며 `host-call` 권한을 요구.
- `try_call_host(fn, args, granted=...)` — 권한 게이트를 통과해야 호출(`{success, value|error}`).

## 어디에 쓰나
- 언어/시스템 경계를 넘는 데이터 파이프라인에서 **손실을 은폐하지 않고 추적**
- 신뢰 경계에서 host 기능 호출을 **capability로 명시 허가/거부**
- 변환·호출마다 witness를 남겨 **감사 가능한 interop**

## 실행
```sh
python pnix-hy/examples/04-host-interop-loss-effect/limit_python.py
python pnix-hy/examples/04-host-interop-loss-effect/pnix_hy_way.py
```
