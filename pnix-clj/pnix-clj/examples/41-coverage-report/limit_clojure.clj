;;; plain Clojure의 한계 - sample eval은 evaluator coverage evidence가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/41-coverage-report/limit_clojure.clj

(ns coverage-report-limit)

(def samples
  ['(+ 1 2)
   '(if true 1 2)
   '(count [1 2 3])])

(def values
  (mapv eval samples))

(println "values:" values)
(println "op coverage report?:" false)
(println "branch coverage report?:" false)

(assert (= [3 1 3] values))

(println)
(println "결론: plain eval sample은 실행값만 주며 evaluator op/builtin/branch coverage를 남기지 않는다.")

