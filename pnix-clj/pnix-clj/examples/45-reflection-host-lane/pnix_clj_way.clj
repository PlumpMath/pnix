;;; pnix-clj의 방식 - host-varying reflection 입력을 stable snapshot으로 pin한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/45-reflection-host-lane/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.reflect :as reflect]))

(let [snap1 (reflect/reflection-snapshot)
      snap2 (reflect/reflection-snapshot)
      when-var (reflect/var-snapshot 'clojure.core/when)
      report (reflect/report)]
  (println "snapshot:" snap1)
  (println "when var snapshot:" when-var)
  (println "report accepted/rejected:" (:accepted report) (:rejected report))

  (assert (= snap1 snap2))
  (assert (string? (:host-lane-id snap1)))
  (assert (:macro when-var))
  (assert (= :ok (:status report)))
  (assert (= (:total report) (:accepted report))))

(println)
(println "결론: pnix-clj reflect는 JVM host lane을 pure EDN snapshot과 stable hash로 고정한다.")
(shutdown-agents)
