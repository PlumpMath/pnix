;;; pnix-clj의 방식 - snapshot이 evaluator/host runtime version을 pin하고,
;;; mismatch에서는 resolve-under-snapshot이 fail closed 한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/15-runtime-snapshot/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.snapshot :as snapshot]))

(let [snap (snapshot/make-snapshot)
      ok (snapshot/resolve-under-snapshot "1 + 2" snap)
      stale (assoc snap :evaluator-version "not-current")
      refused (snapshot/resolve-under-snapshot "1 + 2" stale)]
  (println "snapshot id:" (:snapshot/id snap))
  (println "runtime matches?:" (snapshot/runtime-matches? snap))
  (println "ok resolve:" (select-keys ok [:status :value :snapshot/id]))
  (println "stale resolve:" (select-keys refused [:status :reason :expected :actual]))

  (assert (string? (:snapshot/id snap)))
  (assert (= true (snapshot/runtime-matches? snap)))
  (assert (= :ok (:status ok)))
  (assert (= 3 (:value ok)))
  (assert (= (:snapshot/id snap) (:snapshot/id ok)))
  (assert (= :held (:status refused)))
  (assert (= :snapshot-evaluator-version-mismatch (:reason refused))))

(println)
(println "결론: pnix-clj snapshot은 content-addressed result를 runtime pin 없이 재사용하지 못하게 막는다.")
