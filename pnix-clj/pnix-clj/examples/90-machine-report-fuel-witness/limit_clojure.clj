;;; plain Clojure의 한계 - hand report는 shared corpus/fuel/constant-stack witness를 직접 보장하지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/90-machine-report-fuel-witness/limit_clojure.clj

(ns machine-report-fuel-witness-limit)

(def sample-corpus
  ["1 + 2" "let x = 1; in x"])

(defn hand-report
  [rows]
  {:status :ok
   :row-count (count rows)
   :divergent []
   :constant-stack-witness nil
   :fuel-budget-shared? false})

(let [r (hand-report sample-corpus)]
  (println "hand report:" r)
  (assert (= :ok (:status r)))
  (assert (= 2 (:row-count r)))
  (assert (nil? (:constant-stack-witness r)))
  (assert (= false (:fuel-budget-shared? r))))

(println)
(println "결론: plain report map은 만들 수 있지만, machine/evaluator shared corpus, constant-stack witness, fuel bound를 자동으로 증명하지 않는다.")
