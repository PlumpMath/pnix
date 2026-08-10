;;; plain Clojure의 한계 - 작은 재귀 interpreter는 derived abstract machine lane이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/61-abstract-machine-lane/limit_clojure.clj

(ns abstract-machine-lane-limit)

(defn eval-expr
  [expr env]
  (case (:op expr)
    :int (:value expr)
    :var (get env (:name expr))
    :add (+ (eval-expr (:left expr) env)
            (eval-expr (:right expr) env))))

(def expr
  {:op :add
   :left {:op :var :name "x"}
   :right {:op :int :value 2}})

(println "plain recursive interpreter value:" (eval-expr expr {"x" 40}))
(println "derived from pnix evaluator?:" false)
(println "unsupported-op gate?:" false)

(assert (= 42 (eval-expr expr {"x" 40})))

(println)
(println "결론: hand-written recursive interpreter는 pnix evaluator와 공유된 abstract-machine lane 증거가 아니다.")

