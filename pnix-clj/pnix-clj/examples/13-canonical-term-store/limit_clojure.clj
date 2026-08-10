;;; plain Clojure의 한계 - pr-str/hash는 의미상 같은 term의 canonical identity가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/13-canonical-term-store/limit_clojure.clj

(ns canonical-term-store-limit)

(def a '(fn [x] x))
(def b '(fn [y] y))

(println "a:" (pr-str a))
(println "b:" (pr-str b))
(println "string equal?:" (= (pr-str a) (pr-str b)))
(println "plain hash equal?:" (= (hash (pr-str a)) (hash (pr-str b))))

(assert (not= (pr-str a) (pr-str b)))

(println)
(println "결론: plain string/hash identity는 alpha-equivalence나 term canonicalization을 보장하지 않는다.")
