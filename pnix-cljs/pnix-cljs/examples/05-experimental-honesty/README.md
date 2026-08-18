# 05 — experimental 정직성

## 쉽게 말하면 (비유)
소프트웨어 README는 종종 "무엇이 되는가"만 적고 "무엇이 아직 안 되는가"는
빼먹는다. 이 예제는 그 반대를 한다 — pnix-cljs가 **주장하지 않는 것**을
코드처럼 명시적으로 고정해서, 카탈로그 자체가 과대 주장을 못 하게 막는다.

## 무엇을
`honesty.md`가 "주장 / 비주장" 표를 코드가 아니라 **정책 문서**로 고정한다.
다른 예제(00~04, 07~17)가 실제로 하는 일을 넘어서는 claim이 이 카탈로그
어디에도 안 생기게 하는 앵커 역할.

## 비주장
- 다섯 호스트 동일 의미 패리티
- clj machine / oracle 전 레인
- crates.io / npm / nuget 공개 배포 제품성
- Stage15/N 자기호스팅 완료

## 주장 (좁게)
- Node에서 admitted seed eval
- 로컬 라이브러리 export + require
- Done/Failed 관측 결과

## 왜 필요한가
pnix-cljs는 다른 4개 host보다 늦게 시작된 experimental seed다(모노레포
`examples/EXAMPLES_BALANCE.md` 규칙 4). "성숙도가 다르다"는 말은 추상적이라
지키기 어렵다 — 이 문서는 그걸 **구체적인 표**로 바꿔서, 새 예제를 추가할
때마다 "이게 비주장 목록을 깨는가?"를 기계적으로 확인할 수 있게 한다.

## 어디에 쓰나
새 예제를 쓰기 전에 먼저 읽는 문서 — clj/hy의 깊은 연구 슬라이스(machine,
oracle, cegis 등)를 cljs에 그대로 복제하고 싶어질 때, 이 표가 "아직 안 된다"
는 걸 상기시킨다.

## 실행
읽기 전용 문서다 — 코드 실행은 없다. `honesty.md` 참고.
