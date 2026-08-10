;;; pnix-clj의 방식 - 작은 source 하나로 benchmark report shape를 확인한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/69-tiny-benchmark-report/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.benchmark :as benchmark]))

(let [report (benchmark/run-benchmark {:sources ["1 + 2"]
                                       :iterations 1
                                       :run-iterations 1})
      lane-ids (mapv :id (:lanes report))]
  (println "benchmark:" (select-keys report [:kind :status :reason :source-count
                                             :iterations :run-iterations :preflight]))
  (println "lanes:" lane-ids)
  (println "parse cache:" (:parse-cache report))
  (println "lower cache:" (:lower-cache report))

  (assert (= :pnix-clj-benchmark (:kind report)))
  (assert (= :ok (:status report)))
  (assert (= 1 (:source-count report)))
  (assert (= {:total 1
              :accepted 1
              :rejected 0
              :held 0
              :first-frontier nil
              :first-rejected nil}
             (:preflight report)))
  (assert (= [:parse-source-cold
              :parse-source-warm
              :lower-ast-cold
              :lower-ast-warm
              :full-report]
             lane-ids)))

(println)
(println "결론: pnix-clj benchmark는 timing보다 먼저 semantic preflight와 measurement lane 구조를 report로 남긴다.")
(shutdown-agents)

