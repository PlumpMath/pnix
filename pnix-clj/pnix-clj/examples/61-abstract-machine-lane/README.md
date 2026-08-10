# 61. Abstract machine lane

## 무엇을 보여주나

tree-walk evaluator와 별도로, defunctionalized control stack을 가진 abstract machine lane이 같은 fragment에서 evaluator와 같은 결과를 내고, 지원하지 않는 op는 추측하지 않고 held 하는 예제다.

## plain Clojure의 한계

직접 재귀 interpreter를 만들 수는 있지만, evaluator와 공유 value algebra를 쓰는 derived machine lane, unsupported-op gate, constant-stack control boundary를 함께 제공하지 않는다.

## pnix-clj 방식

`pnix-clj.machine/eval-source`는 parser output을 machine fragment에서 실행하고, evaluator와 같은 결과 또는 같은 held reason을 내도록 gate-pinned 되어 있다. 예제는 작은 differential과 아직 남은 dynamic path binding frontier를 보여준다.

## 어디에 쓰나

deep control, CEK/Krivine-style machine, evaluator-derived execution lane을 이해하거나 machine fragment 확장 회귀를 볼 때 쓴다.


## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(ns abstract-machine-lane-limit)
(defn eval-expr
  [expr env]
  (case (:op expr)
    :int (:value expr)
    :var (get env (:name expr))
    :add (+ (eval-expr (:left expr) env)
            (eval-expr (:right expr) env))))
(def expr
  {:op :add
   :left {:op :var :name "x"}
   :right {:op :int :value 2}})
(println "plain recursive interpreter value:" (eval-expr expr {"x" 40}))
(println "derived from pnix evaluator?:" false)
(println "unsupported-op gate?:" false)
(assert (= 42 (eval-expr expr {"x" 40})))
(println)
(println "결론: hand-written recursive interpreter는 pnix evaluator와 공유된 abstract-machine lane 증거가 아니다.")
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.machine :as machine]))
(defn comparable
  [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))
(doseq [source ["let f = x: x + 1; in f 41"
                "builtins.length [ 1 2 3 ]"
                "{ a = 1; b = [ 2 3 ]; }.b"
                "assert false; 42"]]
  (let [evaluator (pnix/eval-source source)
        machine (machine/eval-source source)]
    (println "source:" source)
    (println "evaluator:" (comparable evaluator))
    (println "machine:" (comparable machine))
    (assert (= (comparable evaluator)
               (comparable machine)))))
(let [unsupported (machine/eval-source "let k = \"b\"; in { a.${k} = 1; }")]
  (println "unsupported dynamic path binding:"
           (select-keys unsupported [:status :reason]))
  (assert (= :held (:status unsupported)))
;; ...
```

비교하면, limit 파일은 plain Clojure 값/예외/수동 상태를 직접 만든다. pnix-clj 파일은 같은 문제를 pnix-clj API에 태우고 `assert`로 `:status`, hash, receipt, gate verdict 같은 증거를 확인한다. 전체 실행 코드는 같은 디렉터리의 두 `.clj` 파일을 보면 된다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 61. Abstract machine lane 문제를 어떻게 흉내 내는지 본다.
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
- pnix-clj 쪽 핵심: pnix-clj 쪽은 direct eval, lowering, clj-meta, px-runtime, machine 결과를 비교하고, 아직 지원하지 않는 fragment는 추측 대신 `:held` frontier로 둔다. M7f 이후 `import`는 89번 예제에서 다루며, 여기서는 남은 dynamic path binding frontier를 보여준다.
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
;; 61. Abstract machine lane를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:program source
 :direct-answer direct
 :compiled-answer compiled
 :machine-answer machine
 :same? (= direct compiled machine)}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
