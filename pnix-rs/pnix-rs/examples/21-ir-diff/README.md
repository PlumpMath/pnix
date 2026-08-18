# 21-ir-diff — 정본 IR 레벨 의미 차이(structural, within-program)

## 쉽게 말하면 (비유)
`02-canonical-hash-and-drift`는 "두 프로그램이 통째로 같은가/다른가"를
해시로 본다. ir-diff는 그보다 더 세밀하게, "**어디서부터** 의미가
갈라지는가"를 IR 안의 위치로 짚어준다 — 텍스트 diff가 아니라 **정본 IR
구조** diff다.

## 무엇을
(1) 바인딩 순서만 바꾼 것(reorder)은 IR이 **동일**(meaning-preserving),
(2) 실제 의미가 바뀌면 IR이 다르고 **첫 차이 위치**가 나옴, (3) 동일
프로그램은 당연히 IR 동일, (4) 바인딩을 추가하는 구조 변경은 IR 다름,
(5) alpha-rename(변수 이름만 바꿈)은 ir-diff 기준으로는 **다르다고
나옴** — 알파-불변 비교는 `changed_between`(def-단위)의 역할이고, 이
게이트는 그것을 보완하는 **프로그램 내부 구조 뷰**다.

## plain Rust의 한계 (`limit_rust.rs`)
`git diff`/`diff -u`는 텍스트 라인 단위다. 포맷팅만 바뀌어도, 또는 의미가
같은 재배열이어도 "다르다"고 나온다. 반대로 정본 IR에서 의미가 갈라지는
**정확한 위치**를 구조적으로 짚어주는 표준 도구는 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs ir-diff-check` — 재배열=동일 / 의미변경=첫 차이 위치 / 구조변경
  탐지 / alpha-rename은 diff(비-불변, changed_between과 역할 분담)

## 어디에 쓰나
리뷰 도구("이 변경이 진짜 의미를 바꾸나?"), 회귀 국소화(어디서부터
갈라졌는지).

## 실행
```sh
rustc -O examples/21-ir-diff/limit_rust.rs -o /tmp/limit_21-ir-diff && /tmp/limit_21-ir-diff
bash examples/21-ir-diff/pnix_rs_way.sh
```
