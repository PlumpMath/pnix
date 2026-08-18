# 11 — 리스트 고차 builtins

## 쉽게 말하면 (비유)
`map`/`filter`는 JS 배열의 `.map`/`.filter`와 감각적으로 똑같다. 다른 점은
pnix의 리스트 원소가 정수일 때 **BigInt**로 나온다는 것 — JS 배열 메서드
자체는 익숙하지만, 값을 다시 JS에서 다룰 때 그 BigInt 경계를 의식해야 한다.

## 무엇을
`map`, `filter`, `genList`(인덱스로 리스트 생성), `concatLists`(리스트의
리스트를 평탄화), `head`, `elem`(멤버십 검사) 6가지 리스트 seed builtins.

## plain Node의 한계
JS `Array.prototype.map`/`filter`/`flat`은 있지만, pnix 값(정수→BigInt)과
JS 배열 사이를 오갈 때 타입 변환을 직접 챙겨야 한다. `genList`처럼 "길이 n,
인덱스 함수로 채우기"에 대응하는 것도 `Array.from({length:n}, (_,i)=>...)`로
매번 다시 쓰는 것과 pnix 쪽에 미리 있는 것의 차이다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`, 실행 결과)
```
builtins.map (x: x + 1) [1 2 3]        => done [ '2', '3', '4' ]
builtins.filter (x: x > 1) [1 2 3]     => done [ '2', '3' ]
builtins.genList (i: i * 2) 3          => done [ '0', '2', '4' ]
builtins.concatLists [[1] [2 3]]       => done [ '1', '2', '3' ]
builtins.head [9 8]                    => done 9
builtins.elem 2 [1 2 3]                => done true
```
(문자열로 찍힌 값들은 헬퍼가 BigInt를 `.toString()`한 결과 — 실제 값은
BigInt다.)

## 어디에 쓰나
평가 결과 리스트를 변환/필터링해서 다음 단계 파이프라인에 넘길 때, JS 쪽
`.map`/`.filter` 대신 pnix 소스 안에서 직접 변환을 표현하고 싶을 때.

## 실행
```bash
node pnix-cljs/examples/11-list-higher-order/pnix_cljs_way.js
```
