;;; pnix-clj의 방식 - mirror-pair가 204개 committed 코퍼스 소스 전체에서
;;; 4-레인(direct evaluator / clj-meta bytecode / .px runtime / pnix mirror)
;;; 수렴을 하나의 report로 집계한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/94-mirror-pair-corpus-report/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.mirror-pair :as mp]))

(let [report (mp/report)]
  (println "kind:" (:kind report) "fixture-count:" (:fixture-count report))
  (println "ready/not-ready:" (:mirror-pair-ready-count report)
           (:mirror-pair-not-ready-count report))

  (assert (= :mirror-pair-report (:kind report)))
  (assert (= 204 (:fixture-count report)))
  (assert (= (:fixture-count report) (:mirror-pair-ready-count report)))
  (assert (zero? (:mirror-pair-not-ready-count report)))
  (assert (every? :ready? (:mirror-pair-rows report))))

(println)
(println "결론: pnix-clj mirror-pair는 코퍼스 전체(204개)의 4-레인 수렴을 한 report로 집계한다.")
(shutdown-agents)
