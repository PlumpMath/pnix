;;; pnix-clj의 방식 - generate가 finite examples에서 후보를 만들고,
;;; CEGIS가 counterexample로 spec을 강화한 뒤 arith-proof로 proven까지 올린다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/28-generate-and-cegis/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.cegis :as cegis]
            [pnix-clj.generate :as gen]))

(def inc-spec
  {:vars ["x"]
   :examples [{:in {"x" 1} :out 2}
              {:in {"x" 2} :out 3}
              {:in {"x" 3} :out 4}]
   :max-size 3})

(let [syn (gen/synthesize inc-spec)
      match (first (:matches syn))
      c (cegis/cegis-synthesize {:vars ["x"]
                                  :reference "x + 2"
                                  :seed-probe 0})]
  (println "synthesis matches:" (:matches syn))
  (println "classes/enumerated/pruned:"
           (:classes syn) (:enumerated syn) (:pruned-proven syn))
  (println "first match vector:" (gen/value-vector match (:examples inc-spec)))
  (println "cegis:" c)

  (assert (some #(= [2 3 4] (gen/value-vector % (:examples inc-spec)))
                (:matches syn)))
  (assert (< (:classes syn) (:enumerated syn)))
  (assert (= :converged (:status c)))
  (assert (= :proven (:proof-status c)))
  (assert (> (:iterations c) 1))
  (assert (nil? (cegis/counterexample (:candidate c) "x + 2" "x"
                                      cegis/default-probes))))

(println)
(println "결론: pnix-clj는 finite examples에서 시작하지만 CEGIS와 arith-proof로 counterexample/proven 경계를 명시한다.")
