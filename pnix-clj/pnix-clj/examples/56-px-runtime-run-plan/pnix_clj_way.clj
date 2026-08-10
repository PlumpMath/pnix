;;; pnix-clj의 방식 - vendored px runtime resource graph를 plan으로 점검한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/56-px-runtime-run-plan/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.px-runtime :as px-runtime]))

(let [plan (px-runtime/runtime-run-plan)]
  (println "plan:" (select-keys plan [:status :reason :artifact-count :edge-count :missing-imports]))
  (println "boundary:" (select-keys (:boundary plan) [:status :reason]))
  (println "entry parse:" (select-keys (:entry-parse plan) [:status :reason :ast-op]))
  (println "resource root:" (:resource-root plan))

  (assert (pos? (:artifact-count plan)))
  (assert (pos? (:edge-count plan)))
  (assert (empty? (:missing-imports plan)))
  (assert (= :ok (get-in plan [:boundary :status])))
  (assert (= :ok (get-in plan [:entry-parse :status]))))

(println)
(println "결론: pnix-clj px runtime plan은 resource boundary와 import graph를 실행 전 receipt로 보여준다.")
(shutdown-agents)

