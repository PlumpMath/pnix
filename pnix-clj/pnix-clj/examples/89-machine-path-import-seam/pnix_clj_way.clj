;;; pnix-clj의 방식 - M7f machine이 path literal과 import resolver seam을 실행한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/89-machine-path-import-seam/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.machine :as machine]
            [pnix-clj.parser :as parser]))

(defn comparable
  [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))

(def path-cases
  ["./m"
   "./a == ./a"
   "./a == ./b"
   "toString ./x"
   "[ ./a ./b ]"])

(doseq [source path-cases]
  (let [evaluator (pnix/eval-source source)
        machine (machine/eval-source source)]
    (println "path source:" source)
    (println " evaluator:" (comparable evaluator))
    (println " machine:  " (comparable machine))
    (assert (= (comparable evaluator)
               (comparable machine)))))

(let [unwired (machine/eval-source "import ./m")]
  (println "unwired import:" (select-keys unwired [:status :reason]))
  (assert (= :held (:status unwired)))
  (assert (= :import-evaluation-not-wired (:reason unwired))))

(let [ast (:ast (parser/parse-source "import ./m"))
      resolver (fn [ctx target scope]
                 (assert (= {:example :machine-import} ctx))
                 (assert (= "./m" target))
                 (assert (nil? scope))
                 {:status :ok :value 42})
      imported (binding [evaluator/*import-context* {:example :machine-import}
                         evaluator/*import-resolver* resolver]
                 (machine/run-ast ast))]
  (println "resolved import:" (select-keys imported [:status :value :reason]))
  (assert (= :ok (:status imported)))
  (assert (= 42 (:value imported))))

(println)
(println "결론: M7f machine은 path literal을 evaluator와 맞추고, import는 shared resolver seam이 있을 때만 실행한다.")
(shutdown-agents)
