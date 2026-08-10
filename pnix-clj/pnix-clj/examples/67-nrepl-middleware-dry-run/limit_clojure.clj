;;; plain Clojure의 한계 - eval은 Clojure 코드로 해석하며 pnix nREPL lane routing이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/67-nrepl-middleware-dry-run/limit_clojure.clj

(ns nrepl-middleware-dry-run-limit)

(def clj-result
  (eval '(+ 1 2)))

(println "Clojure eval result:" clj-result)
(println "pnix eval op routed through pnix-clj.core?:" false)
(println "transport value/done messages?:" false)

(assert (= 3 clj-result))

(println)
(println "결론: plain eval은 editor/nREPL eval op를 pnix language lane으로 라우팅하지 않는다.")

