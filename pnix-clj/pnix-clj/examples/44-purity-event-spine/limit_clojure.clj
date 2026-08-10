;;; plain Clojure의 한계 - 반복 eval은 snapshot/event spine이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/44-purity-event-spine/limit_clojure.clj

(ns purity-event-spine-limit)

(def values
  (repeatedly 3 #(eval '(+ 1 2))))

(def mutable-log
  (atom [{:kind :run :value (first values)}]))

(swap! mutable-log conj {:kind :tamper :value 999})

(println "values:" values)
(println "mutable log:" @mutable-log)
(println "hash-chain verified?:" false)

(assert (apply = values))

(println)
(println "결론: plain 반복 eval과 atom 로그는 snapshot에 pin된 append-only purity event가 아니다.")

