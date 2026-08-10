;;; pnix-clj의 방식 - determinism/report가 fixture corpus를 여러 번 parse/eval하고
;;; AST/result hash가 흔들리지 않는지 audit한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/21-determinism-audit/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.determinism :as det]))

(let [report (det/report {:runs 2 :include-runtime? false})]
  (println "source count:" (:source-count report))
  (println "stable/unstable:" (:stable report) (:unstable report))
  (println "families:" (:source-family-counts report))

  (assert (pos? (:source-count report)))
  (assert (= 2 (:runs-per-source report)))
  (assert (zero? (:unstable report)))
  (assert (= (:source-count report) (:stable report))))

(println)
(println "결론: pnix-clj determinism audit는 corpus 전체에 대해 repeatable parse/eval hash를 확인한다.")
