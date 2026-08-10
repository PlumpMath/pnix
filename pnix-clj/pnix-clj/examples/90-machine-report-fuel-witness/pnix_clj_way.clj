;;; pnix-clj의 방식 - M7g machine/report와 fuel budget witness를 확인한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/90-machine-report-fuel-witness/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.evaluator :as evaluator]
            [pnix-clj.machine :as machine]
            [pnix-clj.parser :as parser]))

(let [r (machine/report)]
  (println "machine report:"
           (select-keys r [:status :kind :schema :row-count]))
  (println "divergent count:" (count (:divergent r)))
  (println "constant-stack witness:"
           (select-keys (:constant-stack-witness r) [:depth :ok?]))

  (assert (= :ok (:status r)))
  (assert (= :machine-report (:kind r)))
  (assert (pos? (:row-count r)))
  (assert (empty? (:divergent r)))
  (assert (true? (get-in r [:constant-stack-witness :ok?]))))

(let [fuel-result
      (try
        (binding [evaluator/*fuel* (volatile! 3)]
          (machine/run-whnf
           (:ast (parser/parse-source
                  "let a=1; b=a; c=b; d=c; e=d; f=e; g=f; in g"))
           {})
          {:thrown? false})
        (catch clojure.lang.ExceptionInfo e
          {:thrown? true
           :message (.getMessage e)
           :data (ex-data e)}))]
  (println "fuel result:" fuel-result)
  (assert (= true (:thrown? fuel-result)))
  (assert (re-find #"fuel exhausted" (:message fuel-result)))
  (assert (= true (get-in fuel-result [:data :pnix-fuel-exhausted]))))

(println)
(println "결론: M7g machine report는 shared differential corpus, constant-stack witness, fuel budget bound를 하나의 regression artifact로 보여준다.")
(shutdown-agents)
