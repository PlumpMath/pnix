;;; plain Clojure의 한계 — host function call은 값은 만들지만,
;;; typed interop attestation schema/witness/capability envelope를 기본으로 남기지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/25-typed-attestation/limit_clojure.clj

(ns typed-attestation-limit)

(defn host-add-two
  [x]
  (+ x 2))

(let [input 40
      value (host-add-two input)]
  (println "host call input:" input)
  (println "host call value:" value)
  (println "capability verdict:" nil)
  (println "typed attestation:" nil)
  (println "witness schema:" nil)
  (println "witness hash:" nil)

  (assert (= 42 value)))

(println)
(println "결론: plain Clojure host call은 값은 주지만 typed capability/witness attestation을 기본으로 남기지 않는다.")
