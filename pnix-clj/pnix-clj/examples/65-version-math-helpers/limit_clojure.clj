;;; plain Clojure의 한계 - lexicographic version compare와 / 는 Nix helper semantics가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/65-version-math-helpers/limit_clojure.clj

(ns version-math-helpers-limit)

(println "lexicographic 10 < 2?:" (neg? (compare "10" "2")))
(println "plain / 5 2:" (/ 5 2))
(println "drv split by last dash?:" false)

(assert (neg? (compare "10" "2")))
(assert (= 5/2 (/ 5 2)))

(println)
(println "결론: plain string/math helper는 Nix-compatible version/order/division 규칙을 표현하지 않는다.")

