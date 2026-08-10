;;; plain Clojure의 한계 - 단일 eval은 parse/eval hash stability audit가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/21-determinism-audit/limit_clojure.clj

(ns determinism-audit-limit)

(def source "(+ 1 2)")
(def value (eval (read-string source)))

(println "value:" value)
(println "missing: corpus-wide repeated parse/eval hash stability")

(assert (= 3 value))

(println)
(println "결론: 한 번 실행해 성공한 값은 deterministic runtime evidence가 아니다.")
