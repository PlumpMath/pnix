# 17 — pnix REPL 세션

## 쉽게 말하면 (비유)
`dotnet fsi`나 다른 언어의 대화형 셸을 터미널에서 치면 REPL이 열리듯,
`./bin/pnix-clr --repl`은 pnix 문법을 위한 같은 감각의 REPL이다 — 단,
매 줄이 독립적으로 평가되는 단순 read-eval-print 루프다(이전 줄의 `let`
바인딩이 다음 줄로 안 넘어간다).

## 무엇을

`--repl`(= `.#pnix-clr-pnix` flake app)이 여는 대화형 pnix REPL. 매 줄을
읽어 평가하고 값을 찍는다 — 상태(let 바인딩 등)는 각 줄 단위로 독립이다
(warm env 축적은 아님, 단순 read-eval-print 루프).

## plain .NET의 한계
.NET 대화형 셸(`dotnet fsi`, C# Interactive)은 `let`/`var`가 세션 전체에
누적되는 warm 환경이다. pnix REPL은 의도적으로 그렇지 않다 — 각 줄이 독립
평가이므로, 여러 줄에 걸친 상태를 쌓고 싶다면 `let ... in ...`을 한 줄
(또는 한 표현식) 안에 다 넣어야 한다.

## 실행

```bash
cd pnix-clr
echo '(1 + 2)
(let a = 10; in a * a)' | ./bin/pnix-clr --repl
# 또는 flake app으로:
#   nix run .#pnix-clr-pnix
```

출력:
```
pnix-clr — pnix REPL. :q to quit.
pnix> 3
pnix> 100
pnix>
```

## 어디에 쓰나

빠른 pnix 식 확인, 다른 REPL(`repl.it`류)이 없는 CLR 쪽에서 대화형 탐색.
