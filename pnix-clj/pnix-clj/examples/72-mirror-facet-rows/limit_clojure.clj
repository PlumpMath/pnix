;;; plain Clojure의 한계 - 값 하나는 mirror facet row가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/72-mirror-facet-rows/limit_clojure.clj

(ns mirror-facet-rows-limit)

(def value
  (eval '(+ 1 2)))

(println "plain value:" value)
(println "clojure mirror row?:" false)
(println "px runtime row?:" false)
(println "cross mirror verdict?:" false)

(assert (= 3 value))

(println)
(println "결론: plain eval은 host/px/pnix mirror facet을 분리해서 보여주지 않는다.")

