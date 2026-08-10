;;; plain Clojure의 한계 - 몇 개 숫자 대입 테스트는 모든 입력에 대한 산술 동치 proof가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/26-arithmetic-proof/limit_clojure.clj

(ns arithmetic-proof-limit)

(defn a [x] (+ x 2))
(defn b [x] (+ 2 x))
(def samples [-1 0 1 10])

(println "sample values:" (mapv (fn [x] [(a x) (b x)]) samples))
(println "sample equal?:" (every? true? (map #(= (a %) (b %)) samples)))
(println "missing: canonical polynomial proof over all x")

(assert (every? true? (map #(= (a %) (b %)) samples)))

(println)
(println "결론: finite samples는 모든 integer assignment에 대한 산술 동치 증명이 아니다.")
