;;; plain Clojure의 한계 - load-string은 pnix stage lockin fixture receipt가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/50-stage7-core-lockins/limit_clojure.clj

(ns stage7-core-lockins-limit)

(def value
  (load-string "(+ 40 2)"))

(println "load-string value:" value)
(println "stage7 fixture lineage?:" false)
(println "cross-lane receipt?:" false)

(assert (= 42 value))

(println)
(println "결론: plain load-string은 실행값만 만들며 stage7 lockin fixture 증거가 아니다.")

