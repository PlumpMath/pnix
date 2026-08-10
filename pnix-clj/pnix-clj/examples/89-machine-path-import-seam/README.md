# 89. Machine path/import seam

## 무엇을 보여주나

M7f 이후 machine은 `./x` 같은 path literal과 `import ./m` op를 fragment 안에서 다룬다. path literal은 evaluator와 같은 값이 나와야 하고, import는 resolver가 없으면 `:import-evaluation-not-wired`로 멈추며, resolver가 있으면 그 seam을 통해 값을 받는다.

## plain Clojure의 한계

plain Clojure에서는 path를 문자열로 보고 module map에서 꺼내기 쉽다. 하지만 path value와 import resolver boundary, unwired held reason, import provenance를 구분하지 않는다.

## pnix-clj 방식

path source는 evaluator와 machine을 비교한다. import source는 resolver가 없을 때 held reason을 확인하고, resolver를 명시적으로 바인딩했을 때만 실행한다.

## 어디에 쓰나

Nix module loader, in-memory import fixture, build graph resolver, package config evaluator, runtime migration에서 import 경계가 명확한지 확인할 때 쓴다.

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(def modules {"./m" "1 + 2"})

(defn fake-import [path]
  (get modules path))

(assert (= "1 + 2" (fake-import "./m")))
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(let [unwired (machine/eval-source "import ./m")]
  (assert (= :held (:status unwired)))
  (assert (= :import-evaluation-not-wired (:reason unwired))))

(binding [evaluator/*import-context* {:example :machine-import}
          evaluator/*import-resolver* resolver]
  (assert (= 42 (:value (machine/run-ast ast)))))
```

비교하면, limit 파일은 import를 map lookup으로 흉내 낸다. pnix-clj 파일은 import가 resolver seam 없이는 실행되지 않고, seam이 있을 때만 실행된다는 경계를 확인한다.

## 코드 해설

이전 설명: path literal은 machine이 직접 처리하고, import는 shared resolver seam으로만 실행한다.

초딩 설명: path는 “어느 방으로 가라”는 주소표다. import는 그 방 문을 여는 일이다. 문 열쇠가 없으면 멈추고, 열쇠가 있으면 안의 값을 가져온다.

```clojure
;; resolver가 없으면 문을 못 연다.
(assert (= :import-evaluation-not-wired (:reason unwired)))

;; resolver를 넣어 주면 문을 열고 값을 받는다.
(binding [evaluator/*import-resolver* resolver]
  (machine/run-ast ast))
```

## 산업/실무 적용

- build graph/module loader
- package manager resolver
- monorepo config import
- CI fixture import
- runtime migration import parity

```clojure
{:domain :module-loader
 :import "./m"
 :resolver-present? true
 :status (:status imported)
 :value (:value imported)}
```

## 초딩 설명

이전 설명: import는 resolver가 있을 때만 실행하고 없으면 held reason을 남긴다.

초딩 설명: 교실 문을 열려면 열쇠가 필요하다. 열쇠가 없으면 “열쇠 없음” 쪽지를 붙이고 멈춘다. 열쇠가 있으면 들어가서 물건을 가져온다.
