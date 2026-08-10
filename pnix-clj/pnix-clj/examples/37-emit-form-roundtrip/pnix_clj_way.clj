;;; pnix-clj의 방식 - tools.analyzer AST에서 emitted form으로 돌아와도 값이 같음을 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/37-emit-form-roundtrip/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.emit-form-roundtrip :as rt]))

(let [report (rt/report)
      first-row (first (:rows report))]
  (println "status:" (:status report))
  (println "cases/ok/held:" (:case-count report) (:ok report) (:held-or-rejected report))
  (println "first emitted form:" (:id first-row) "=>" (pr-str (:emitted-form first-row)))
  (println "receipt hash:" (:receipt-hash report))

  (assert (= :ok (:status report)))
  (assert (= (:case-count report) (:ok report)))
  (assert (zero? (:held-or-rejected report)))
  (assert (string? (:receipt-hash report))))

(println)
(println "결론: pnix-clj는 analyzer emit-form 경로를 값 동치와 hash receipt로 고정한다.")
(shutdown-agents)
