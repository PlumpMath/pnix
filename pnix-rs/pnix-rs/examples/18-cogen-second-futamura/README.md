# 18-cogen-second-futamura — 컴파일러 생성기(2차 Futamura 사영)

## 쉽게 말하면 (비유)
1차 Futamura 사영(`06-specialization-futamura`)은 "인터프리터 + 프로그램"을
접어서 "컴파일된 그 프로그램"을 얻는다. 2차 사영은 한 단계 더 올라가서,
"인터프리터를 접는 특화기 자신"을 접어 **컴파일러**를 만든다 — Leuschel이
말한 "self-applicable specializer 없이 얻는 self-application의 이득".

## 무엇을
`cogen`은 프로그램을 받아 그 프로그램 전용 컴파일된 residual을 직접
생성한다(인터프리터를 매번 다시 접지 않음). 게이트: (1) 생성된 residual이
입력 배터리에 걸쳐 해석 결과와 일치, (2) residual이 인터프리터-free(디스패치
흔적 없음 — 생성 시점에 이미 소비됨), (3) 프로그램이 다르면 컴파일 결과도
다름(cogen이 프로그램을 추적).

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 "인터프리터를 한 번 접어 컴파일러 자체를 만든다"는 2차 Futamura
사영에 해당하는 표준 도구가 없다. `rustc`는 Rust 프로그램의 컴파일러이지,
스스로를 "접어서" 다른 언어용 컴파일러를 생성하지 않는다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs cogen-check` — 프로그램별 컴파일된 residual 생성 + 정확성 +
  인터프리터-free + 프로그램 추적성

## 어디에 쓰나
"특화가 된다"(06)에서 "컴파일러를 만들 수 있다"(18)로: DSL/px 프로그램을
매번 해석하지 않고 전용 코드로 직접 생성하는 파이프라인의 근거.

## 실행
```sh
rustc -O examples/18-cogen-second-futamura/limit_rust.rs -o /tmp/limit_18-cogen-second-futamura && /tmp/limit_18-cogen-second-futamura
bash examples/18-cogen-second-futamura/pnix_rs_way.sh
```
