# 07-clojure-macro-over-pnix — Clojure macroexpand 결과를 pnix tower로 검증

## 무엇을 보여주나

plain Clojure macro는 macroexpand 후 eval할 수 있다. 하지만 그 확장 결과가 pnix로 투영 가능한지,
pnix tower에서 같은 값으로 collapse되는지, 어떤 witness를 남기는지는 기본으로 알 수 없다.

pnix-clj 방식은 macro 자체를 pnix로 억지로 옮기지 않는다.

먼저 Clojure가 자기 방식으로 macroexpand 한다.

그 다음 expanded Clojure form이 whitelisted expression subset이면 pnix source로 synthesize한다.

마지막으로 synthesized pnix source를 tower로 올려 clj-meta 평가값과 같은지 검증한다.

## 왜 필요한가

Hy 쪽의 macro 예제는 Hy macro tower를 보여준다.

pnix-clj에서는 같은 일을 Hy식으로 복제하면 안 된다. 대응물은 Clojure macroexpand, syntax-quote,
Clojure form, clj-meta bytecode receipt, 그리고 pnix tower다.

즉 핵심은 이것이다.

    macro definition을 pnix로 번역하는 것이 아니다.
    macro expansion result가 pnix 의미로 검증 가능한지 확인하는 것이다.

## 쉽게 말하면

plain Clojure:

    macroexpand 했다.
    eval 했다.
    값이 나왔다.

pnix-clj:

    macroexpand 했다.
    expanded form을 pnix로 synthesize했다.
    pnix tower에서 collapse했다.
    clj-meta 값과 tower 값이 같다.
    witness가 남았다.

## 실행

pnix-clj 디렉터리에서:

    clojure -M examples/07-clojure-macro-over-pnix/limit_clojure.clj
    clojure -M examples/07-clojure-macro-over-pnix/pnix_clj_way.clj

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(ns clojure-macro-over-pnix-limit)
(defmacro add-two
  [x]
  `(+ ~x 2))
(defmacro choose
  [condition then-expr else-expr]
  `(if ~condition ~then-expr ~else-expr))
(let [form '(add-two 40)
      expanded (macroexpand form)
      value (eval expanded)]
  (println "macro form:" form)
  (println "expanded:" expanded)
  (println "eval value:" value)
  (println "pnix source:" nil)
  (println "tower collapse:" nil)
  (println "tower witness:" nil)
  (assert (= '(clojure.core/+ 40 2) expanded))
  (assert (= 42 value)))
;; ...
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(ns pnix-clj-way
  (:require [clojure.walk :as walk]
            [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.synthesize :as synth]
            [pnix-clj.tower :as tower]))
(defmacro add-two
  [x]
  `(+ ~x 2))
(defmacro choose
  [condition then-expr else-expr]
  `(if ~condition ~then-expr ~else-expr))
(defn unqualify-core-symbols
  [form]
  (walk/postwalk
   (fn [x]
     (if (and (symbol? x)
              (= "clojure.core" (namespace x)))
       (symbol (name x))
       x))
   form))
(defn macro-projection-row
  [id form]
;; ...
```

비교하면, limit 파일은 plain Clojure 값/예외/수동 상태를 직접 만든다. pnix-clj 파일은 같은 문제를 pnix-clj API에 태우고 `assert`로 `:status`, hash, receipt, gate verdict 같은 증거를 확인한다. 전체 실행 코드는 같은 디렉터리의 두 `.clj` 파일을 보면 된다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 07-clojure-macro-over-pnix — Clojure macroexpand 결과를 pnix tower로 검증 문제를 어떻게 흉내 내는지 본다.
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

- plain 쪽 한계: plain Clojure 쪽은 데이터 구조를 만들고 출력하지만, 어느 단계에서 어떤 보장이 생겼는지 분리하지 않는다.
- pnix-clj 쪽 핵심: pnix-clj 쪽은 각 단계 결과를 map으로 남기고 `assert`로 status/value/hash/reason을 고정한다.
- 판단 기준: 일반 데이터/소스 변환도 값만 확인하지 말고, parse/normalize/validate/report 단계별로 증거를 남긴다.

## 산업/실무 적용

적용 가능한 개발 도메인 예시는 다음과 같다.

- data pipeline validation
- workflow automation
- configuration management
- developer tooling
- audit-friendly backend jobs

실무 흐름으로 바꾸면 이렇게 쓴다. 입력 → normalize → validate → execute → receipt 순서로 row를 만들고, 실패는 exception 대신 held reason으로 저장한다.

```clojure
;; 실제 서비스 코드에서는 아래 map을 DB row, CI artifact, PR comment,
;; deployment approval payload 같은 형태로 저장하면 된다.
{:domain :data-pipeline
 :input source
 :phase phase
 :status (:status result)
 :reason (:reason result)
 :evidence evidence}
```

업체나 팀 관점에서 보면, 이 예제는 라이브러리 기능 하나를 보여주는 것이 아니라 "자동화가 결정을 내리기 전에 어떤 증거를 요구할 것인가"를 정하는 작은 패턴이다.


## 초딩 설명

### 이 예제가 말하는 것

이전 설명: 일반 데이터/소스 변환도 값만 확인하지 말고, parse/normalize/validate/report 단계별로 증거를 남긴다.

초딩 설명: 숙제를 제출할 때 답만 내지 말고, 이름 썼는지, 풀이가 있는지, 선생님 도장이 있는지 차례로 확인하는 것이다.

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

이전 설명: plain Clojure는 데이터를 만들고 출력할 수 있지만, 어느 단계에서 무엇을 확인했는지 한눈에 보기 어렵다.

초딩 설명: plain Clojure는 장난감을 바로 움직여 보는 것과 같다. 빠르고 쉽지만, 장난감이 어디를 건드렸는지, 같은 놀이를 내일 다시 해도 같은 결과가 나오는지, 누가 허락했는지 적어 두지 않는다.

### pnix-clj 쪽을 쉽게 말하면

이전 설명: pnix-clj는 각 단계를 작은 카드로 만든다. 카드마다 `:status`와 `:reason`이 있어서 어디서 멈췄는지 바로 보인다.

초딩 설명: pnix-clj는 놀이 전에 체크리스트를 읽고, 놀이가 끝나면 영수증을 붙인다. 성공하면 초록불, 위험하면 멈춤, 멈춘 이유는 쪽지로 남긴다. 그래서 사람이 다시 보거나 CI가 자동으로 판단하기 쉽다.

### 실무 응용을 쉽게 말하면

이전 설명: 데이터 파이프라인, workflow 자동화, 설정 관리, 개발자 도구, 감사 가능한 backend job에 쓴다.

초딩 설명: 회사에서는 사람이 모든 코드를 매번 눈으로 확인하기 어렵다. 그래서 이 예제처럼 작은 검사표를 만들어 두면, AI나 자동화가 만든 결과를 바로 믿지 않고 `통과`, `사람이 봐야 함`, `막아야 함`으로 나눌 수 있다.

```clojure
;; 07-clojure-macro-over-pnix — Clojure macroexpand 결과를 pnix tower로 검증를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:input source
 :step :validate
 :status :ok
 :reason nil
 :next :execute}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
