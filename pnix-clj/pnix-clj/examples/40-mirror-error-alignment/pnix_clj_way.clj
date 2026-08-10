;;; pnix-clj의 방식 - negative fixture도 lane별 error frontier가 맞는지 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/40-mirror-error-alignment/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.mirror-error :as mirror-error]))

(let [report (mirror-error/report)
      rows (:mirror-error-rows report)
      first-row (first rows)]
  (println "fixtures:" (:fixture-count report))
  (println "accepted/held/rejected:" (:accepted report) (:held report) (:rejected report))
  (println "first source:" (:source-id first-row))
  (println "first observed:" (:observed first-row))

  (assert (= (:fixture-count report) (:accepted report)))
  (assert (zero? (:held report)))
  (assert (zero? (:rejected report)))
  (assert (= (:fixture-count report) (count rows)))
  (assert (= :agree (:alignment first-row)))
  (assert (= "error" (get-in first-row [:observed :runtime-mirror-status]))))

(println)
(println "결론: pnix-clj는 실패해야 하는 source도 runtime mirror error까지 정렬해 증거로 남긴다.")
(shutdown-agents)
