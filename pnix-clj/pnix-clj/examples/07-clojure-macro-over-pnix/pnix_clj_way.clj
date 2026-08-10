;;; pnix-clj의 방식 — Clojure macroexpand 결과를 whitelisted form으로 정규화한 뒤,
;;; synthesize/form->pnix와 tower/run-tower로 meaning preservation을 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/07-clojure-macro-over-pnix/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [clojure.walk :as walk]
            [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.synthesize :as synth]
            [pnix-clj.tower :as tower]))

(defmacro add-two
  [x]
  `(+ ~x 2))

(defmacro choose
  [condition then-expr else-expr]
  `(if ~condition ~then-expr ~else-expr))

(defn unqualify-core-symbols
  [form]
  (walk/postwalk
   (fn [x]
     (if (and (symbol? x)
              (= "clojure.core" (namespace x)))
       (symbol (name x))
       x))
   form))

(defn macro-projection-row
  [id form]
  (let [expanded (macroexpand form)
        normalized (unqualify-core-symbols expanded)
        projected (synth/form->pnix normalized)
        expected (clj-meta/eval-lowered normalized)
        tower-result (when (= :ok (:status projected))
                       (tower/run-tower (:source projected)))
        collapse (:collapse tower-result)
        witness (:witness collapse)
        verified? (and (= :ok (:status projected))
                       (= :ok (:status expected))
                       (= :collapsed (:status collapse))
                       (= (:value expected) (:value collapse)))]
    {:id id
     :form form
     :expanded expanded
     :normalized normalized
     :projected-status (:status projected)
     :pnix-source (:source projected)
     :expected-status (:status expected)
     :expected-value (:value expected)
     :tower-collapse (:status collapse)
     :tower-value (:value collapse)
     :tower-witness witness
     :verified? verified?}))

(let [row-a (macro-projection-row :add-two '(add-two 40))
      row-b (macro-projection-row :choose '(choose true (+ 20 22) 0))
      rows [row-a row-b]
      all-verified? (every? :verified? rows)]

  (doseq [{:keys [id form expanded normalized pnix-source expected-value tower-collapse tower-value tower-witness verified?]} rows]
    (println "case:" id)
    (println " macro form:" form)
    (println " expanded:" expanded)
    (println " normalized:" normalized)
    (println " pnix source:" pnix-source)
    (println " clj-meta value:" expected-value)
    (println " tower collapse:" tower-collapse "value=" tower-value)
    (println " tower witness:" tower-witness)
    (println " verified?:" verified?)
    (println))

  (assert (= true (:verified? row-a)))
  (assert (= true (:verified? row-b)))
  (assert (= 42 (:expected-value row-a)))
  (assert (= 42 (:tower-value row-a)))
  (assert (= 42 (:expected-value row-b)))
  (assert (= 42 (:tower-value row-b)))
  (assert (= :collapsed (:tower-collapse row-a)))
  (assert (= :collapsed (:tower-collapse row-b)))
  (assert (= :ok (:projected-status row-a)))
  (assert (= :ok (:projected-status row-b)))
  (assert (string? (:source-hash (:tower-witness row-a))))
  (assert (string? (:ast-hash (:tower-witness row-a))))
  (assert (string? (:source-hash (:tower-witness row-b))))
  (assert (string? (:ast-hash (:tower-witness row-b))))
  (assert (= true all-verified?)))

(println "결론: pnix-clj는 Clojure macroexpand 결과를 pnix source로 투영하고 tower witness로 검증한다.")
