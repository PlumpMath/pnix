;;; pnix-clj의 방식 - seed가 있는 generated source를 run-source gate로 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/42-grammar-fuzzer/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.grammar-fuzzer :as fuzz]))

(let [opts {:positive-count 4 :error-count 2 :seed 7}
      report (fuzz/report opts)]
  (println "seed:" (:seed report))
  (println "sources/ok/failed:" (:source-count report) (:ok report) (:failed report))
  (println "actual status counts:" (:actual-status-counts report))
  (println "first row:" (select-keys (first (:rows report))
                                     [:source-id :fixture-class :expected-status :actual-status]))

  (assert (= 7 (:seed report)))
  (assert (= 6 (:source-count report)))
  (assert (= 6 (:ok report)))
  (assert (zero? (:failed report)))
  (assert (= 4 (get (:actual-status-counts report) :accepted)))
  (assert (= 2 (get (:actual-status-counts report) :held))))

(println)
(println "결론: pnix-clj grammar fuzzer는 generated source까지 expected verdict와 실제 lane 결과를 맞춘다.")
(shutdown-agents)
