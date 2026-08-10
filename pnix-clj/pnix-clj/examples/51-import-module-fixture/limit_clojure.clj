;;; plain Clojure의 한계 - load/slurp는 실제 파일 경로에 묶이고 lane receipt가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/51-import-module-fixture/limit_clojure.clj

(ns import-module-fixture-limit)

(def modules
  {"./m" "1 + 2"})

(def attempted
  (try
    (slurp "./m")
    (catch Throwable t
      {:status :error
       :class (.getName (class t))})))

(println "in-memory module map:" modules)
(println "plain slurp ./m:" attempted)
(println "cross-lane import receipt?:" false)

(assert (= "1 + 2" (get modules "./m")))
(assert (= :error (:status attempted)))

(println)
(println "결론: plain file IO는 import fixture map과 evaluator/lowering/runtime receipt를 묶지 않는다.")

