# 16 — 클로저 (커링 + 캡처 재사용)

## 무엇을

람다가 반환한 함수가 바깥 바인딩을 캡처하고, 그 클로저를 **여러 번, 서로
다른 인자로** 재호출해도 캡처한 값이 그대로 유지되는지.

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
