# 17 — pnix REPL 세션

## 무엇을

`--repl`(= `.#pnix-cljs-pnix` flake app)이 여는 대화형 pnix REPL. 매 줄을
읽어 평가하고 값을 찍는다 — 상태(let 바인딩 등)는 각 줄 단위로 독립이다
(warm env 축적은 아님, 단순 read-eval-print 루프).

## 실행

```bash
cd pnix-cljs
echo '1 + 2
let a = 10; in a * a' | ./bin/pnix-cljs --repl
# 또는 flake app으로:
#   nix run .#pnix-cljs-pnix
```

출력:
```
pnix-cljs — pnix REPL. :q to quit.
pnix> 3
pnix> 100
pnix>
```

## 어디에 쓰나

빠른 pnix 식 확인, Node 쪽에서 대화형 탐색.
