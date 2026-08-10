;;; plain Clojure의 한계 - 실행 결과를 저장해도 term/snapshot/tower/mirror/purity가 묶인 witness가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/29-witnessed-durable-run/limit_clojure.clj

(ns witnessed-durable-run-limit)

(def saved (atom nil))
(reset! saved {:source "(+ 40 2)" :value (eval (read-string "(+ 40 2)"))})

(println "saved:" @saved)
(println "missing: admitted witness, event hashes, residual key, durable term/event persistence")

(assert (= 42 (:value @saved)))

(println)
(println "결론: plain 저장은 evidence spine에 의해 admission된 witnessed run이 아니다.")
