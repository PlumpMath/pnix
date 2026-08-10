;;; plain Clojure의 한계 - hand interpreter는 default env/builtins 의미를 직접 심어야 한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/78-machine-default-env-builtins/limit_clojure.clj

(ns machine-default-env-builtins-limit)

(defn eval-expr
  [expr env]
  (case (:op expr)
    :literal (:value expr)
    :var (if (contains? env (:name expr))
           (get env (:name expr))
           (throw (ex-info "unbound var" {:name (:name expr)})))
    :call (let [f (eval-expr (:fn expr) env)
                arg (eval-expr (:arg expr) env)]
            (f arg))))

(def program
  {:op :call
   :fn {:op :var :name "builtins.length"}
   :arg {:op :literal :value [1 2 3]}})

(def result
  (try
    (eval-expr program {})
    (catch Throwable t
      {:status :threw :data (ex-data t)})))

(println "hand interpreter result:" result)
(println "default builtins env included?:" false)
(println "evaluator-derived machine agreement?:" false)

(assert (= :threw (:status result)))
(assert (= "builtins.length" (get-in result [:data :name])))

(println)
(println "결론: plain hand interpreter는 default env와 builtins delegation을 직접 복제해야 하며 evaluator-derived lane 증거가 없다.")
