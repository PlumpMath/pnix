(ns pnix.clr-meta.independent-mini-backend-test
  "Trusting-Trust (Diverse Double-Compiling) witness: cross-check real host
  ClojureCLR `eval` against `independent-mini-backend`, a from-scratch
  reader+analyzer+DynamicMethod IL emitter that shares no code with the
  Compiler Stage1-7 family. See STATUS.md's 'Trusting-Trust defense roadmap'
  for the honest scope: a bounded fixture subset, behavior equivalence (not
  bit-identical IL), not the checked-Int64 expression profile Stage1-7
  formally closes."
  (:require [clojure.test :refer [deftest is testing]]
            [pnix.clr-meta.independent-mini-backend :as mini]))

(def ^:private fixtures
  [["(fn [x] (+ x 1))" [41] 42]
   ["(fn [x y] (if (< x y) (* (+ x 1) y) (- x y)))" [5 7] 42]
   ["(fn [] (* 6 7))" [] 42]
   ["(fn [x] (- x 1))" [43] 42]
   ["(fn [x y] (+ x y))" [20 22] 42]
   ["(fn [x] (if (> x 0) x (- 0 x)))" [-42] 42]
   ["(fn [x] (if (= x 42) 1 0))" [42] 1]
   ["(fn [x] (if (>= x 41) 42 0))" [41] 42]])

(deftest independent-mini-backend-agrees-with-host-eval
  (testing "real host ClojureCLR eval and the independent mini backend agree"
    (doseq [[source args expected] fixtures]
      (let [host-fn (eval (read-string source))
            host-result (long (apply host-fn args))
            mini-result (long (mini/compile-and-invoke source args))]
        (is (= expected host-result) (str "host mismatch: " source))
        (is (= expected mini-result) (str "mini backend mismatch: " source))))))
