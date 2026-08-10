# 92. Machine property fuzzer lane

## 무엇을 보여주나

M7h 이후 `property-fuzzer`는 다섯 번째 속성으로 `machine-property`를 가진다. 랜덤으로 만든 typed pnix source에 대해 machine과 evaluator가 값뿐 아니라 `:ok`/`:held` reason까지 정확히 같은지 본다.

## plain Clojure의 한계

plain Clojure로 sample 몇 개를 직접 돌리면 “내가 고른 예제는 통과했다” 정도만 알 수 있다. 랜덤 source 생성, 여러 seed, 실패 시 smallest counterexample shrink, machine/evaluator exact agreement는 자동으로 생기지 않는다.

## pnix-clj 방식

`pf/machine-agrees?`는 한 source에서 machine과 evaluator의 comparable result를 비교한다. `pf/report`는 cross-lane, specializer, cache, proven arithmetic, machine property를 한 번에 돌리고 `:machine-pass?`를 report에 남긴다.

## 어디에 쓰나

- AI가 evaluator/machine 코드를 수정한 PR 검토
- DSL compiler regression
- runtime rewrite safety gate
- release 전 random semantic sweep
- fixed corpus가 놓친 frame bug 찾기

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(defn pretend-same?
  [source]
  ;; 실제로 machine/evaluator 두 레인을 돌리지 않는다.
  (boolean (seq source)))

{:machine-pass? true
 :shrinks-on-failure? false
 :generated-random-sources? false}
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(doseq [source fixed-sources]
  (assert (= true (pf/machine-agrees? source))))

(let [report (pf/report {:num-tests 5 :seed 20260708})]
  (assert (= :pnix-clj.property-fuzzer-report.v4 (:schema report)))
  (assert (= true (:machine-pass? report)))
  (assert (= :ok (:status report))))
```

비교하면, limit 파일은 “통과했다고 적은 표”에 가깝다. pnix-clj 파일은 실제 generator와 property report를 돌려 `:machine-pass?`를 확인한다.

## 코드 해설

이전 설명: M7h는 machine을 N-version harness의 fifth property로 편입했다.

초딩 설명: 예전에는 정해진 문제집 155문제를 풀었다. 이제는 문제 출제기가 새 문제도 계속 내고, machine 친구와 evaluator 친구가 같은 답을 쓰는지 본다.

```clojure
;; 한 문제를 직접 비교한다.
(pf/machine-agrees? "1 + 2 * 3")

;; 랜덤 문제 묶음을 돌리고 report를 받는다.
(pf/report {:num-tests 5 :seed 20260708})

;; machine 문제가 전부 통과했는지 본다.
(assert (= true (:machine-pass? report)))
```

`seed`는 문제 출제기의 시작 번호다. 같은 seed를 쓰면 같은 랜덤 흐름을 다시 만들 수 있어서, CI에서 재현하기 좋다.

## 산업/실무 적용

실무 CI에서는 이렇게 붙일 수 있다.

```clojure
{:job :machine-property-fuzzer
 :seed seed
 :num-tests num-tests
 :machine-pass? (:machine-pass? report)
 :smallest-failing-source (:smallest-failing-source report)
 :decision (if (:machine-pass? report)
             :allow-merge
             :block-merge)}
```

AI agent가 evaluator control, machine frame, builtin boundary를 고쳤다면 fixed corpus만으로는 부족할 수 있다. 이 예제처럼 generated source sweep을 추가하면 “내가 생각한 예제” 밖의 작은 반례를 찾을 수 있다.

## 초딩 설명

이전 설명: property fuzzer는 machine과 evaluator가 랜덤 source에서도 정확히 같은지 검사하고, 실패하면 가장 작은 반례를 남긴다.

초딩 설명: 선생님이 문제를 계속 새로 만들어 낸다. 두 학생이 답을 비교해서 다르면, 제일 작은 틀린 문제만 남겨서 “여기서 틀렸어”라고 알려준다.

```text
machine-pass? = machine 친구가 evaluator 친구와 계속 같은 답을 냈나?
seed          = 같은 문제를 다시 만들기 위한 번호
shrinking     = 큰 틀린 문제를 작은 틀린 문제로 줄이는 것
```

기억할 것: 91번은 machine report를 파일 artifact로 남기는 예제이고, 92번은 machine을 랜덤 생성 검증 함대에 넣는 예제다.
