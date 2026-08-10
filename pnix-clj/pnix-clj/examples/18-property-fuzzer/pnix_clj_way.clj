;;; pnix-clj의 방식 - property-fuzzer가 쓰는 public property들을 작은 소스로 직접 확인한다.
;;; 실제 report는 test.check가 generated source와 shrink counterexample을 다룬다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/18-property-fuzzer/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.property-fuzzer :as pf]))

(let [collapse? (pf/lanes-collapse? "1 + 2 * 3")
      cache? (pf/cache-preserves-meaning? "1 + 2")
      specialize? (pf/specialize-preserves-meaning? "x + y" ["x"])
      machine? (pf/machine-agrees? "let v = 1; in v + 2")
      report (pf/report {:num-tests 3 :seed 42})]
  (println "cross-lane property:" collapse?)
  (println "cache property:" cache?)
  (println "specializer property:" specialize?)
  (println "machine property:" machine?)
  (println "report:" (select-keys report [:schema :status :pass? :machine-pass?]))

  (assert (= true collapse?))
  (assert (= true cache?))
  (assert (= true specialize?))
  (assert (= true machine?))
  (assert (= :pnix-clj.property-fuzzer-report.v4 (:schema report)))
  (assert (= true (:machine-pass? report)))
  (assert (= :ok (:status report))))

(println)
(println "결론: pnix-clj의 fuzzer property는 lane collapse, cache soundness, specializer meaning preservation, machine agreement를 같은 틀에서 검사한다.")
