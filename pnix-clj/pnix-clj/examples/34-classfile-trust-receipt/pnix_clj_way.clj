;;; pnix-clj의 방식 - classfile-receipt가 clj-meta emitted class artifact를
;;; hash/verification으로 요약하고, trust report가 common-mode risk boundary를 명시한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/34-classfile-trust-receipt/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.classfile-receipt :as classfile]
            [pnix-clj.trust :as trust]))

(let [class-report (classfile/report)
      trust-report (trust/report)]
  (println "classfile status:" (:status class-report)
           "rows=" (:row-count class-report))
  (println "generated class kinds:" (:generated-class-kinds class-report))
  (println "pnix compile row:"
           (select-keys (:pnix-compile-row class-report)
                        [:source-id :status :receipt-schema]))
  (println "trust status:" (:status trust-report))
  (println "shared TCB:" (:shared-tcb trust-report))
  (println "residual risk:" (:residual-risk trust-report))

  (assert (= :ok (:status class-report)))
  (assert (pos? (:row-count class-report)))
  (assert (string? (:receipt-hash class-report)))
  (assert (some #{:deftype} (:generated-class-kinds class-report)))
  (assert (= :ok (:status trust-report)))
  (assert (string? (:report-hash trust-report)))
  (assert (seq (:shared-tcb trust-report)))
  (assert (seq (:residual-risk trust-report))))

(println)
(println "결론: pnix-clj는 JVM classfile evidence와 cross-lane trust boundary를 값과 별도로 receipt화한다.")
