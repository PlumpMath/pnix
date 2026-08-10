;;; plain Clojure의 한계 - eval은 host call을 실행한 뒤 값만 돌려준다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/38-form-analysis-host-surface/limit_clojure.clj

(ns form-analysis-host-surface-limit)

(def host-form
  '(System/getProperty "user.home"))

(def value
  (eval host-form))

(println "form:" (pr-str host-form))
(println "value class:" (.getName (class value)))
(println "pre-exec host surface classification?:" false)

(assert (string? value))

(println)
(println "결론: plain eval은 host interop 표면을 실행 전 AST 수준에서 분류하지 않는다.")

