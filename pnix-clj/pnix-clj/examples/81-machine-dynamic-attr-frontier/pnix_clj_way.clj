;;; pnix-clj의 방식 - M7e 이후 machine도 dynamic attr key를 native로 실행한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/81-machine-dynamic-attr-frontier/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.machine :as machine]))

(defn comparable
  [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))

(def cases
  ["let k = \"x\"; in { \"${k}\" = 5; }.x"
   "{ ${\"x\"} = 1; } ? ${\"x\"}"
   "{ a = 1; }.${\"a\"}"
   "{ }.${\"q\"} or \"dflt\""
   "{ a = 1; \"${\"a\"}\" = 2; }.a"
   "{ }.${1} or \"d\""])

(doseq [source cases]
  (let [evaluator (pnix/eval-source source)
        machine (machine/eval-source source)]
    (println "source:" source)
    (println " evaluator:" (comparable evaluator))
    (println " machine:  " (comparable machine))
    (assert (= (comparable evaluator)
               (comparable machine)))))

(println)
(println "결론: M7e 이후 machine dynamic-attr frontier는 닫혔다. 값도 held reason도 evaluator와 같아야 한다.")
(shutdown-agents)
