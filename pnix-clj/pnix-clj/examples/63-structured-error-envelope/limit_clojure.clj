;;; plain Clojure의 한계 - exception string은 pnix lane error schema가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/63-structured-error-envelope/limit_clojure.clj

(ns structured-error-envelope-limit)

(def caught
  (try
    (/ 1 0)
    (catch Throwable t
      {:class (.getName (class t))
       :message (.getMessage t)})))

(println "caught:" caught)
(println "schema?:" false)
(println "phase/reason envelope?:" false)

(assert (string? (:class caught)))

(println)
(println "결론: plain catch map은 pnix-clj report들이 공유하는 held error envelope가 아니다.")

