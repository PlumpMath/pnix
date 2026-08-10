# 73. Wiki integrity index

## 무엇을 보여주나

generated WIKI의 capability registry, roadmap, integrity check를 data API로 확인하는 예제다.

## plain Clojure의 한계

문서 문자열이나 체크리스트를 손으로 관리하면 landed capability가 실제 report registry에 연결됐는지 자동으로 확인하지 못한다.

## pnix-clj 방식

`pnix-clj.wiki/capability-registry`, `roadmap`, `integrity`가 generated docs의 원천 data와 landed capability wiring check를 제공한다.

## 어디에 쓰나

AI/개발자가 새 기능을 만들기 전에 이미 있는 capability를 찾거나, roadmap claim이 code registry를 앞서가지 않는지 확인할 때 쓴다.


## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(ns wiki-integrity-index-limit)
(def checklist
  [{:id :feature-a :status :landed :capability :missing-report}])
(println "manual checklist:" checklist)
(println "report registry checked?:" false)
(println "generated wiki integrity?:" false)
(assert (= :landed (:status (first checklist))))
(println)
(println "결론: plain checklist는 landed capability가 실제 report registry에 연결됐는지 검증하지 않는다.")
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(ns pnix-clj-way
  (:require [pnix-clj.wiki :as wiki]))
(let [registry (wiki/capability-registry)
      roadmap (wiki/roadmap)
      integrity (wiki/integrity)]
  (println "capability count:" (count registry))
  (println "first capability:" (select-keys (first registry) [:kind :module :run]))
  (println "roadmap items:" (count (:items roadmap)))
  (println "integrity:" integrity)
  (assert (pos? (count registry)))
  (assert (some #(= "weval" (:kind %)) registry))
  (assert (pos? (count (:items roadmap))))
  (assert (= :ok (:status integrity))))
(println)
(println "결론: pnix-clj wiki data는 docs가 code registry를 앞서가지 않도록 integrity gate를 제공한다.")
(shutdown-agents)
```

비교하면, limit 파일은 plain Clojure 값/예외/수동 상태를 직접 만든다. pnix-clj 파일은 같은 문제를 pnix-clj API에 태우고 `assert`로 `:status`, hash, receipt, gate verdict 같은 증거를 확인한다. 전체 실행 코드는 같은 디렉터리의 두 `.clj` 파일을 보면 된다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 73. Wiki integrity index 문제를 어떻게 흉내 내는지 본다.
;; - 핵심은 '값은 만들 수 있지만, 그 값이 안전한지/재현 가능한지/같은 의미인지
;;   증거가 자동으로 남지 않는다'는 점이다.
;; - 실무에서는 이 부분이 버그 리포트, 감사 로그, CI verdict로 바로 이어지기 어렵다.

;; pnix_clj_way.clj
;; - source나 fixture를 pnix-clj API에 넣고, result map을 받는다.
;; - (:status result), (:reason result), (:value result), hash/receipt/witness field를 assert 한다.
;; - 이 assert는 예제에서는 교육용이지만, 실제로는 CI gate, PR comment,
;;   deployment approval, audit event row로 바꿔 붙이면 된다.
```

이 예제에서 특히 봐야 할 점은 다음이다.

- plain 쪽 한계: plain Clojure 쪽은 문자열 출력이나 수동 목록으로 충분해 보이지만, 구현이 바뀌면 문서와 운영 화면이 drift 난다.
- pnix-clj 쪽 핵심: pnix-clj 쪽은 registry/report API를 통해 현재 구현 상태를 읽고, count/hash/status를 assert 해서 문서와 코드의 거리를 줄인다.
- 판단 기준: 사람이 손으로 적은 문서/목록 대신, 코드와 resource에서 현재 capability/report/lane 상태를 뽑아 운영자가 볼 수 있게 만든다.

## 산업/실무 적용

적용 가능한 개발 도메인 예시는 다음과 같다.

- developer experience platform
- internal docs portal
- SRE runbook generation
- release dashboard
- runtime capability inventory

실무 흐름으로 바꾸면 이렇게 쓴다. release 전 docs/report job에서 registry를 다시 렌더링하고, count/hash 변화가 있으면 changelog나 review task로 연결한다.

```clojure
;; 실제 서비스 코드에서는 아래 map을 DB row, CI artifact, PR comment,
;; deployment approval payload 같은 형태로 저장하면 된다.
{:domain :devex-dashboard
 :report-kind kind
 :status (:status report)
 :count (:total report)
 :drift? (not= expected-hash (:report-hash report))}
```

업체나 팀 관점에서 보면, 이 예제는 라이브러리 기능 하나를 보여주는 것이 아니라 "자동화가 결정을 내리기 전에 어떤 증거를 요구할 것인가"를 정하는 작은 패턴이다.


## 초딩 설명

### 이 예제가 말하는 것

이전 설명: 사람이 손으로 적은 문서/목록 대신, 코드와 resource에서 현재 capability/report/lane 상태를 뽑아 운영자가 볼 수 있게 만든다.

초딩 설명: 교실 시간표를 손으로 계속 고치면 틀릴 수 있다. 대신 실제 수업 목록에서 자동으로 시간표를 만들면 덜 틀린다.

한 문장으로 줄이면, 이 예제는 `그냥 믿고 실행하기` 대신 `먼저 확인하고, 이유를 적고, 나중에 다시 볼 수 있게 남기기`를 보여준다.

### 코드 쉽게 읽기

이전 설명: `limit_clojure.clj`와 `pnix_clj_way.clj`를 비교해서 plain Clojure의 한계와 pnix-clj 방식을 본다.

초딩 설명: 두 파일은 같은 문제를 두 가지 방식으로 푼다.

```clojure
;; limit_clojure.clj
;; 그냥 해 본다. 답이 나올 수도 있지만, 위험했는지, 왜 멈췄는지,
;; 나중에 다시 확인할 영수증이 있는지는 잘 모른다.

;; pnix_clj_way.clj
;; 먼저 검사하고, 결과를 표처럼 받는다.
;; :ok    = 초록불, 해도 됨
;; :held  = 잠깐 멈춤, 이유를 봐야 함
;; :reason = 왜 멈췄는지 적힌 쪽지
;; :value  = 진짜 답
;; assert  = 예상한 답이 맞는지 확인하는 선생님
```

이 README의 `코드 비교`에서 `assert`가 보이면, 어렵게 생각하지 말고 `이 줄은 약속한 결과가 맞는지 확인한다`고 읽으면 된다. `hash`는 물건의 지문, `receipt`는 영수증, `witness`는 증인 도장, `gate`는 문지기라고 생각하면 된다.

### plain 쪽을 쉽게 말하면

이전 설명: plain Clojure로 문서를 출력할 수는 있지만, 실제 코드와 문서가 달라졌는지 자동으로 잡기 어렵다.

초딩 설명: plain Clojure는 장난감을 바로 움직여 보는 것과 같다. 빠르고 쉽지만, 장난감이 어디를 건드렸는지, 같은 놀이를 내일 다시 해도 같은 결과가 나오는지, 누가 허락했는지 적어 두지 않는다.

### pnix-clj 쪽을 쉽게 말하면

이전 설명: pnix-clj는 코드에서 현재 목록을 읽고 report로 만든다. 숫자나 hash가 바뀌면 무엇이 바뀌었는지 알 수 있다.

초딩 설명: pnix-clj는 놀이 전에 체크리스트를 읽고, 놀이가 끝나면 영수증을 붙인다. 성공하면 초록불, 위험하면 멈춤, 멈춘 이유는 쪽지로 남긴다. 그래서 사람이 다시 보거나 CI가 자동으로 판단하기 쉽다.

### 실무 응용을 쉽게 말하면

이전 설명: 개발자 포털, SRE runbook, release dashboard, capability inventory, 내부 문서 자동 생성에 쓴다.

초딩 설명: 회사에서는 사람이 모든 코드를 매번 눈으로 확인하기 어렵다. 그래서 이 예제처럼 작은 검사표를 만들어 두면, AI나 자동화가 만든 결과를 바로 믿지 않고 `통과`, `사람이 봐야 함`, `막아야 함`으로 나눌 수 있다.

```clojure
;; 73. Wiki integrity index를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:dashboard "현재 기능 목록"
 :count total
 :hash report-hash
 :drift? changed?}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
