;;; pnix-clj의 방식 - M7h machine-property가 property-fuzzer report에 들어간 것을 확인한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/92-machine-property-fuzzer-lane/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.property-fuzzer :as pf]))

(def fixed-sources
  ["1 + 2 * 3"
   "let v = 3; in v * 2"
   "if 1 < 2 then 10 else 20"
   "{ k = builtins.length [ 1 2 3 ]; }.k"])

(doseq [source fixed-sources]
  (let [same? (pf/machine-agrees? source)]
    (println "machine-agrees?" source "=>" same?)
    (assert (= true same?))))

(let [report (pf/report {:num-tests 5 :seed 20260708})]
  (println "property-fuzzer report:"
           (select-keys report [:schema :status :num-tests :seed :num-tests-run
                                :cross-lane-pass? :specializer-pass?
                                :cache-pass? :specializer-proven-arith-pass?
                                :machine-pass?]))

  (assert (= :pnix-clj.property-fuzzer-report.v4 (:schema report)))
  (assert (= :ok (:status report)))
  (assert (= true (:pass? report)))
  (assert (= true (:machine-pass? report)))
  (assert (= true (:cross-lane-pass? report)))
  (assert (= true (:specializer-pass? report)))
  (assert (= true (:cache-pass? report)))
  (assert (= true (:specializer-proven-arith-pass? report)))
  (assert (>= (:num-tests-run report) 25)))

(println)
(println "결론: M7h 이후 property-fuzzer는 random typed source에서도 machine⇄evaluator exact agreement를 gate한다.")
(shutdown-agents)
