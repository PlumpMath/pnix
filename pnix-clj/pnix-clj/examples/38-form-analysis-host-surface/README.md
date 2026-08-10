# 38. Form analysis host surface

## 무엇을 보여주나

Clojure form을 실행하기 전에 tools.analyzer AST로 host interop 표면을 분류하는 예제다.

## plain Clojure의 한계

plain `eval`은 `(System/getProperty ...)` 같은 host call을 실행해 버린 뒤 값만 돌려준다. 실행 전에 "이 form은 pure core인가, host interop인가"를 구조적으로 분류하지 않는다.

## pnix-clj 방식

`pnix-clj.form-analysis/analyze-form`은 analyzer AST node를 걸어 pure arithmetic과 host static/instance/new 호출을 분리하고, report는 corpus 전체의 기대 분류를 검증한다.

## 어디에 쓰나

Clojure form을 pnix로 투영하거나 safe-eval 전에 host surface를 gate해야 할 때 쓴다.


## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(ns form-analysis-host-surface-limit)
(def host-form
  '(System/getProperty "user.home"))
(def value
  (eval host-form))
(println "form:" (pr-str host-form))
(println "value class:" (.getName (class value)))
(println "pre-exec host surface classification?:" false)
(assert (string? value))
(println)
(println "결론: plain eval은 host interop 표면을 실행 전 AST 수준에서 분류하지 않는다.")
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(ns pnix-clj-way
  (:require [pnix-clj.form-analysis :as fa]))
(let [pure (fa/analyze-form '(+ (* 2 3) 4))
      host (fa/analyze-form '(System/getProperty "user.home"))
      report (fa/report)]
  (println "pure status/core?:" (:status pure) (:pure-core? pure))
  (println "host status/core?:" (:status host) (:pure-core? host))
  (println "host interop:" (:host-interop host))
  (println "report accepted/rejected:" (:accepted report) (:rejected report))
  (assert (= :ok (:status pure)))
  (assert (= true (:pure-core? pure)))
  (assert (= :ok (:status host)))
  (assert (= false (:pure-core? host)))
  (assert (seq (:host-interop host)))
  (assert (= :ok (:status report))))
(println)
(println "결론: pnix-clj는 form 실행 전 host surface를 구조화해 gate 입력으로 쓸 수 있다.")
(shutdown-agents)
```

비교하면, limit 파일은 plain Clojure 값/예외/수동 상태를 직접 만든다. pnix-clj 파일은 같은 문제를 pnix-clj API에 태우고 `assert`로 `:status`, hash, receipt, gate verdict 같은 증거를 확인한다. 전체 실행 코드는 같은 디렉터리의 두 `.clj` 파일을 보면 된다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 38. Form analysis host surface 문제를 어떻게 흉내 내는지 본다.
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

- plain 쪽 한계: plain Clojure 쪽은 `eval`, host API, Java object, atom mutation을 직접 다루기 때문에 실행 후에야 위험을 알 수 있다.
- pnix-clj 쪽 핵심: pnix-clj 쪽은 실행 전 purity/capability gate를 두고, 허용된 crossing도 witness hash와 effect-class를 남긴다. deny는 exception이 아니라 `:held` verdict다.
- 판단 기준: 신뢰 경계를 넘는 코드나 host object를 값처럼 믿지 않고, capability·loss-status·witness를 붙여 승인 가능한 crossing으로 만든다.

## 산업/실무 적용

적용 가능한 개발 도메인 예시는 다음과 같다.

- enterprise plugin marketplace
- AI agent tool sandbox
- fintech admin automation
- internal developer portal
- data-platform UDF gate

실무 흐름으로 바꾸면 이렇게 쓴다. 기본 capability set은 `#{:pure}`로 두고, 사용자가 승인한 effect만 추가한다. 결과는 audit log에 witness와 함께 저장한다.

```clojure
;; 실제 서비스 코드에서는 아래 map을 DB row, CI artifact, PR comment,
;; deployment approval payload 같은 형태로 저장하면 된다.
{:domain :agent-tool-call
 :requested-effect :file-read
 :granted-capabilities #{:pure}
 :decision (:status crossing)
 :evidence (:witness crossing)}
```

업체나 팀 관점에서 보면, 이 예제는 라이브러리 기능 하나를 보여주는 것이 아니라 "자동화가 결정을 내리기 전에 어떤 증거를 요구할 것인가"를 정하는 작은 패턴이다.


## 초딩 설명

### 이 예제가 말하는 것

이전 설명: 신뢰 경계를 넘는 코드나 host object를 값처럼 믿지 않고, capability·loss-status·witness를 붙여 승인 가능한 crossing으로 만든다.

초딩 설명: 모르는 사람이 우리 집 컴퓨터를 만지려 하면 바로 들여보내면 안 된다. 먼저 문지기가 '파일 읽어도 돼?', '환경변수 봐도 돼?' 하고 물어본다. 허락받은 일만 시키고, 무엇을 했는지 출입증에 적어 둔다.

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

이전 설명: plain Clojure는 열쇠를 바로 주는 것과 비슷하다. 코드가 파일을 읽거나 Java 객체를 만져도, 나중에야 알 수 있다.

초딩 설명: plain Clojure는 장난감을 바로 움직여 보는 것과 같다. 빠르고 쉽지만, 장난감이 어디를 건드렸는지, 같은 놀이를 내일 다시 해도 같은 결과가 나오는지, 누가 허락했는지 적어 두지 않는다.

### pnix-clj 쪽을 쉽게 말하면

이전 설명: pnix-clj는 `gate`라는 문지기를 둔다. 허락이 없으면 `:held`로 멈추고, 허락이 있으면 `witness`라는 출입 기록을 남긴다.

초딩 설명: pnix-clj는 놀이 전에 체크리스트를 읽고, 놀이가 끝나면 영수증을 붙인다. 성공하면 초록불, 위험하면 멈춤, 멈춘 이유는 쪽지로 남긴다. 그래서 사람이 다시 보거나 CI가 자동으로 판단하기 쉽다.

### 실무 응용을 쉽게 말하면

이전 설명: AI agent tool, 사내 plugin, 데이터 UDF, admin 자동화처럼 외부 코드가 회사 컴퓨터 자원에 접근할 때 쓴다.

초딩 설명: 회사에서는 사람이 모든 코드를 매번 눈으로 확인하기 어렵다. 그래서 이 예제처럼 작은 검사표를 만들어 두면, AI나 자동화가 만든 결과를 바로 믿지 않고 `통과`, `사람이 봐야 함`, `막아야 함`으로 나눌 수 있다.

```clojure
;; 38. Form analysis host surface를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:request "파일 읽기"
 :allowed? false
 :status :held
 :reason :capability-denied
 :record :witness}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
