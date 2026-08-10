;;; plain Clojure의 한계: 문자열 eval은 값은 주지만 AST/lowering/receipt chain은 없다.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/06-ast-lowering-roundtrip/limit_clojure.clj

(ns limit-clojure)

(let [source "(+ 1 (* 2 3))"
      form (read-string source)
      value (eval form)]
  (println "source:" source)
  (println "read form:" form)
  (println "eval value:" value)
  (assert (= 7 value)))

(println)
(println "결론: plain Clojure는 read/eval은 쉽지만 pnix AST hash, lowering hash, host compile/eval receipt를 기본으로 남기지 않는다.")
