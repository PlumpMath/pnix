;;; pnix-clj의 방식 - compile-source가 parse/lower/clj-meta receipt를 묶는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/70-core-compile-pipeline/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(let [receipt (pnix/compile-source "1 + 2")]
  (println "compile receipt:" (select-keys receipt [:kind :status :reason :source-hash
                                                    :ast-hash :lowered-form-hash]))
  (println "clj-meta:" (select-keys (:clj-meta-result receipt)
                                    [:status :value :mode :api-values-agree?]))

  (assert (= :pnix-clj.compile-source (:kind receipt)))
  (assert (= :ok (:status receipt)))
  (assert (= 3 (get-in receipt [:clj-meta-result :value])))
  (assert (= true (get-in receipt [:clj-meta-result :api-values-agree?])))
  (assert (string? (:source-hash receipt)))
  (assert (string? (:lowered-form-hash receipt))))

(println)
(println "결론: pnix-clj compile-source는 값 이전에 compile evidence를 먼저 다룰 수 있게 한다.")
(shutdown-agents)

