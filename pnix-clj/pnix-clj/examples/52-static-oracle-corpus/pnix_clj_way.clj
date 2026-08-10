;;; pnix-clj의 방식 - captured Nix oracle fixture를 repo resource로 읽어 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/52-static-oracle-corpus/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.oracle :as oracle]))

(let [fixture-set (oracle/ground-truth-fixture-set)
      all-cases (oracle/ground-truth-cases)
      sample-cases (vec (take 5 all-cases))
      report (pnix/report sample-cases)]
  (println "oracle fixture:" (select-keys fixture-set [:kind :schema-version :lineage]))
  (println "sample count:" (count sample-cases) "of" (count all-cases))
  (println "accepted/held/rejected:" (:accepted report) (:held report) (:rejected report))

  (assert (= :nix-ground-truth-oracle-set (:kind fixture-set)))
  (assert (pos? (count all-cases)))
  (assert (= (count sample-cases) (:accepted report)))
  (assert (zero? (:held report)))
  (assert (zero? (:rejected report))))

(println)
(println "결론: pnix-clj는 외부 Nix 호출 대신 captured oracle resource를 lane report에 태운다.")
(shutdown-agents)

