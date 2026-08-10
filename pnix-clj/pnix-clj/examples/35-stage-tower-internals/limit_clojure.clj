;;; plain Clojure의 한계 — eval은 값은 주지만 self-hosting tower의
;;; layer/pair/collapse witness 구조를 기본으로 만들지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/35-stage-tower-internals/limit_clojure.clj

(ns stage-tower-internals-limit)

(def source
  "(let [x 40] (+ x 2))")

(let [form (read-string source)
      value (eval form)]
  (println "source:" source)
  (println "read form:" form)
  (println "eval value:" value)
  (println "tower layers:" nil)
  (println "tower pairs:" nil)
  (println "collapse witness:" nil)
  (assert (= 42 value)))

(println)
(println "결론: plain Clojure eval은 값은 주지만 self-hosting tower 내부 layer/pair/witness를 기본으로 주지 않는다.")
