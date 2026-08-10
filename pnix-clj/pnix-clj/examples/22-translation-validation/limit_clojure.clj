;;; plain Clojure의 한계 - compile/run 성공은 per-phase translation validation이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/22-translation-validation/limit_clojure.clj

(ns translation-validation-limit)

(let [form '(+ 40 2)
      value (eval form)]
  (println "form:" form)
  (println "value:" value)
  (println "missing: parse/lowering/compile/px/mirror validator catalog")
  (assert (= 42 value)))

(println)
(println "결론: plain Clojure는 source-form -> bytecode/runtime candidate별 validation row를 기본 제공하지 않는다.")
