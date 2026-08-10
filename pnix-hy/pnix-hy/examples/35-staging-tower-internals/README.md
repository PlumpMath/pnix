# 35. staging tower 내부 — 평가를 데이터로 (proposal 0026 M2/M3)

## 무엇을
pnix가 평가를 **데이터로 다루는** 저수준 기계들:
- **CEK 기계** (`cek_run`/`cek_step`/`reify_cek_state`/`cek_resume`) — 계산을 (control, env, kont)
  상태로 두고 한 스텝씩; 도중에 멈춰 상태를 reify(sha256 + witness)하고 나중에 resume.
- **stage-polymorphic mini 평가기** (`stage_poly_interpret` / `stage_poly_compile`) — 같은 소스를
  값으로(interpret) 또는 잔여 코드로(compile).
- **offline BTA** (`binding_time_analysis`) — 특화 전에 각 부분을 정적(S)/동적(D)으로 분류.

## 왜
평범한 평가는 호스트 콜스택에 갇혀 "통째로" 일어난다 — 멈추고·스냅샷하고·재개할 수 없고, 같은
인터프리터가 interpret/compile 두 역할을 겸하지도, 정적/동적을 분류하지도 못한다. staging tower는
그 셋을 **데이터로** 노출한다(read→compile→run→collapse 타워의 기반).

## 예
```
paused  = cek_run("(2+3)*4", pause_at=4)     # status="paused", reified: sha256+witness
resumed = cek_resume(paused["reified"])       # value == 20 (통째 실행과 동일)
stage_poly_interpret("(input+1)*3", {"input":5})  == 18
stage_poly_compile   ("(input+1)*3", ("input",))  == "((input + 1) * 3)"
binding_time_analysis("a*x+b", ("x",))["division"] == {"x": "D"}
```

## 한 줄
> 평가를 **데이터**(CEK 상태)로 두면 — 멈추고, 해시·증거로 reify하고, 재개할 수 있다; 같은 평가기가
> interpret도 compile도 하고, 특화 전에 정적/동적을 분류한다.

## 경계
- 이 기계들이 Futamura 사다리(examples/33)와 cogen(examples/20)의 기반. 정본 평가기(`pnix_runtime`)·
  4-lane 미러 무접촉 — tower는 그 위에 얹힌 추가 레인.
