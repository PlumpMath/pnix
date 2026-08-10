;;; pnix-clj의 방식 - M7d machine이 pattern lambda를 native control로 실행한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/79-machine-pattern-lambda-native/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.machine :as machine]))

(defn comparable
  [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))

(def sources
  ["({ a ? throw \"x\" }: 1) { }"
   "({ a ? b, b ? 2 }: a) { }"
   "({ a ? b, b ? a }: a) { }"
   "({ a }: a) { a = 1; b = 2; }"
   "({ a, ... }: a) { a = 1; b = 2; }"
   "({ a }: a) 1"
   "({ a ? 5 }@args: args.a or \"absent\") { }"
   "let f = { x, y ? x * 2 }: x + y; in f { x = 3; }"
   "builtins.map ({ v }: v + 1) [ { v = 1; } { v = 2; } ]"
   "builtins.functionArgs ({ a, b ? 1, ... }: a)"])

(doseq [source sources]
  (let [evaluator (pnix/eval-source source)
        machine (machine/eval-source source)]
    (println "source:" source)
    (println " evaluator:" (comparable evaluator))
    (println " machine:  " (comparable machine))
    (assert (= (comparable evaluator)
               (comparable machine)))))

(println)
(println "결론: M7d machine은 pattern-bind/default/body를 machine control 안에서 실행하며 evaluator와 같은 value 또는 held reason으로 수렴한다.")
(shutdown-agents)
