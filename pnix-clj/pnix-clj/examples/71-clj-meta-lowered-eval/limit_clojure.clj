;;; plain Clojure의 한계 - eval은 clj-meta compile/eval receipt가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/71-clj-meta-lowered-eval/limit_clojure.clj

(ns clj-meta-lowered-eval-limit)

(def form
  '(+ 1 2))

(println "plain eval:" (eval form))
(println "compile receipt?:" false)
(println "api values agree?:" false)

(assert (= 3 (eval form)))

(println)
(println "결론: plain eval은 clj-meta compiler/evaluator의 증거 API를 통과하지 않는다.")

