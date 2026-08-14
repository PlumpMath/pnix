# 15 — 문자열 · 버전 helpers

## 무엇을

`substring`, `concatStringsSep`, `splitVersion`, `toString` 등.

## 실행

```bash
cd pnix-clr
./bin/pnix-clr -e 'builtins.substring 1 2 "abcd"'
./bin/pnix-clr -e 'builtins.concatStringsSep "," ["a" "b"]'
./bin/pnix-clr -e 'builtins.splitVersion "1.2.3"'
./bin/pnix-clr pnix-clr/examples/15-string-and-version/sample.px
```
