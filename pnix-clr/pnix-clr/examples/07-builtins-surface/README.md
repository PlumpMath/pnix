# 07 — builtins / lib 표면

## 무엇을

clr seed 가 인정하는 builtins·`lib` 별칭을 CLI 로 확인한다.
README 제품 코퍼스와 같은 방향 (typeOf, getAttrFromPath, sum 등).

## 실행

```bash
cd pnix-clr
./bin/pnix-clr -e 'builtins.typeOf 1'
./bin/pnix-clr -e 'builtins.getAttrFromPath [ "foo" "bar" ] { foo.bar = 42; }'
./bin/pnix-clr -e 'lib.sum [1 2 3 4]'
./bin/pnix-clr pnix-clr/examples/07-builtins-surface/sample.px
```
