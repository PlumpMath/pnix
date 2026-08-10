;;; plain Clojure의 한계 - map assoc은 dynamic attr key collision/type semantics를 모른다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/77-dynamic-attr-key-strictness/limit_clojure.clj

(ns dynamic-attr-key-strictness-limit)

(def overwritten
  (assoc {"a" 1} "a" 2))

(def numeric-key
  (assoc {} 1 "one"))

(println "Clojure assoc collision value:" (get overwritten "a"))
(println "numeric key accepted:" (get numeric-key 1))
(println "Nix-style dynamic key construction-time error?:" false)
(println "select-or catch taxonomy?:" false)

(assert (= 2 (get overwritten "a")))
(assert (= "one" (get numeric-key 1)))

(println)
(println "결론: Clojure map은 overwrite와 임의 key를 허용하므로 D20의 duplicate/non-string dynamic attr semantics를 표현하지 않는다.")
