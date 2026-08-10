;;; plain Clojure의 한계 - 후보를 손으로 고르면 observational dedup, counterexample refinement,
;;; proof upgrade가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/28-generate-and-cegis/limit_clojure.clj

(ns generate-and-cegis-limit)

(def candidates ["x + 1" "x + 2" "x * x"])
(def examples [{:in 1 :out 2} {:in 2 :out 3} {:in 3 :out 4}])

(defn eval-candidate [expr x]
  (case expr
    "x + 1" (+ x 1)
    "x + 2" (+ x 2)
    "x * x" (* x x)))

(println "manual matches:"
         (filter (fn [expr]
                   (every? (fn [{:keys [in out]}]
                             (= out (eval-candidate expr in)))
                           examples))
                 candidates))
(println "missing: generator search space, value-vector dedup, CEGIS counterexamples, arith proof upgrade")

(assert (= ["x + 1"]
           (vec (filter (fn [expr]
                          (every? (fn [{:keys [in out]}]
                                    (= out (eval-candidate expr in)))
                                  examples))
                        candidates))))

(println)
(println "결론: 손으로 후보를 나열하는 것은 synthesis/refinement/proof pipeline이 아니다.")
