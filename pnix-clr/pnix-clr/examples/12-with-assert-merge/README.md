# 12 — with · assert · 병합

## 무엇을

`with`, `assert`, 리스트 `++`, attrset `//`.

## 실행

```bash
cd pnix-clr
./bin/pnix-clr -e 'with { a = 1; }; a'
./bin/pnix-clr -e '[1 2] ++ [3]'
./bin/pnix-clr -e '{ a = 1; } // { b = 2; }'
./bin/pnix-clr pnix-clr/examples/12-with-assert-merge/sample.px
```
