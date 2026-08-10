# 55. Stage15 control plan

## 무엇을 보여주나

clj-meta stage15 이상 backend command를 자동 실행하지 않고, 읽기 전용 control plan과 입력 hash, command 목록으로만 노출하는 예제다.

## plain Clojure의 한계

`ProcessBuilder`로 명령을 바로 실행하면 어떤 backend source hash를 기준으로 했는지, write policy가 무엇인지, owner/manual gate가 어디인지 분리되지 않는다.

## pnix-clj 방식

`pnix-clj.stage15/control-plan`은 clj-meta backend root, stage range, input hashes, command list를 `:held` plan으로 반환한다. 실제 실행은 별도 `stage15-execute` entrypoint이며 예제에서는 호출하지 않는다.

## 어디에 쓰나

외부 backend gate를 사람에게 보여주되, examples 실행 중 자동 명령 실행이나 backend mutation을 막아야 할 때 쓴다.


## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(ns stage15-control-plan-limit)
(def commands
  [{:id :compiler-smoke
    :command ["clojure" "-M:compiler-smoke"]}])
(println "manual command list:" commands)
(println "input hashes?:" false)
(println "read-only backend policy?:" false)
(println "default held until explicitly executed?:" false)
(assert (= :compiler-smoke (:id (first commands))))
(println)
(println "결론: plain command list는 backend input hash와 owner/manual gate를 구조화하지 않는다.")
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(ns pnix-clj-way
  (:require [pnix-clj.stage15 :as stage15]))
(let [plan (stage15/control-plan)]
  (println "plan:" (select-keys plan [:kind :status :reason :backend :stage-range :write-policy]))
  (println "inputs:" (mapv #(select-keys % [:path :exists :hash]) (:inputs plan)))
  (println "commands:" (mapv #(select-keys % [:id :purpose]) (:commands plan)))
  (assert (= :stage15-control-plan (:kind plan)))
  (assert (= :held (:status plan)))
  (assert (= :stage15-gates-not-executed (:reason plan)))
  (assert (= :read-only-backend (:write-policy plan)))
  (assert (pos? (:command-count plan)))
  (assert (every? :exists (:inputs plan))))
(println)
(println "결론: pnix-clj stage15 plan은 외부 backend 실행을 자동화하지 않고 owner-visible plan으로 고정한다.")
(shutdown-agents)
```

비교하면, limit 파일은 plain Clojure 값/예외/수동 상태를 직접 만든다. pnix-clj 파일은 같은 문제를 pnix-clj API에 태우고 `assert`로 `:status`, hash, receipt, gate verdict 같은 증거를 확인한다. 전체 실행 코드는 같은 디렉터리의 두 `.clj` 파일을 보면 된다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 55. Stage15 control plan 문제를 어떻게 흉내 내는지 본다.
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

- plain 쪽 한계: plain Clojure 쪽은 함수를 eval/compile할 수는 있지만, 어느 projection/lowering/machine 경로가 같은 의미인지 별도 증거를 남기지 않는다.
- pnix-clj 쪽 핵심: pnix-clj 쪽은 direct eval, lowering, clj-meta, px-runtime, machine 결과를 비교하고, 지원하지 않는 fragment는 추측 대신 `:held` frontier로 둔다.
- 판단 기준: DSL source가 interpreter, lowered form, compiled path, machine lane 사이에서 같은 의미를 유지하는지 확인한다.

## 산업/실무 적용

적용 가능한 개발 도메인 예시는 다음과 같다.

- internal DSL compiler
- build-system optimizer
- low-code rule engine
- language migration
- edge policy compiler

실무 흐름으로 바꾸면 이렇게 쓴다. 새 compiler optimization이나 machine fragment를 넣을 때, direct evaluator와 compiled/machine lane의 comparable result를 CI에서 비교한다.

```clojure
;; 실제 서비스 코드에서는 아래 map을 DB row, CI artifact, PR comment,
;; deployment approval payload 같은 형태로 저장하면 된다.
{:domain :dsl-compiler-ci
 :source source
 :direct (:eval-result receipt)
 :compiled (:clj-meta-result receipt)
 :machine machine-result
 :same? (= direct compiled)}
```

업체나 팀 관점에서 보면, 이 예제는 라이브러리 기능 하나를 보여주는 것이 아니라 "자동화가 결정을 내리기 전에 어떤 증거를 요구할 것인가"를 정하는 작은 패턴이다.


## 초딩 설명

### 이 예제가 말하는 것

이전 설명: DSL source가 interpreter, lowered form, compiled path, machine lane 사이에서 같은 의미를 유지하는지 확인한다.

초딩 설명: 같은 이야기를 한국어, 영어, 그림책, 연극으로 바꿔도 내용이 같아야 한다. 이 예제는 프로그램을 여러 모양으로 바꿔도 답이 같은지 확인한다.

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

이전 설명: plain Clojure는 한 가지 방법으로 실행하거나 컴파일할 수는 있지만, 다른 길로 바꿔도 같은 뜻인지 자동으로 비교해 주지 않는다.

초딩 설명: plain Clojure는 장난감을 바로 움직여 보는 것과 같다. 빠르고 쉽지만, 장난감이 어디를 건드렸는지, 같은 놀이를 내일 다시 해도 같은 결과가 나오는지, 누가 허락했는지 적어 두지 않는다.

### pnix-clj 쪽을 쉽게 말하면

이전 설명: pnix-clj는 여러 길을 동시에 걸어 본다. 직접 실행한 답, 바꿔서 실행한 답, machine이 낸 답을 비교하고 다르면 멈춘다.

초딩 설명: pnix-clj는 놀이 전에 체크리스트를 읽고, 놀이가 끝나면 영수증을 붙인다. 성공하면 초록불, 위험하면 멈춤, 멈춘 이유는 쪽지로 남긴다. 그래서 사람이 다시 보거나 CI가 자동으로 판단하기 쉽다.

### 실무 응용을 쉽게 말하면

이전 설명: 사내 DSL compiler, rule engine, build optimizer, low-code platform, runtime rewrite에서 새 최적화가 뜻을 바꾸지 않는지 볼 때 쓴다.

초딩 설명: 회사에서는 사람이 모든 코드를 매번 눈으로 확인하기 어렵다. 그래서 이 예제처럼 작은 검사표를 만들어 두면, AI나 자동화가 만든 결과를 바로 믿지 않고 `통과`, `사람이 봐야 함`, `막아야 함`으로 나눌 수 있다.

```clojure
;; 55. Stage15 control plan를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:program source
 :direct-answer direct
 :compiled-answer compiled
 :machine-answer machine
 :same? (= direct compiled machine)}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
