# 90. Machine report/fuel witness

## 무엇을 보여주나

M7g의 `machine/report`는 machine과 evaluator의 differential corpus를 한 번에 돌리고, divergence가 없는지와 constant-stack witness가 살아 있는지 보여준다. machine loop도 evaluator와 같은 fuel volatile을 써서 예산 초과를 같은 tagged throw로 멈춘다.

## plain Clojure의 한계

report map은 손으로 만들 수 있지만, 그 report가 실제 shared corpus를 돌렸는지, 작은 stack에서도 machine이 끝났는지, evaluator와 fuel budget을 공유하는지는 보장하지 않는다.

## pnix-clj 방식

`machine/report`를 호출해 `:row-count`, `:divergent`, `:constant-stack-witness`를 확인한다. 별도로 아주 작은 fuel budget을 걸어 `machine/run-whnf`가 `fuel exhausted`로 멈추는지도 확인한다.

## 어디에 쓰나

machine lane을 CI capability로 승격하거나, interpreter recursion을 machine control로 바꾼 뒤 regression report를 남길 때 쓴다.

파일 artifact와 gate alias까지 보고 싶으면 다음 예제인 `91-machine-report-artifact-gate`를 보면 된다.

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(defn hand-report [rows]
  {:status :ok
   :row-count (count rows)
   :divergent []
   :constant-stack-witness nil
   :fuel-budget-shared? false})
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(let [r (machine/report)]
  (assert (= :ok (:status r)))
  (assert (empty? (:divergent r)))
  (assert (true? (get-in r [:constant-stack-witness :ok?]))))

(binding [evaluator/*fuel* (volatile! 3)]
  (machine/run-whnf ast {}))
```

비교하면, limit 파일은 report 모양만 만든다. pnix-clj 파일은 실제 machine report와 fuel bound를 실행해서 확인한다.

## 코드 해설

이전 설명: machine report는 shared differential corpus, constant-stack witness, fuel budget bound를 regression artifact로 만든다.

초딩 설명: 선생님이 “이 반 전체가 시험을 봤고, 틀린 사람은 0명이고, 좁은 책상에서도 문제를 풀 수 있었고, 시간이 다 되면 멈춘다”는 성적표를 만든다.

```clojure
;; row-count는 시험 문제 수다.
;; divergent가 비어 있으면 machine과 evaluator가 다르게 푼 문제가 없다는 뜻이다.
;; constant-stack-witness는 좁은 stack에서도 machine이 버텼다는 증거다.
(assert (empty? (:divergent r)))
```

## 산업/실무 적용

- compiler/DSL CI
- runtime rewrite regression gate
- build optimizer safety check
- release dashboard
- AI-generated evaluator change review

```clojure
{:domain :machine-ci
 :row-count (:row-count report)
 :divergent (count (:divergent report))
 :constant-stack? (get-in report [:constant-stack-witness :ok?])
 :ship? (= :ok (:status report))}
```

## 초딩 설명

이전 설명: `machine/report`는 machine이 evaluator와 같은지, 작은 stack에서도 되는지, fuel로 멈추는지 확인한다.

초딩 설명: 게임 캐릭터가 여러 장애물을 전부 통과하는지 보는 테스트다. 틀린 장애물이 0개면 통과다. 시간이 다 되면 계속 달리지 않고 멈춰야 한다.
