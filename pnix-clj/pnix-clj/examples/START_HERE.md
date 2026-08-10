# START HERE

pnix-clj examples를 처음 보면 여기부터 읽으면 된다.

중간에 `meta-circular`, `lowering`, `receipt`, `witness` 같은 단어에서 막히면 [WORDS.md](WORDS.md)를 먼저 보면 된다. 단어를 전부 외울 필요는 없다.

## 한 문장

pnix-clj는 코드를 그냥 실행하는 것이 아니라, 실행 전에 검사하고 실행 뒤에 증거를 남기는 방식이다.

## 아주 쉽게

plain Clojure:

```text
일단 실행한다.
답은 나온다.
그런데 위험했는지, 왜 멈췄는지, 나중에 다시 확인할 수 있는지는 직접 챙겨야 한다.
```

pnix-clj:

```text
먼저 검사한다.
답과 이유를 표로 받는다.
나중에 다시 확인할 영수증을 남긴다.
```

## 제일 중요한 차이

```clojure
;; plain Clojure 느낌
(eval '(+ 1 2))
;; => 3

;; pnix-clj 느낌
{:status :ok
 :value 3
 :reason nil
 :receipt "어떤 길로 실행했는지 남김"}
```

값 `3`만 있으면 “답”만 있다. pnix-clj는 “답 + 왜 믿을 수 있는지”를 같이 보려는 쪽이다.

## 결과 읽는 법

```text
:ok     = 통과
:held   = 잠깐 멈춤. 사람이 이유를 봐야 함
:reason = 멈춘 이유
:value  = 나온 답
```

`held`는 “망했다”가 아니다. “그냥 넘어가면 위험하니 이유를 보고 결정하라”는 뜻이다.

## 처음 볼 예제 5개

1. [83-ai-generated-config-gate](83-ai-generated-config-gate/README.md)
   AI가 만든 설정을 바로 믿지 않고 검사하는 예제.

2. [86-service-option-contract](86-service-option-contract/README.md)
   설정표에 빠진 칸이나 틀린 칸을 잡는 예제.

3. [84-ci-receipt-matrix](84-ci-receipt-matrix/README.md)
   CI에서 값만 보지 않고 영수증까지 남기는 예제.

4. [87-plugin-capability-boundary](87-plugin-capability-boundary/README.md)
   agent/plugin이 파일을 읽으려 할 때 권한을 확인하는 예제.

5. [88-refactor-cache-stability](88-refactor-cache-stability/README.md)
   공백만 바뀐 코드를 같은 코드로 알아보는 예제.

## 파일 두 개를 어떻게 읽나

각 예제에는 보통 두 파일이 있다.

```text
limit_clojure.clj
  plain Clojure로 하면 어디가 부족한지 보여준다.

pnix_clj_way.clj
  pnix-clj로 같은 문제를 어떻게 검사하고 증거로 남기는지 보여준다.
```

먼저 `limit_clojure.clj`를 보고 “아, 그냥 하면 이런 문제가 있구나”를 이해한다. 그 다음 `pnix_clj_way.clj`를 보고 “그래서 pnix-clj는 status/reason/receipt로 남기는구나”를 보면 된다.

## 다 읽을 필요 없다

92개 예제를 처음부터 다 읽지 않아도 된다. 이 순서만 추천한다.

```text
START_HERE.md
WORDS.md
BEGINNER_PATH.md
REAL_WORLD_USE_CASES.md
WHY_AI_DEVELOPMENT.md
```

그 다음 필요한 예제만 골라 보면 된다.
