# WORDS

어려운 단어를 쉬운 말로 바꾼 문서다.

## meta-circular

어려운 말:

```text
언어가 자기 자신이나 자기와 아주 가까운 표현으로 자기 실행기를 설명하는 구조.
```

쉬운 말:

```text
레고 설명서를 레고로 다시 만드는 느낌.
코드를 실행하는 규칙도 코드처럼 다루는 것이다.
```

왜 중요하나:

```text
실행기 자체를 검사하고 비교할 수 있다.
그래서 "그냥 돌아간다"보다 "왜 같은 뜻인지"를 더 잘 따질 수 있다.
```

## evaluator

쉬운 말:

```text
코드를 읽고 값을 계산해 주는 계산기.
```

예:

```text
"1 + 2"를 넣으면 3을 만든다.
```

## lowering

어려운 말:

```text
높은 수준의 코드를 더 낮은 수준의 실행 형태로 바꾸는 일.
```

쉬운 말:

```text
사람이 읽기 좋은 레시피를 기계가 따라 하기 좋은 순서표로 바꾸는 일.
```

pnix-clj에서는 evaluator 결과와 lowering 결과가 같은 뜻인지 계속 비교한다.

## runtime

쉬운 말:

```text
코드가 실제로 움직일 때 필요한 작은 실행 도구 상자.
```

## Futamura

어려운 말:

```text
partial evaluation으로 interpreter와 프로그램을 섞어 compiler처럼 만드는 아이디어.
```

쉬운 말:

```text
매번 반복해서 생각할 부분을 미리 계산해 두는 방법.
```

처음에는 이름을 외울 필요 없다. “미리 계산해서 더 작고 빠른 실행 모양을 만든다” 정도로 보면 된다.

## gate

어려운 말:

```text
실행하거나 반영하기 전에 통과/거부를 판단하는 단계.
```

쉬운 말:

```text
문지기.
위험하면 못 지나가게 막는다.
```

코드 느낌:

```clojure
{:status :held
 :reason :capability-denied}
```

## held

어려운 말:

```text
실패가 아니라 증거 있는 보류 verdict.
```

쉬운 말:

```text
잠깐 멈춤.
사람이 이유를 보고 결정해야 한다.
```

`held`는 “에러 터짐”보다 낫다. 왜 멈췄는지 `:reason`이 있기 때문이다.

## reason

쉬운 말:

```text
멈춘 이유 쪽지.
```

예:

```clojure
:duplicate-attr
:capability-denied
:import-evaluation-not-wired
```

## receipt

어려운 말:

```text
실행 경로와 결과를 담은 증거 레코드.
```

쉬운 말:

```text
영수증.
무엇을 했는지 적힌 종이.
```

값만 있으면 “결과”다. receipt가 있으면 “결과가 어떻게 나왔는지”도 있다.

## witness

쉬운 말:

```text
증인 도장.
입력, 출력, 효과를 나중에 확인할 수 있게 찍어 둔 표시.
```

예:

```clojure
{:effect-class :file-read
 :input-hash "..."
 :output-hash "..."
 :witness-hash "..."}
```

## hash

쉬운 말:

```text
지문.
내용이 같으면 같은 지문이 나온다.
```

공백만 바뀌었는데 같은 프로그램이면 같은 hash가 나올 수 있다. 그래서 cache가 똑똑해진다.

## lane

어려운 말:

```text
같은 source를 보는 다른 실행/검증 경로.
```

쉬운 말:

```text
같은 문제를 푸는 다른 친구들.
모두 같은 답을 내면 더 믿을 수 있다.
```

예:

```text
evaluator
clj-meta
px-runtime
machine
```

## oracle

쉬운 말:

```text
정답 비교용 기준.
```

예를 들어 Nix와 같은 동작을 해야 하는 기능이면, 실제 Nix 결과를 보고 pnix-clj 결과가 맞는지 비교할 수 있다.

## N-version harness

어려운 말:

```text
같은 source를 여러 구현/실행 경로에 넣고 결과가 같은지 비교하는 검증 장치.
```

쉬운 말:

```text
같은 문제를 여러 친구에게 풀게 하고 답이 같은지 보는 시험.
```

pnix-clj에서는 evaluator, clj-meta, px runtime, mirror, machine 같은 여러 길을 비교한다.

## shrinking

쉬운 말:

```text
큰 틀린 문제를 제일 작은 틀린 문제로 줄이는 일.
```

왜 좋나:

```text
긴 source가 틀렸다고만 하면 고치기 어렵다.
작은 반례 하나로 줄이면 어디가 문제인지 빨리 볼 수 있다.
```

## frontier

쉬운 말:

```text
아직 모르는 곳.
모르면 추측하지 않고 멈춘다.
```

좋은 frontier는 이런 것이다.

```clojure
{:status :held
 :reason :machine-unsupported-op}
```

나쁜 방식은 모르는 기능을 대충 실행해서 틀린 답을 내는 것이다.

## content-address

어려운 말:

```text
source 문자열이 아니라 정본 구조의 hash로 identity를 잡는 방식.
```

쉬운 말:

```text
겉모습 말고 속뜻으로 이름표를 붙인다.
```

예:

```text
"1 + 2"
" 1   +   2 "
"(1 + 2)"
```

겉모습은 다르지만 뜻은 같다. pnix-clj cache는 이런 경우 같은 것으로 볼 수 있다.

## interop

어려운 말:

```text
서로 다른 실행 세계가 값을 주고받는 일.
```

쉬운 말:

```text
pnix-clj 코드와 Clojure/JVM 세상이 서로 말을 거는 일.
```

중요한 점:

```text
아무 값이나 마음대로 건네면 위험하다.
그래서 opaque host ref, capability, witness 같은 장치로 경계를 관리한다.
```

## machine

어려운 말:

```text
evaluator에서 파생된 abstract machine 실행 경로.
```

쉬운 말:

```text
같은 코드를 다른 엔진으로 한 번 더 돌려 보는 길.
재귀로 깊이 들어가지 않고, 할 일 목록을 들고 차근차근 실행하는 실행기.
```

왜 필요하나:

```text
같은 코드가 evaluator와 machine에서 같은 답을 내면 더 믿을 수 있다.
너무 깊은 식을 일반 evaluator가 풀면 stack이 터질 수 있다.
machine은 작은 stack에서도 버티는지 확인할 수 있다.
```

## import seam

쉬운 말:

```text
다른 파일/모듈을 가져오는 문.
문을 열 열쇠 resolver가 있어야 가져올 수 있다.
```

resolver가 없으면:

```clojure
{:status :held
 :reason :import-evaluation-not-wired}
```

## assert

쉬운 말:

```text
예상한 결과가 맞는지 확인하는 줄.
```

예:

```clojure
(assert (= :ok (:status result)))
```

뜻:

```text
result가 통과 상태여야 한다.
아니면 이 예제는 실패다.
```
