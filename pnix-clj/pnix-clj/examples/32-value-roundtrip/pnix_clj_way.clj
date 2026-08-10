;;; pnix-clj의 방식 - value-roundtrip이 pnix value를 canonical Clojure form으로
;;; 합성하고 clj-meta로 다시 값이 되는지 확인한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/32-value-roundtrip/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.core :as pnix]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.value-roundtrip :as vr]))

(let [eval-result (pnix/eval-source "[1 2 3]")
      pnix-value (lowering/force-normal (:value eval-result))
      synthesized (vr/synthesize-form pnix-value)
      host-result (clj-meta/eval-lowered (:form synthesized))
      held (vr/synthesize-form {:kind :closure})]
  (println "pnix value:" pnix-value)
  (println "synthesized form:" (:form synthesized))
  (println "host value:" (:value host-result))
  (println "held function-like value:" held)

  (assert (= :ok (:status eval-result)))
  (assert (= :ok (:status synthesized)))
  (assert (= [1 2 3] (:form synthesized)))
  (assert (= :ok (:status host-result)))
  (assert (= pnix-value (:value host-result)))
  (assert (= :held (:status held)))
  (assert (= :function-value-not-synthesizable (:reason held))))

(println)
(println "결론: pnix-clj는 value-level projection이 닫히는 범위와 held frontier를 분리한다.")
