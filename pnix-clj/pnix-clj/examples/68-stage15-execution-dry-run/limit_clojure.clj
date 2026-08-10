;;; plain Clojure의 한계 - fake command map은 stage15 execution receipt가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/68-stage15-execution-dry-run/limit_clojure.clj

(ns stage15-execution-dry-run-limit)

(def fake-result
  {:command :compiler-smoke
   :exit 0})

(println "fake command result:" fake-result)
(println "control-plan hash?:" false)
(println "selected command receipt hash?:" false)

(assert (zero? (:exit fake-result)))

(println)
(println "결론: plain fake result는 stage15 plan과 결합된 execution receipt가 아니다.")

