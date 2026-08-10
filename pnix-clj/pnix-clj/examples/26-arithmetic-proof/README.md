# 26-arithmetic-proof - arithmetic proof

plain Clojure에서 몇 개 숫자를 넣어보는 것은 산술식 동치의 증명이 아니다.

pnix-clj의 `arith-proof`는 `+`, `-`, `*`, int, var fragment를 canonical polynomial로 바꿔 모든 변수값에 대한 동치를 증명한다. fragment 밖은 `:unprovable`로 둔다.

실행:

```sh
cd pnix-clj
clojure -M examples/26-arithmetic-proof/limit_clojure.clj
clojure -M examples/26-arithmetic-proof/pnix_clj_way.clj
```

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(ns arithmetic-proof-limit)
(defn a [x] (+ x 2))
(defn b [x] (+ 2 x))
(def samples [-1 0 1 10])
(println "sample values:" (mapv (fn [x] [(a x) (b x)]) samples))
(println "sample equal?:" (every? true? (map #(= (a %) (b %)) samples)))
(println "missing: canonical polynomial proof over all x")
(assert (every? true? (map #(= (a %) (b %)) samples)))
(println)
(println "결론: finite samples는 모든 integer assignment에 대한 산술 동치 증명이 아니다.")
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(ns pnix-clj-way
  (:require [pnix-clj.arith-proof :as arith]))
(let [commute? (arith/equivalent? "x + 2" "2 + x")
      folded (arith/prove-specialize-meaning "x + y" {"y" 7})
      non-fragment (arith/prove-specialize-meaning "if x then 1 else 2" {})]
  (println "x + 2 == 2 + x proven?:" commute?)
  (println "specialize proof:" folded)
  (println "non fragment:" non-fragment)
  (assert (= true commute?))
  (assert (= :proven (:status folded)))
  (assert (= true (arith/equivalent? (:residual-source folded) "x + 7")))
  (assert (= :unprovable (:status non-fragment))))
(println)
(println "결론: pnix-clj arith-proof는 증명 가능한 산술 fragment와 unprovable frontier를 명확히 나눈다.")
```

비교하면, limit 파일은 plain Clojure 값/예외/수동 상태를 직접 만든다. pnix-clj 파일은 같은 문제를 pnix-clj API에 태우고 `assert`로 `:status`, hash, receipt, gate verdict 같은 증거를 확인한다. 전체 실행 코드는 같은 디렉터리의 두 `.clj` 파일을 보면 된다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 26-arithmetic-proof - arithmetic proof 문제를 어떻게 흉내 내는지 본다.
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

- plain 쪽 한계: plain Clojure 쪽은 샘플을 손으로 돌리거나 try/catch로 실패를 세지만, shrink/counterexample/proof boundary가 약하다.
- pnix-clj 쪽 핵심: pnix-clj 쪽은 generated corpus, oracle row, proof result, coverage summary, strict held reason을 구조화해서 regression을 잡는다.
- 판단 기준: 몇 개의 happy path sample이 아니라, oracle·fuzzer·proof·coverage로 반례와 frontier를 찾는다.

## 산업/실무 적용

적용 가능한 개발 도메인 예시는 다음과 같다.

- compiler QA
- financial rule validation
- insurance rating rules
- safety-critical config checks
- education/proof tooling

실무 흐름으로 바꾸면 이렇게 쓴다. 새 문법이나 evaluator 변경이 들어오면 corpus를 늘리고, rejected row는 버그인지 deliberate frontier인지 reason으로 분류한다.

```clojure
;; 실제 서비스 코드에서는 아래 map을 DB row, CI artifact, PR comment,
;; deployment approval payload 같은 형태로 저장하면 된다.
{:domain :semantic-regression
 :generated-case source
 :expected oracle-row
 :actual verdict
 :counterexample? (= :rejected (:status row))}
```

업체나 팀 관점에서 보면, 이 예제는 라이브러리 기능 하나를 보여주는 것이 아니라 "자동화가 결정을 내리기 전에 어떤 증거를 요구할 것인가"를 정하는 작은 패턴이다.


## 초딩 설명

### 이 예제가 말하는 것

이전 설명: 몇 개의 happy path sample이 아니라, oracle·fuzzer·proof·coverage로 반례와 frontier를 찾는다.

초딩 설명: 시험 문제 두세 개만 맞았다고 전부 아는 게 아니다. 여러 문제를 자동으로 많이 내 보고, 틀린 문제를 찾고, 왜 틀렸는지 표시한다.

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

이전 설명: plain Clojure는 몇 개 예제를 손으로 돌리기 쉽지만, 이상한 반례를 자동으로 찾거나 증명 결과를 남기기는 어렵다.

초딩 설명: plain Clojure는 장난감을 바로 움직여 보는 것과 같다. 빠르고 쉽지만, 장난감이 어디를 건드렸는지, 같은 놀이를 내일 다시 해도 같은 결과가 나오는지, 누가 허락했는지 적어 두지 않는다.

### pnix-clj 쪽을 쉽게 말하면

이전 설명: pnix-clj는 문제 출제기, 정답지, 채점표를 같이 둔다. 틀리면 `:reason`으로 어디서 막혔는지 남긴다.

초딩 설명: pnix-clj는 놀이 전에 체크리스트를 읽고, 놀이가 끝나면 영수증을 붙인다. 성공하면 초록불, 위험하면 멈춤, 멈춘 이유는 쪽지로 남긴다. 그래서 사람이 다시 보거나 CI가 자동으로 판단하기 쉽다.

### 실무 응용을 쉽게 말하면

이전 설명: compiler QA, 금융 규칙 검증, 보험 요율 계산, 안전한 설정 검사, 교육용 proof tool에 쓴다.

초딩 설명: 회사에서는 사람이 모든 코드를 매번 눈으로 확인하기 어렵다. 그래서 이 예제처럼 작은 검사표를 만들어 두면, AI나 자동화가 만든 결과를 바로 믿지 않고 `통과`, `사람이 봐야 함`, `막아야 함`으로 나눌 수 있다.

```clojure
;; 26-arithmetic-proof - arithmetic proof를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:test-case generated-source
 :expected oracle-answer
 :actual actual-answer
 :pass? same?
 :counterexample (when-not same? generated-source)}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
