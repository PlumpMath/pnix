;;; plain Clojure의 한계 — eval/compile은 가능하지만,
;;; Futamura 1차/2차/3차 projection ladder receipt를 기본으로 주지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/33-futamura-ladder/limit_clojure.clj

(ns futamura-ladder-limit)

(defn interp-like
  [x]
  (+ x 2))

(let [value (interp-like 40)]
  (println "plain value:" value)
  (println "1st projection residual hash:" nil)
  (println "2nd projection compiler-id:" nil)
  (println "3rd projection status:" nil)
  (println "Jones-optimality witness:" nil)

  (assert (= 42 value)))

(println)
(println "결론: plain Clojure는 값은 만들 수 있지만 Futamura ladder의 projection evidence를 기본으로 주지 않는다.")
