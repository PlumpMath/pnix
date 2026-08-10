;;; plain Clojure의 한계 - read-string 반복은 parser/lowering cache stats가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/74-parser-lowering-cache-stats/limit_clojure.clj

(ns parser-lowering-cache-stats-limit)

(def a
  (read-string "(+ 1 2)"))

(def b
  (read-string "(+ 1 2)"))

(println "same data?:" (= a b))
(println "cache hits/misses?:" false)
(println "lowering cache stats?:" false)

(assert (= a b))

(println)
(println "결론: plain read-string 반복은 parse/lower cache hit evidence를 남기지 않는다.")

