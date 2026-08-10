;;; plain Clojure의 한계 - hand-written interpreter는 매번 tag dispatch를 한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/48-weval-partial-eval/limit_clojure.clj

(ns weval-partial-eval-limit)

(defn interp
  [expr]
  (case (:op expr)
    :lit (:value expr)
    :add (+ (interp (:left expr)) (interp (:right expr)))))

(def program
  {:op :add
   :left {:op :lit :value 40}
   :right {:op :lit :value 2}})

(println "interpreted value:" (interp program))
(println "residual dispatch-free function?:" false)

(assert (= 42 (interp program)))

(println)
(println "결론: plain interpreter는 값은 만들지만 residual dispatch-free report를 만들지 않는다.")

