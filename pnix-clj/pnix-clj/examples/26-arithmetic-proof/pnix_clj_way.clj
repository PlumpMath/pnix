;;; pnix-clj의 방식 - arith-proof가 산술 fragment를 canonical polynomial로 정규화해
;;; 모든 변수값에 대한 동치를 증명한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/26-arithmetic-proof/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.arith-proof :as arith]))

(let [commute? (arith/equivalent? "x + 2" "2 + x")
      folded (arith/prove-specialize-meaning "x + y" {"y" 7})
      non-fragment (arith/prove-specialize-meaning "if x then 1 else 2" {})]
  (println "x + 2 == 2 + x proven?:" commute?)
  (println "specialize proof:" folded)
  (println "non fragment:" non-fragment)

  (assert (= true commute?))
  (assert (= :proven (:status folded)))
  (assert (= true (arith/equivalent? (:residual-source folded) "x + 7")))
  (assert (= :unprovable (:status non-fragment))))

(println)
(println "결론: pnix-clj arith-proof는 증명 가능한 산술 fragment와 unprovable frontier를 명확히 나눈다.")
