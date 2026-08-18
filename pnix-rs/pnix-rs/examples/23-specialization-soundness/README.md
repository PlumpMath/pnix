# 23-specialization-soundness — 특화의 구조적 경계와 재사용 안전

## 쉽게 말하면 (비유)
부분평가가 "값"은 맞혀도 "왜 안전하게 재사용해도 되는지"는 별개 질문이다.
이 예제는 두 게이트를 묶는다: (phase) 잔여 코드가 **정확히 동적 변수만**
남기는가(정적 입력이 새고 있지 않은가), (assumption) 그 잔여 코드가
**어떤 가정 아래**에서만 유효한지, 그 가정이 깨지면 재사용이 오답을
내는지.

## 무엇을
**phase-check**: `x * (y + 3)`처럼 y가 동적이면 residual의 자유변수가
정확히 `["y"]`(+x)이어야 함 — 정적 입력(`k=3`처럼 고정된 값)은 특화
시점에 완전히 소진되어야 한다(값 자체가 아니라 **구조적 경계**를 검사,
06/12와 상호보완).

**assumption-check**: residual은 특화 당시의 정적 가정(`k=2`) 아래
유효하다. 가정이 그대로면 재사용은 정답(`x*5`, x=4→20). 가정이
바뀌면(`k=3`) 옛 residual을 그대로 재사용하는 것은 **오답**(20, stale)이고,
재특화한 새 residual(`x*6`)이 정답(24)이다 — "언제 캐시를 무효화해야
하는가"의 근거.

## plain Rust의 한계 (`limit_rust.rs`)
Rust 컴파일러 최적화는 상수를 접지만, "이 특화가 어떤 가정에 의존하는지"를
명시적으로 추적하거나, 그 가정이 깨졌을 때 재사용이 안전한지 검사할
표준 메커니즘이 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs phase-check` — residual 자유변수 == 동적 변수 집합(구조 경계)
- `pnix-rs assumption-check` — 가정 유지 시 유효 / 가정 변경 시 stale
  탐지 + 재특화로 복구

## 어디에 쓰나
캐시 무효화 정책, 증분 특화 파이프라인에서 "언제 다시 특화해야 하는가"의
정확한 경계.

## 실행
```sh
rustc -O examples/23-specialization-soundness/limit_rust.rs -o /tmp/limit_23-specialization-soundness && /tmp/limit_23-specialization-soundness
bash examples/23-specialization-soundness/pnix_rs_way.sh
```
