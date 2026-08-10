# 91. Machine report artifact gate

## 무엇을 보여주나

M7g 이후 machine은 단순 내부 함수가 아니라 `report-artifact` registry에 등록된 `:machine` capability다. 그래서 `clojure -M:report-machine`이나 gate script가 같은 report를 만들 수 있다.

## plain Clojure의 한계

plain Clojure도 `{:kind :machine-report :status :ok}` 같은 파일은 쓸 수 있다. 하지만 그 파일이 실제 155개 differential corpus를 돌렸는지, constant-stack witness를 확인했는지, gate alias와 연결됐는지는 보장하지 않는다.

## pnix-clj 방식

`pnix-clj.report-artifact/write-report!`에 `:machine`을 넘기면 실제 machine report를 EDN artifact로 materialize한다. 이 artifact에는 version, kind, hash, bytes, row-count, divergence, witness, honest labels가 같이 남는다.

## 어디에 쓰나

- CI gate artifact
- release dashboard
- compiler/runtime rewrite review
- AI가 수정한 evaluator/machine 변경 검토
- 운영자가 다운로드해 보는 evidence file

랜덤 생성 source까지 돌려 machine과 evaluator가 계속 같은지 보고 싶으면 다음 예제인 `92-machine-property-fuzzer-lane`을 보면 된다.

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(def hand-report
  {:kind :machine-report
   :status :ok
   :row-count 155
   :divergent []
   :constant-stack-witness {:ok? true}})

(spit file (pr-str hand-report))
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(assert (some #{:machine} artifact/supported-kinds))

(let [{:keys [kind path hash bytes report]}
      (artifact/write-report! :machine dir)]
  (assert (= :machine kind))
  (assert (= :machine-report (:kind report)))
  (assert (= :ok (:status report)))
  (assert (>= (:row-count report) 155))
  (assert (empty? (:divergent report)))
  (assert (true? (get-in report [:constant-stack-witness :ok?]))))
```

비교하면, limit 파일은 report처럼 생긴 종이를 만든다. pnix-clj 파일은 실제 registry를 통해 report를 만들고, gate에서 볼 핵심 증거를 확인한다.

## 코드 해설

이전 설명: `:machine` report artifact는 shared differential corpus와 constant-stack witness를 registry/gate 경로에서 materialize한다.

초딩 설명: 그냥 “우리 반 시험 다 맞았어요”라고 종이에 쓰는 것과, 실제 시험지를 채점하고 성적표에 도장을 찍는 것은 다르다. 91번은 실제 성적표를 만드는 쪽이다.

```clojure
;; :machine이 report registry에 등록되어 있는지 본다.
(assert (some #{:machine} artifact/supported-kinds))

;; 실제 machine report 파일을 만든다.
(artifact/write-report! :machine dir)

;; 틀린 줄이 0개인지 본다.
(assert (empty? (:divergent report)))
```

`honest labels`는 “이 report가 무엇을 보장하고, 무엇은 보장하지 않는지” 적은 주의사항이다. 예를 들어 `:differential-not-proof`는 “155개가 맞았다고 수학적 완전 증명은 아니다”라는 정직한 표시다.

## 산업/실무 적용

실무에서는 이 예제를 CI job 하나로 붙이면 된다.

```clojure
{:job :machine-report
 :command "clojure -M:report-machine"
 :artifact path
 :hash hash
 :rows (:row-count report)
 :divergent (count (:divergent report))
 :constant-stack? (get-in report [:constant-stack-witness :ok?])
 :decision (if (= :ok (:status report))
             :allow-merge
             :block-merge)}
```

AI agent가 machine/evaluator 쪽 코드를 수정한 PR을 만들면, 이 artifact를 PR comment나 release dashboard에 붙인다. 사람이 코드를 전부 읽기 전에 “machine과 evaluator가 아직 같은 뜻인가?”, “작은 stack witness가 살아 있나?”, “틀린 row가 생겼나?”를 먼저 볼 수 있다.

## 초딩 설명

이전 설명: machine report artifact는 gate가 실행할 수 있는 versioned/hash EDN 증거 파일이다.

초딩 설명: 게임 대회에서 “이겼다”라고 말만 하면 믿기 어렵다. 점수표, 심판 도장, 경기 번호가 있으면 나중에 다시 확인할 수 있다. `:machine` artifact는 그런 점수표다.

```text
row-count  = 문제 수
divergent  = machine과 evaluator가 다르게 푼 문제
witness    = 작은 stack에서도 버텼다는 증거
hash       = 파일 지문
gate alias = CI에서 누르는 실행 버튼
```

기억할 것: 90번은 machine report를 직접 보는 예제이고, 91번은 그 report를 CI/gate에서 쓰는 파일로 남기는 예제다.
