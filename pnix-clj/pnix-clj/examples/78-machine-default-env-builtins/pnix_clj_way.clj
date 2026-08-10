;;; pnix-clj의 방식 - M7c machine이 evaluator default env/builtins 아래에서 수렴한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/78-machine-default-env-builtins/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.machine :as machine]))

(defn comparable
  [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))

(def sources
  ["builtins.length [ 1 2 3 ]"
   "builtins.add 1 2"
   "map (x: x + 1) [ 2 ]"
   "toString 1.5"
   "builtins.length (builtins.map (throw \"BOOM\") [ ])"
   "builtins.any (throw \"BOOM\") [ ]"
   "builtins.readFile \"/x\""])

(doseq [source sources]
  (let [evaluator (pnix/eval-source source)
        machine (machine/eval-source source)]
    (println "source:" source)
    (println " evaluator:" (comparable evaluator))
    (println " machine:  " (comparable machine))
    (assert (= (comparable evaluator)
               (comparable machine)))))

(println)
(println "결론: M7c machine은 builtins/default env를 별도 복제하지 않고 evaluator public apply boundary와 같은 결과/held reason으로 수렴한다.")
(shutdown-agents)
