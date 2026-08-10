;;; pnix-clj의 방식 - stage7 core lockin fixture를 pnix report로 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/50-stage7-core-lockins/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.stage7-core :as stage7]))

(let [report (stage7/report)]
  (println "fixtures:" (:fixture-count report))
  (println "accepted/held/rejected:" (:accepted report) (:held report) (:rejected report))
  (println "lineage:" (:lineage report))
  (println "first hash:" (first (:fixture-hashes report)))

  (assert (= (:fixture-count report) (:accepted report)))
  (assert (zero? (:held report)))
  (assert (zero? (:rejected report)))
  (assert (= (:fixture-count report)
             (count (:fixture-hashes report))))
  (assert (nil? (:first-frontier report))))

(println)
(println "결론: pnix-clj stage7 report는 self-hosting core lockin fixture를 lane receipt로 고정한다.")
(shutdown-agents)
