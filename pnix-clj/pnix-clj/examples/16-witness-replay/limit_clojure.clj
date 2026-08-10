;;; plain Clojure의 한계 - 저장된 "증거"를 나중에 fresh re-run으로 검증하는 표준 경로가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/16-witness-replay/limit_clojure.clj

(ns witness-replay-limit)

(def saved {:source "(+ 40 2)" :value 42})
(def fresh (eval (read-string (:source saved))))

(println "saved:" saved)
(println "fresh value:" fresh)
(println "missing: term-hash/result-hash/snapshot-id 비교와 reproduced/diverged verdict")

(assert (= (:value saved) fresh))

(println)
(println "결론: plain 재실행 비교는 witness schema와 divergence field를 갖춘 replay verdict가 아니다.")
