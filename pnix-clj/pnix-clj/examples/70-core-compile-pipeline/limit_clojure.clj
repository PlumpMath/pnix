;;; plain Clojure의 한계 - eval은 compile pipeline receipt가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/70-core-compile-pipeline/limit_clojure.clj

(ns core-compile-pipeline-limit)

(def value
  (eval '(+ 1 2)))

(println "plain eval value:" value)
(println "source hash?:" false)
(println "lowered form hash?:" false)
(println "compile receipt?:" false)

(assert (= 3 value))

(println)
(println "결론: plain eval은 parse/lower/compile 증거를 한 receipt로 묶지 않는다.")

