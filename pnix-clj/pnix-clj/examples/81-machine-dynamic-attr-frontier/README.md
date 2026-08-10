# 81. Machine dynamic attr frontier closed

## 무엇을 보여주나

예전에는 evaluator만 D20 dynamic attr key를 처리하고 machine은 `:machine-unsupported-op`로 멈췄다. M7e 이후에는 machine도 dynamic key를 native로 실행한다. 이 예제는 evaluator와 machine이 값과 held reason까지 같은지 확인한다.

## plain Clojure의 한계

loose interpreter는 dynamic key를 `str`로 바꿔 map에 넣고 충돌을 조용히 overwrite하기 쉽다. 그러면 real Nix의 `:duplicate-attr`나 `:dynamic-attr-key-not-string` 같은 이유가 사라진다.

## pnix-clj 방식

`pnix/eval-source`와 `machine/eval-source`를 같은 source에 태우고 `[:ok value]` 또는 `[status reason]` 모양으로 비교한다. dynamic select, has-attr, duplicate, non-string key가 모두 같은 결과여야 한다.

## 어디에 쓰나

AI가 만든 config key, Nix module generator, feature flag map, tenant option map처럼 key가 계산식으로 만들어지는 곳에서 machine lane까지 같은 semantics를 유지하는지 확인한다.

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(defn loose-attrset [pairs]
  (reduce (fn [m [k v]] (assoc m (str k) v)) {} pairs))

(assert (= 1 (get (loose-attrset [["x" 1]]) "x")))
(assert (= 2 (get (loose-attrset [["a" 1] ["a" 2]]) "a")))
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(doseq [source cases]
  (let [evaluator (pnix/eval-source source)
        machine (machine/eval-source source)]
    (assert (= (comparable evaluator)
               (comparable machine)))))
```

비교하면, limit 파일은 key 충돌을 덮어쓴다. pnix-clj 파일은 evaluator와 machine이 `:duplicate-attr`, `:dynamic-attr-key-not-string`까지 같은지 확인한다.

## 코드 해설

이전 설명: M7e 이후 machine은 dynamic attr key를 native로 실행하고 evaluator와 같은 결과를 내야 한다.

초딩 설명: 예전에는 선생님 한 명만 어려운 이름표를 읽을 수 있었다. 이제 두 선생님, evaluator와 machine이 둘 다 같은 이름표를 읽고 같은 답을 말해야 한다.

```clojure
;; comparable은 결과를 쉽게 비교하려고 만든 작은 변환기다.
;; 성공이면 [:ok 값]
;; 멈춤이면 [:held 이유]
(defn comparable [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))

;; 같은 source를 두 길로 실행한다.
;; 둘이 다르면 machine이 evaluator와 다른 뜻으로 실행한다는 뜻이다.
(assert (= (comparable evaluator)
           (comparable machine)))
```

## 산업/실무 적용

- SRE/platform 설정 검토
- Kubernetes/Nix module generator
- SaaS tenant config
- feature flag rollout
- AI coding-agent PR review

실무에서는 generated config를 바로 merge하지 말고 아래처럼 검사 row를 남긴다.

```clojure
{:domain :generated-config-key
 :source source
 :evaluator (comparable evaluator)
 :machine (comparable machine)
 :safe-to-continue? (= (comparable evaluator)
                       (comparable machine))}
```

## 초딩 설명

### 이 예제가 말하는 것

이전 설명: dynamic key를 machine도 실행하고 evaluator와 같은 value/held reason을 내는지 확인한다.

초딩 설명: 같은 문제를 두 친구에게 풀게 한다. 둘 다 같은 답을 내거나, 둘 다 같은 이유로 멈추면 믿을 수 있다.

### 코드 쉽게 읽기

```clojure
;; :ok은 초록불이다.
;; :held는 잠깐 멈춤이다.
;; :reason은 왜 멈췄는지 적힌 쪽지다.
```

`assert`는 “두 친구 답이 같은지” 확인하는 줄이다.

### 응용을 쉽게 말하면

AI가 설정표 key를 만들어 왔을 때, 이름이 겹치면 덮어쓰지 말고 “이름 중복!”이라고 멈춰야 한다. 이 예제는 그 멈춤 이유까지 machine이 똑같이 말하는지 본다.
