;;; pnix-clj의 방식 - Clojure fixture projection을 report로 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/36-clojure-projection-report/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.clojure-projection :as projection]))

(let [report (projection/report)]
  (println "fixtures:" (:fixture-count report))
  (println "accepted/held/rejected:" (:accepted report) (:held report) (:rejected report))
  (println "runtime:" (:runtime-status report) (:runtime-reason report))
  (println "host crossing count:" (:host-crossing-count report))

  (assert (= (:total report) (:accepted report)))
  (assert (zero? (:held report)))
  (assert (zero? (:rejected report)))
  (assert (= :ok (:runtime-status report)))
  (assert (pos? (:host-crossing-count report)))
  (assert (= (:fixture-count report)
             (count (:clojure-projection-rows report)))))

(println)
(println "결론: pnix-clj projection report는 host crossing과 projection runtime 검증을 fixture별 증거로 남긴다.")
(shutdown-agents)
