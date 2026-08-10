;;; pnix-clj의 방식 - lowered form을 clj-meta proof lane에서 평가한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/71-clj-meta-lowered-eval/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.core :as pnix]))

(let [lowered (pnix/lower-source "1 + 2")
      result (clj-meta/eval-lowered (:form lowered))]
  (println "lowered:" (select-keys lowered [:status :form-hash]))
  (println "clj-meta:" (select-keys result [:status :value :mode :api-values-agree?]))
  (println "receipt:" (select-keys (:compile-receipt result) [:schema :determinism]))

  (assert (= :ok (:status lowered)))
  (assert (= :ok (:status result)))
  (assert (= 3 (:value result)))
  (assert (= true (:api-values-agree? result)))
  (assert (= :pnix-clj.clj-meta.compile-receipt.v0
             (get-in result [:compile-receipt :schema]))))

(println)
(println "결론: pnix-clj는 lowered form 실행을 clj-meta compile receipt와 함께 다룬다.")
(shutdown-agents)

