;;; pnix-clj의 방식 - REPL renderer와 eval-print가 pnix value 출력 정책을 공유한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/66-repl-rendering/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.repl :as repl]))

(let [rendered (repl/render {"b" [1 2] "a" nil})
      printed (with-out-str (repl/eval-print "1 + 2"))]
  (println "rendered:" rendered)
  (println "eval-print output:" (pr-str printed))

  (assert (= "{ a = null; b = [ 1 2 ]; }" rendered))
  (assert (= "3\n" printed)))

(println)
(println "결론: pnix-clj REPL surface는 evaluator result를 pnix 값 문법에 가까운 출력으로 렌더링한다.")
(shutdown-agents)

