;;; plain Clojure의 한계 - 손 checklist는 capability wiring integrity가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/73-wiki-integrity-index/limit_clojure.clj

(ns wiki-integrity-index-limit)

(def checklist
  [{:id :feature-a :status :landed :capability :missing-report}])

(println "manual checklist:" checklist)
(println "report registry checked?:" false)
(println "generated wiki integrity?:" false)

(assert (= :landed (:status (first checklist))))

(println)
(println "결론: plain checklist는 landed capability가 실제 report registry에 연결됐는지 검증하지 않는다.")

