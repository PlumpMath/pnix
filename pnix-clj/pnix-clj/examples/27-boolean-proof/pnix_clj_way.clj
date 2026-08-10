;;; pnix-clj의 방식 - bool-proof가 finite boolean domain 전체를 exhaustive truth table로 검사한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/27-boolean-proof/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.bool-proof :as bp]))

(let [dm (bp/prove-equivalent "(!(a && b))" "((!a) || (!b))")
      not-same (bp/prove-equivalent "a && b" "a || b")]
  (println "de Morgan:" dm)
  (println "and vs or:" not-same)

  (assert (= :proven (:status dm)))
  (assert (= ["a" "b"] (:vars dm)))
  (assert (= 4 (:assignments-checked dm)))
  (assert (= :refuted (:status not-same)))
  (assert (map? (:assignment not-same))))

(println)
(println "결론: pnix-clj bool-proof는 boolean fragment에서 proven/refuted를 전체 truth table로 판정한다.")
