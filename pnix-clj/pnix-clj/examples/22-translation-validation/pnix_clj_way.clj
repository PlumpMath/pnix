;;; pnix-clj의 방식 - translation-validation report가 각 candidate를
;;; default-held validator row로 감싼다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/22-translation-validation/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.translation-validation :as tv]))

(let [report (tv/report)
      rows (:sample-rows report)]
  (println "status:" (:status report))
  (println "validator count:" (:validator-count report))
  (println "sample source:" (:sample-source report))
  (doseq [row rows]
    (println " -" (:validator row) "status=" (:status row)
             "evidence=" (:evidence row)))

  (assert (= :ok (:status report)))
  (assert (pos? (:validator-count report)))
  (assert (= :accepted (:sample-receipt-status report)))
  (assert (every? #(= :ok (:status %)) rows))
  (assert (string? (:receipt-hash report))))

(println)
(println "결론: pnix-clj는 compile/runtime 후보를 validator catalog와 evidence row로 감싼다.")
