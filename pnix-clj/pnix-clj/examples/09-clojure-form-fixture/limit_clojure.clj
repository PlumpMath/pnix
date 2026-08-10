;;; plain Clojure의 한계 - form eval은 host 값만 만들고 projection fixture receipt가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/09-clojure-form-fixture/limit_clojure.clj

(ns clojure-form-fixture-limit)

(def form-source "(let [x 40] (+ x 2))")

(let [form (read-string form-source)
      value (eval form)]
  (println "form source:" form-source)
  (println "host value:" value)
  (println "missing: clj-meta compiled value, projection term, fixture hash")
  (assert (= 42 value)))

(println)
(println "결론: plain Clojure form 실행은 host 값만 확인하고, projection/fixture evidence를 남기지 않는다.")
