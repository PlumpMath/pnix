;;; pnix-clj의 방식 - clojure-form fixture corpus가 host eval, clj-meta,
;;; projection validation을 같은 fixture hash 아래에서 비교한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/09-clojure-form-fixture/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.clojure-form :as cf]))

(let [report (cf/report)
      first-row (first (:clojure-form-rows report))]
  (println "fixture count:" (:fixture-count report))
  (println "accepted/rejected/held:"
           (:accepted report) (:rejected report) (:held report))
  (println "first fixture:"
           (:source-id first-row)
           "status=" (:status first-row)
           "expected=" (:expected-value first-row)
           "host=" (get-in first-row [:host-result :value])
           "clj-meta=" (get-in first-row [:clj-meta-result :value]))
  (println "projection term hash:" (:projection-term-hash first-row))

  (assert (pos? (:fixture-count report)))
  (assert (zero? (:rejected report)))
  (assert (zero? (:held report)))
  (assert (= (:fixture-count report) (:accepted report)))
  (assert (= :accepted (:status first-row)))
  (assert (string? (:fixture-hash first-row)))
  (assert (string? (:projection-term-hash first-row))))

(println)
(println "결론: pnix-clj는 Clojure form을 fixture hash, clj-meta bytecode, projection validation으로 고정한다.")
