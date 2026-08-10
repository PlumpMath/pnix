;;; plain Clojure의 한계 - 손 목록은 구현과 자동으로 맞지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/58-capability-index/limit_clojure.clj

(ns capability-index-limit)

(def handwritten
  {:reports ["safe-eval" "tower"]
   :builtins ["map" "length"]})

(println "handwritten index:" handwritten)
(println "derived from code?:" false)
(println "public API scanned?:" false)

(assert (some #{"tower"} (:reports handwritten)))

(println)
(println "결론: 손으로 쓴 capability 목록은 report registry, builtins, public API와 자동 동기화되지 않는다.")

