;;; pnix-clj의 방식 - live-oracle이 실제 nix-instantiate와 pnix-clj를
;;; 같은 소스에 돌려 값을 직접 비교한다. nix-instantiate가 없으면 fail이
;;; 아니라 구조화된 :skipped로 빠진다(external-authority: comparison-only).
;;;
;;; 실행: cd pnix-clj && clojure -M examples/93-live-oracle-differential/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.live-oracle :as oracle]))

(let [report (oracle/report {:positive-count 5 :seed 0})]
  (println "status:" (:status report) "source-count:" (:source-count report))
  (println "matched/mismatched:" (:matched report) (:mismatched report))
  (println "pnix-held/oracle-held:" (:pnix-held report) (:oracle-held report))

  (if (= :skipped (:status report))
    (println "nix-instantiate를 못 찾음 - 이 환경에선 구조화된 skip으로 빠짐(실패 아님).")
    (do
      (assert (= :pnix-live-oracle-report (:kind report)))
      (assert (= :ok (:status report)))
      (assert (zero? (:mismatched report)))
      (assert (pos? (:matched report))))))

(println)
(println "결론: pnix-clj live-oracle은 실제 Nix와의 값 일치를 소스별 구조화된 행으로 남긴다.")
(shutdown-agents)
