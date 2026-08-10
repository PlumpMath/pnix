;;; plain Clojure의 한계 - expected 값을 손으로 두면 provenance와 lane 검증이 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/52-static-oracle-corpus/limit_clojure.clj

(ns static-oracle-corpus-limit)

(def hand-written-expectations
  {"1 + 1" 2
   "true" true})

(println "hand-written expectations:" hand-written-expectations)
(println "captured Nix provenance?:" false)
(println "multi-lane oracle report?:" false)

(assert (= 2 (get hand-written-expectations "1 + 1")))

(println)
(println "결론: 손으로 둔 expected map은 실제 Nix 캡처 provenance와 cross-lane 검증 report가 아니다.")

