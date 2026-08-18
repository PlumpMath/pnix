# 16-jones-optimality — Jones-최적성: 인터프리터 계층이 정말 사라졌는가

## 쉽게 말하면 (비유)
부분평가기가 "정말로" 인터프리터를 지웠다면, 인터프리터를 두 배로 부풀려도
(불필요한 분기를 잔뜩 끼워 넣어도) 나오는 잔여 코드(residual)는 **한 글자도**
달라지지 않아야 한다. Jones-최적성은 이 "인터프리터 비만도 무시" 성질을
측정 가능한 형태로 검사한다.

## 무엇을
`06-specialization-futamura`가 "특화가 된다"를 보인다면, 이 게이트는
"특화가 **최적으로** 된다"를 본다: (1) 인터프리터를 부풀려도 residual이
AST-동일, (2) residual은 인터프리터 디스패치 흔적(`.tag`/`prog`/`int` 매칭)이
전혀 없음, (3) 프로그램이 다르면 residual도 다름(잔여 코드가 **프로그램을
추적**함을 확인 — 상수로 뭉개진 게 아님), (4) residual == 실제 인터프리터
결과. 소스-레벨 BTA(예제 12)는 이 상한을 원리적으로 넘을 수 없음을 명시한다.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 "이 부분평가가 인터프리터 계층을 완전히 제거했는가"를 측정하는
표준 개념이 없다. 컴파일러 최적화가 인라이닝을 하더라도, 그것이
Jones-최적(인터프리터 비만에 불변)인지 증명하거나 검사할 방법이 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs jones-check` — 인터프리터 비만-불변, 인터프리터-free 구조,
  프로그램 추적성, 정확성을 함께 게이트

## 어디에 쓰나
부분평가 구현이 "그냥 상수접기"가 아니라 진짜 컴파일러 생성(Futamura 2사영)에
필요한 품질 기준을 갖췄는지 검증. `18-cogen-second-futamura`와 짝을 이룬다.

## 실행
```sh
rustc -O examples/16-jones-optimality/limit_rust.rs -o /tmp/limit_16-jones-optimality && /tmp/limit_16-jones-optimality
bash examples/16-jones-optimality/pnix_rs_way.sh
```
