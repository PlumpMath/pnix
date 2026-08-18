# 16 — 클로저 (커링 + 캡처 재사용)

## 쉽게 말하면 (비유)
`make_adder 5`가 돌려주는 함수는 `n = 5`를 "기억한 채" 이후 몇 번을 다른
인자로 불러도 그 5를 계속 쓴다 — C# 람다 클로저와 같은 감각이지만, 여기서는
pnix 언어의 람다가 그 캡처를 갖는다는 걸 확인한다.

## 무엇을
람다가 반환한 함수가 바깥 바인딩을 캡처하고, 그 클로저를 **여러 번, 서로
다른 인자로** 재호출해도 캡처한 값이 그대로 유지되는지.

## plain .NET의 한계
C# 람다식도 당연히 클로저를 갖는다 — 이 예제가 보여주는 한계는 C# 자체의
한계가 아니라, **pnix 언어의 람다가 별도 구현 없이 호스트(ClojureCLR/.NET)의
클로저 메커니즘 위에 자연스럽게 얹힌다**는 사실 쪽이다. pnix-clr는 클로저를
흉내내기 위한 자체 캡처-환경 구조를 새로 만들 필요가 없었다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e 'let make_adder = n: (x: x + n); add5 = make_adder 5; in add5 10'
{"host":"pnix-clr","outcome_kind":"done",...,"value":15}

$ ./bin/pnix-clr -e 'let counter_maker = start: (step: start + step); c = counter_maker 100; in [(c 1) (c 2) (c 3)]'
{"host":"pnix-clr","outcome_kind":"done",...,"value":[101,102,103]}

$ ./bin/pnix-clr pnix-clr/examples/16-closures/sample.px
{"host":"pnix-clr","outcome_kind":"done",...,"value":{"added":15,"counted":[101,102,103]}}
```
`c`를 세 번 다른 인자로 불러도 `start = 100` 캡처는 매 호출 그대로다.

## 어디에 쓰나
부분 적용된 설정 함수(`make_adder 5`처럼 고정 파라미터를 가진 헬퍼)를
만들어 여러 값에 재사용할 때.

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e 'let make_adder = n: (x: x + n); add5 = make_adder 5; in add5 10'
./bin/pnix-clr -e 'let counter_maker = start: (step: start + step); c = counter_maker 100; in [(c 1) (c 2) (c 3)]'
./bin/pnix-clr pnix-clr/examples/16-closures/sample.px
```

## 경계

- 이건 **pnix-clr 제품 런타임**(ClojureCLR 호스트 위 pnix 평가)의 클로저다.
  `clr-meta`(자기호스팅 증명 레인) 자신의 클로저 구현은 별개이며 — 이 카탈로그가
  Stage15/N을 승격하지 않듯, 그쪽 세부사항도 여기서 다루지 않는다.
- `06-meta-pair-boundary` 참고: 제품/meta 두 절반의 경계.
