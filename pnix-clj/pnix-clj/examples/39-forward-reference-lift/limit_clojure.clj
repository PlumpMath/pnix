;;; plain Clojure의 한계 - let은 순차 binding이라 forward reference contract가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/39-forward-reference-lift/limit_clojure.clj

(ns forward-reference-lift-limit)

(def result
  (try
    (eval '(let [x y
                 y 41]
             x))
    (catch Throwable t
      {:status :error
       :class (.getName (class t))
       :message (.getMessage t)})))

(println "plain let forward reference result:" result)

(assert (= :error (:status result)))

(println)
(println "결론: plain Clojure는 forward reference를 fixture/lane contract로 lift해 검증하지 않는다.")

