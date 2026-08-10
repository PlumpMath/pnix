;;; plain Clojure의 한계 - time 출력은 semantic preflight가 붙은 benchmark report가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/69-tiny-benchmark-report/limit_clojure.clj

(ns tiny-benchmark-report-limit)

(def elapsed
  (with-out-str
    (time (+ 1 2))))

(println "time output:" (pr-str elapsed))
(println "semantic preflight?:" false)
(println "parse/lower/full-report lanes?:" false)

(assert (string? elapsed))

(println)
(println "결론: plain time은 사람이 보는 출력일 뿐 preflight와 lane stats가 있는 benchmark report가 아니다.")

