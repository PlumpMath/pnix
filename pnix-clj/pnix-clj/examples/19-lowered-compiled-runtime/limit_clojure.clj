;;; plain Clojure의 한계: eval과 compile/evidence 경로가 기본적으로 분리되어 있지 않다.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/19-lowered-compiled-runtime/limit_clojure.clj

(ns limit-clojure)

(def source "(let [x 40] (+ x 2))")

(let [form (read-string source)
      value (eval form)]
  (println "source:" source)
  (println "eval value:" value)
  (assert (= 42 value)))

(println)
(println "결론: plain Clojure eval은 값은 주지만 direct evaluator vs lowered compiled runtime 동등성 receipt를 기본으로 주지 않는다.")
