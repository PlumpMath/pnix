# 94. Mirror-pair 코퍼스 집계 report

## 무엇을 보여주나

`72-mirror-facet-rows`가 **한 소스 하나**의 mirror facet(clojure-mirror/
px-runtime/pnix-mirror/cross-mirror-verdict)을 디버그용으로 들여다본다면,
이건 그 4-레인 수렴을 **committed 코퍼스 204개 전체**에서 집계한
report다 — direct evaluator / clj-meta bytecode / .px runtime / pnix
mirror가 코퍼스 규모로 하나로 수렴하는지 보는 핵심 회귀 게이트.

## plain Clojure의 한계

plain Clojure로 이걸 하려면 204개 소스를 손으로 돌리고, 매번 4개 레인의
값을 눈으로 비교하고, 몇 개가 `ready?`인지 직접 세야 한다 — 코퍼스가
자라도 자동으로 안 늘어나고, 회귀가 어디서 났는지 행 단위로 안 남는다.

## pnix-clj 방식

`mp/report`(`pnix-clj.mirror-pair`)는 committed fixture 코퍼스
(`cases.edn`)를 읽어 각 소스를 `pnix/report`에 통과시키고,
`mirror-pair-ready-count`/`mirror-pair-not-ready-count`와 소스별
row(`mirror-pair-rows`)를 반환한다.

## 어디에 쓰나

- evaluator/clj-meta/px-runtime/pnix-mirror 중 하나를 고치는 PR의 핵심
  회귀 게이트
- `bootstrap_test.clj`의 `mirror-pair-report-tracks-basic-runtime-fixtures`
  가 바로 이 report를 검증한다(이 세션에서 `-M:test` 51개 실패를 고칠 때
  실측한 대상)

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(defn pretend-four-lanes-agree? [source]
  (boolean (seq source)))
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(let [report (mp/report)]
  (assert (= 204 (:fixture-count report)))
  (assert (= (:fixture-count report) (:mirror-pair-ready-count report)))
  (assert (zero? (:mirror-pair-not-ready-count report))))
```

비교하면, limit 파일은 소스 하나가 비어있지 않으면 그냥 "동의"한다고 침한다.
pnix-clj 파일은 실제 204개 코퍼스를 4-레인에 통과시키고 `ready?` 개수를
집계한다.

## 코드 해설

```clojure
;; fixture-set: 코퍼스 lineage/schema-version 메타
(mp/fixture-set)

;; cases: 204개 소스 + expected import-module 목록
(mp/cases)

;; report: 각 case를 pnix/report에 태워 4-레인 값을 비교하고 집계
(mp/report)
```

`ready?`는 그 소스에서 4-레인(evaluator/clj-meta/px-runtime/pnix-mirror)이
전부 같은 값으로 수렴했다는 뜻이다.

## 산업/실무 적용

CI 게이트로 붙이면:

```clojure
{:job :mirror-pair-corpus-gate
 :fixture-count (:fixture-count report)
 :not-ready-count (:mirror-pair-not-ready-count report)
 :decision (if (zero? (:mirror-pair-not-ready-count report))
             :allow-merge
             :block-merge)}
```

레인 하나(예: clj-meta 컴파일러)를 고쳤을 때, 그 변경이 나머지 3개 레인과
계속 같은 값을 내는지 204개 규모로 즉시 확인한다.

## 초딩 설명

이전 설명: mirror-pair는 4명의 선생님(evaluator/clj-meta/px-runtime/
pnix-mirror)이 204개 문제 전부에서 같은 답을 내는지 확인한다.

초딩 설명: 네 선생님한테 같은 문제집 204문제를 다 풀게 하고, 넷의 답이
전부 똑같은 문제가 몇 개인지 센다. 한 선생님만 다른 답을 내면 그 문제
번호가 바로 보인다.

기억할 것: `72`번은 문제 하나를 자세히 들여다보는 예제고, `94`번은 문제집
204개 전체의 점수표를 보는 예제다.
