;;; plain Clojure의 한계 - 일부 boolean sample은 논리식 전체 동치 proof가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/27-boolean-proof/limit_clojure.clj

(ns boolean-proof-limit)

(defn lhs [a b] (not (and a b)))
(defn rhs [a b] (or (not a) (not b)))
(def samples [[true true] [true false]])

(println "sample values:" (mapv (fn [[a b]] [(lhs a b) (rhs a b)]) samples))
(println "missing: exhaustive truth table over all assignments")

(assert (every? true? (map (fn [[a b]] (= (lhs a b) (rhs a b))) samples)))

(println)
(println "결론: 손으로 고른 boolean sample은 전체 truth table proof가 아니다.")
