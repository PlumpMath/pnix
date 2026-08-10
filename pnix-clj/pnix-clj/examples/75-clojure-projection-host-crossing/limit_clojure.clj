;;; plain Clojure의 한계 - read/eval은 projection host crossing witness가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/75-clojure-projection-host-crossing/limit_clojure.clj

(ns clojure-projection-host-crossing-limit)

(def form
  (read-string "(+ 1 2)"))

(def value
  (eval form))

(println "form:" form)
(println "value:" value)
(println "interop metadata?:" false)
(println "input/output witness hash?:" false)

(assert (= 3 value))

(println)
(println "결론: plain read/eval은 host crossing을 만들지만 projection interop witness를 남기지 않는다.")

