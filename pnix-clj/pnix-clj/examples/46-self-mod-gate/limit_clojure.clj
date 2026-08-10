;;; plain Clojure의 한계 - Var를 바꾸는 것은 가능하지만 owner-gated proposal trail이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/46-self-mod-gate/limit_clojure.clj

(ns self-mod-gate-limit)

(defn target []
  1)

(def before
  (target))

(alter-var-root #'target (constantly (fn [] 2)))

(def after
  (target))

(println "before/after:" before after)
(println "witnessed proposal?:" false)
(println "owner gate decision?:" false)

(assert (= 1 before))
(assert (= 2 after))

(println)
(println "결론: plain Var mutation은 가능하지만 witness와 owner-held decision을 강제하지 않는다.")

