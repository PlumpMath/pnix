;;; pnix-clj의 방식 - forward reference fixture를 lane별 contract로 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/39-forward-reference-lift/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.forward-reference :as fr]))

(let [report (fr/report)]
  (println "fixtures:" (:fixture-count report))
  (println "accepted/held/rejected:" (:accepted report) (:held report) (:rejected report))
  (println "forward-ok / semantic-error:" (:forward-ok-count report) (:semantic-error-count report))
  (println "receipt hash:" (:receipt-hash report))

  (assert (= (:fixture-count report) (:accepted report)))
  (assert (zero? (:held report)))
  (assert (zero? (:rejected report)))
  (assert (pos? (:forward-ok-count report)))
  (assert (pos? (:semantic-error-count report)))
  (assert (string? (:receipt-hash report))))

(println)
(println "결론: pnix-clj는 forward reference 성공과 semantic error를 lane별 증거로 분리해 고정한다.")
(shutdown-agents)
