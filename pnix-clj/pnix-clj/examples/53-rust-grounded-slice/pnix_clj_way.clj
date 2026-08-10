;;; pnix-clj의 방식 - Rust-grounded manifest와 작은 fixture slice를 같이 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/53-rust-grounded-slice/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.rust-batch :as rust-batch]))

(let [manifest (rust-batch/manifest)
      inventory (rust-batch/suite-source-inventory)
      cases (rust-batch/batch-cases)
      first-case (first cases)
      receipt (pnix/run-source first-case)]
  (println "manifest:" (select-keys manifest [:kind :schema-version :source-revision]))
  (println "inventory count:" (count inventory))
  (println "imported suite sources:" (count (filter #(= :imported-source (:status %)) inventory)))
  (println "first fixture:" (select-keys first-case [:source-id :fixture-path :oracle-result]))
  (println "first receipt:" (select-keys receipt [:status :reason]))

  (assert (= :rust-grounded-invariant-manifest (:kind manifest)))
  (assert (pos? (count inventory)))
  (assert (pos? (count cases)))
  (assert (= :accepted (:status receipt)))
  (assert (= :all-lanes-agree (:reason receipt)))
  (assert (= (get-in first-case [:oracle-result :value])
             (get-in receipt [:eval-result :value]))))

(println)
(println "결론: pnix-clj는 Rust-grounded provenance를 보존하면서 작은 fixture도 lane receipt로 검증한다.")
(shutdown-agents)

