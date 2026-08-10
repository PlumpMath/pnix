;;; plain Clojure의 한계 - macroexpand/eval은 analyzer emit-form 왕복 증거가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/37-emit-form-roundtrip/limit_clojure.clj

(ns emit-form-roundtrip-limit)

(def form
  '(let [x 20] (+ x 22)))

(def expanded
  (macroexpand form))

(println "form value:" (eval form))
(println "macroexpanded equal to original?:" (= form expanded))
(println "has analyzer emitted form hash?:" false)

(assert (= 42 (eval form)))

(println)
(println "결론: plain Clojure는 실행값은 확인하지만 analyzer AST emit-form roundtrip receipt를 남기지 않는다.")

