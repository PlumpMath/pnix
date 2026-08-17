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
   ["(fn [x] (if (>= x 41) 42 0))" [41] 42]
   ["(fn [x] (if (> x 0) (if (> x 100) 2 1) 0))" [50] 1]
   ["(fn [x] (if (> x 0) (if (> x 100) 2 1) 0))" [200] 2]
   ["(fn [x] (if (> x 0) (if (> x 100) 2 1) 0))" [-5] 0]
   ["(fn [a b c] (+ (+ a b) c))" [10 20 12] 42]
   ["(fn [a b c] (if (< a b) (+ b c) (- b c)))" [1 20 22] 42]
   ["(fn [a b c d] (+ (+ a b) (+ c d)))" [10 10 10 12] 42]
   ["(fn [a b c d] (if (> a b) (+ c d) (- d c)))" [5 1 20 22] 42]
   ["(fn [x] (let [y (+ x 1)] (* y 2)))" [10] 22]
   ["(fn [x y] (let [a (+ x y) b (* a 2)] (- b 1)))" [3 4] 13]
   ["(fn [x] (let [y (if (> x 0) x (- 0 x))] (* y y)))" [-5] 25]
   ["(fn [x] (let [x (+ x 1)] (* x 2)))" [10] 22]
   ["(fn [x] (let [a (+ x 1)] (let [b (* a 2)] (+ a b))))" [5] 18]
   ["(fn [n] (loop [i 0 acc 0] (if (> i n) acc (recur (+ i 1) (+ acc i)))))" [10] 55]
   ["(fn [n] (loop [i n acc 1] (if (<= i 1) acc (recur (- i 1) (* acc i)))))" [5] 120]
   ["(fn [n acc] (if (= n 0) acc (recur (- n 1) (+ acc n))))" [5 0] 15]
   ["(fn [n a b] (loop [i n a a b b] (if (= i 0) a (recur (- i 1) b a))))" [3 1 2] 2]
   ["(fn [] ((fn [x] (+ x 1)) 41))" [] 42]
   ["(fn [] (((fn [x] (fn [y] (+ x y))) 20) 22))" [] 42]
   ["(fn [] ((((fn [a] (fn [b] (fn [c] (+ a (+ b c))))) 1) 2) 3))" [] 6]
   ["(fn [n] (if (> n 0) ((fn [x] (* x 2)) n) 0))" [5] 10]
   ["(fn [n] (let [square (fn [x] (* x x))] (square n)))" [7] 49]
   ["(fn [n] (let [k 10 add-k (fn [x] (+ x k))] (add-k n)))" [5] 15]])

(deftest independent-mini-backend-agrees-with-host-eval
  (testing "real host ClojureCLR eval and the independent mini backend agree"
    (doseq [[source args expected] fixtures]
      (let [host-fn (eval (read-string source))
            host-result (long (apply host-fn args))
            mini-result (long (mini/compile-and-invoke source args))]
        (is (= expected host-result) (str "host mismatch: " source))
        (is (= expected mini-result) (str "mini backend mismatch: " source))))))

(def ^:private overflow-fixtures
  "checked-i64-expression profile's :overflow :system-overflow-exception --
  both the real host and the independent mini backend use checked (not
  wrapping) Int64 arithmetic, so both must reject these the same way."
  [["(fn [x] (+ x 1))" [Int64/MaxValue]]
   ["(fn [x] (- x 1))" [Int64/MinValue]]
   ["(fn [x] (* x 2))" [Int64/MaxValue]]
   ["(fn [x y] (+ x y))" [Int64/MaxValue Int64/MaxValue]]])

(deftest independent-mini-backend-agrees-on-checked-overflow
  (testing "real host ClojureCLR eval and the independent mini backend both reject Int64 overflow"
    (doseq [[source args] overflow-fixtures]
      (let [host-fn (eval (read-string source))
            host-threw? (try (apply host-fn args) false (catch Exception _ true))
            mini-threw? (try (mini/compile-and-invoke source args) false (catch Exception _ true))]
        (is host-threw? (str "host did not reject overflow: " source))
        (is mini-threw? (str "mini backend did not reject overflow: " source))))))
