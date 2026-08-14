# 14 — tryEval

## 무엇을

`builtins.tryEval` 로 throw 를 `{ success, value }` 형태로 받는다.

## 실행

```bash
cd pnix-clr
./bin/pnix-clr -e 'builtins.tryEval (1 + 1)'
./bin/pnix-clr -e 'builtins.tryEval (throw "x")'
./bin/pnix-clr pnix-clr/examples/14-tryEval/sample.px
```
