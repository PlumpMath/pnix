;;; pnix-clj의 방식 - durable store report가 content-address와 event chain을 재검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/49-persistent-evidence-store/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.persist :as persist]))

(let [report (persist/report)]
  (println "status:" (:status report))
  (println "accepted/rejected:" (:accepted report) (:rejected report))
  (println "rows:" (:rows report))
  (println "report hash:" (:report-hash report))

  (assert (= :ok (:status report)))
  (assert (= (:total report) (:accepted report)))
  (assert (zero? (:rejected report)))
  (assert (string? (:report-hash report))))

(println)
(println "결론: pnix-clj persist는 term과 event를 저장한 뒤 reload integrity를 report로 확인한다.")
(shutdown-agents)
