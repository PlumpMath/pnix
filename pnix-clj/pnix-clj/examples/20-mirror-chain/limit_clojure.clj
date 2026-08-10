;;; plain Clojure의 한계 - 반복 실행이 같은 값이어도 drift event chain을 남기지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/20-mirror-chain/limit_clojure.clj

(ns mirror-chain-limit)

(def source "(+ 40 2)")
(def values (repeatedly 3 #(eval (read-string source))))

(println "values:" values)
(println "stable?:" (apply = values))
(println "missing: per-run event, first divergent run, hash-chain verification")

(assert (apply = values))

(println)
(println "결론: plain repeated eval은 temporal mirror-chain evidence를 남기지 않는다.")
