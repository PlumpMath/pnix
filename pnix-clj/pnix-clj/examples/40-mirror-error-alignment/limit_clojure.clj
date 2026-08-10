;;; plain Clojure의 한계 - 하나의 exception은 잡지만 multi-lane error alignment는 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/40-mirror-error-alignment/limit_clojure.clj

(ns mirror-error-alignment-limit)

(def caught
  (try
    (/ 1 0)
    (catch Throwable t
      {:class (.getName (class t))
       :message (.getMessage t)})))

(println "caught exception:" caught)
(println "has evaluator/runtime/mirror alignment?:" false)

(assert (string? (:class caught)))

(println)
(println "결론: plain try/catch는 실패를 구조화하지만 cross-lane error receipt까지 맞추지는 않는다.")

