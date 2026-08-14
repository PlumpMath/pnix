# 13 — 패턴 람다

## 무엇을

attrset formal: 필수 키, 기본값 `?`.

## 실행

```bash
cd pnix-clr
./bin/pnix-clr -e '({ a }: a) { a = 7; }'
./bin/pnix-clr -e '({ a ? 2 }: a) {}'
./bin/pnix-clr pnix-clr/examples/13-pattern-lambda/sample.px
```
