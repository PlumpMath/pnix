;;; plain Clojure의 한계 - ProcessBuilder는 plan/gate 없이 바로 실행할 수 있다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/55-stage15-control-plan/limit_clojure.clj

(ns stage15-control-plan-limit)

(def commands
  [{:id :compiler-smoke
    :command ["clojure" "-M:compiler-smoke"]}])

(println "manual command list:" commands)
(println "input hashes?:" false)
(println "read-only backend policy?:" false)
(println "default held until explicitly executed?:" false)

(assert (= :compiler-smoke (:id (first commands))))

(println)
(println "결론: plain command list는 backend input hash와 owner/manual gate를 구조화하지 않는다.")

