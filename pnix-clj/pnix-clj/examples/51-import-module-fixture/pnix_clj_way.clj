;;; pnix-clj의 방식 - in-memory import fixture를 모든 lane에 전달해 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/51-import-module-fixture/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.import-module :as import-module]))

(let [case (first (import-module/cases))
      receipt (pnix/run-source case)]
  (println "case:" (select-keys case [:source-id :source :import-modules :oracle-result]))
  (println "receipt:" (select-keys receipt [:status :reason :source-id]))
  (println "eval result:" (:eval-result receipt))
  (println "lane summary:" (:lane-summary receipt))

  (assert (= :import-module/in-memory-value (:source-id case)))
  (assert (= :accepted (:status receipt)))
  (assert (= :all-lanes-agree (:reason receipt)))
  (assert (= 3 (get-in receipt [:eval-result :value])))
  (assert (= 3 (get-in receipt [:clj-meta-result :value])))
  (assert (= 3 (get-in receipt [:px-runtime :value]))))

(println)
(println "결론: pnix-clj import fixture는 파일 IO 없이 module map을 receipt 전체에 고정한다.")
(shutdown-agents)

