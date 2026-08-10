;;; plain Clojure의 한계 - pr-str/read-string roundtrip은 pnix value projection receipt가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/32-value-roundtrip/limit_clojure.clj

(ns value-roundtrip-limit)

(def value [1 2 3])
(def roundtripped (read-string (pr-str value)))

(println "value:" value)
(println "roundtripped:" roundtripped)
(println "missing: pnix eval value, lowered form, synthesized Clojure form, closure hash")

(assert (= value roundtripped))

(println)
(println "결론: plain data roundtrip은 pnix value -> Clojure form -> value projection evidence가 아니다.")
