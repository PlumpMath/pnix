(ns pnix.clr-meta.bootstrap-test
  (:require [clojure.test :refer [deftest is run-tests testing]]
            [pnix.clr-meta.bootstrap :as bootstrap]
            [pnix.clr-meta.compiler-stage1-test]
            [pnix.clr-meta.main-test]
            [pnix.clr-meta.runtime-artifact-test]))

(defn symbols-in
  [form]
  (filter symbol? (tree-seq coll? seq form)))

(deftest interpreted-evaluator-has-no-host-eval-escape
  (testing "the interpreted evaluator cannot call the host compiler"
    (is (not-any? #{'eval} (symbols-in bootstrap/evaluator-source)))
    (is (not (contains? bootstrap/base-env 'eval)))))

(deftest evaluator-reproduces-itself-across-two-stages
  (let [stages (bootstrap/build-stage-chain 2)
        quoted-source (list 'quote bootstrap/evaluator-source)]
    (is (= 3 (count stages)))
    (is (every? ifn? stages))
    (is (every? true?
                (map (fn [[left right]]
                       (not (identical? left right)))
                     (partition 2 1 stages))))
    (is (every? #(= bootstrap/evaluator-source
                    (% quoted-source bootstrap/base-env))
                stages))))

(deftest all-stages-agree-with-the-focused-corpus
  (let [stages (bootstrap/build-stage-chain 2)
        rows (map #(bootstrap/evaluate-case stages %)
                  bootstrap/conformance-cases)]
    (is (= 9 (count rows)))
    (is (every? :ok rows))))

(deftest receipt-is-ready-and-honest-about-its-boundary
  (let [receipt (bootstrap/run-gate)
        not-claimed (set (:not-claimed receipt))]
    (is (:ready receipt))
    (is (= :evaluator-self-interpretation
           (get-in receipt [:claim :kind])))
    (is (= 1 (get-in receipt [:claim :seed-eval-count])))
    (is (= 2 (get-in receipt [:claim :self-interpreted-stages])))
    (is (= 3 (get-in receipt [:claim :physical-generations])))
    (is (= :evaluator-generation
           (get-in receipt [:naming :physical-sequence])))
    (is (= :absent
           (get-in receipt [:naming :compiler-stage-sequence])))
    (is (false? (get-in receipt [:naming :compiler-stage15-n])))
    (is (false? (get-in receipt [:boundary :target-can-call-host-eval])))
    (is (contains? not-claimed :clojureclr-compiler-self-reproduction))
    (is (contains? not-claimed :clr-il-fixed-point))
    (is (contains? not-claimed :compiler-stage15-n))
    (is (contains? not-claimed :full-clojureclr-tool-replacement))
    (is (contains? not-claimed :full-clojure-language-surface))
    (is (contains? not-claimed :pnix-language-semantics))))

(defn -main
  [& _]
  (let [summary (run-tests 'pnix.clr-meta.bootstrap-test
                           'pnix.clr-meta.compiler-stage1-test
                           'pnix.clr-meta.main-test
                           'pnix.clr-meta.runtime-artifact-test)
        bootstrap-receipt (bootstrap/run-gate)
        tests-ready (and (zero? (:fail summary))
                         (zero? (:error summary)))
        receipt {:schema :pnix.clr-meta.bootstrap-test-receipt.v1
                 :bootstrap bootstrap-receipt
                 :tests (select-keys summary [:test :pass :fail :error])
                 :ready (and tests-ready (:ready bootstrap-receipt))}]
    (prn receipt)
    (flush)
    (when-not (:ready receipt)
      (System.Environment/Exit 1))))
