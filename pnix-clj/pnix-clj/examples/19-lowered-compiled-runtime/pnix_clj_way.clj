;;; pnix-clj의 방식: direct evaluator와 lowered clj-meta compiled path를 비교한다.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/19-lowered-compiled-runtime/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.parser :as parser]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.clj-meta :as clj-meta]))

(def source "let x = 40; in x + 2")

(let [direct (pnix/eval-source source)
      parsed (parser/parse-source source)
      lowered (lowering/lower-ast (:ast parsed))
      compiled (clj-meta/eval-lowered (:form lowered))
      determinism-status (get-in compiled [:compile-receipt :determinism :status])]
  (println "source:" source)
  (println "direct evaluator:" (:status direct) (:value direct))
  (println "parse:" (:status parsed))
  (println "lowering:" (:status lowered))
  (println "compiled host path:" (:status compiled) (:value compiled))
  (println "compile mode:" (:mode compiled))
  (println "compile receipt determinism:" determinism-status)
  (println "api values agree?:" (:api-values-agree? compiled))

  (assert (= :ok (:status direct)))
  (assert (= :ok (:status parsed)))
  (assert (= :ok (:status lowered)))
  (assert (= :ok (:status compiled)))
  (assert (= 42 (:value direct)))
  (assert (= (:value direct) (:value compiled)))
  (assert (= :ok determinism-status))
  (assert (= true (:api-values-agree? compiled))))

(println)
(println "결론: pnix-clj는 직접 evaluator 값과 lowered clj-meta compiled path 값을 receipt로 교차 확인한다.")
