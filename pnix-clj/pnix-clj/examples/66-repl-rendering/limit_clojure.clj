;;; plain Clojure의 한계 - pr-str는 pnix REPL value renderer가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/66-repl-rendering/limit_clojure.clj

(ns repl-rendering-limit)

(def value
  {"b" [1 2]
   "a" nil})

(println "pr-str:" (pr-str value))
(println "pnix attrset/list renderer?:" false)

(assert (string? (pr-str value)))

(println)
(println "결론: plain pr-str는 pnix REPL의 attrset/list/opaque value rendering policy가 아니다.")

