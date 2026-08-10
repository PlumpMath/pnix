;;; plain Clojure의 한계 - source 하나만 보면 Rust-grounded provenance가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/53-rust-grounded-slice/limit_clojure.clj

(ns rust-grounded-slice-limit)

(def copied-source
  "{ sum = 1 + 2; }")

(def value
  {:sum 3})

(println "copied source:" copied-source)
(println "plain value:" value)
(println "rust suite inventory/hash?:" false)

(assert (= 3 (:sum value)))

(println)
(println "결론: plain sample은 Rust corpus provenance, imported test count, fixture hash를 보존하지 않는다.")

