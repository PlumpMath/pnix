;;; pnix-clj의 방식 - stage15 backend 명령은 실행하지 않고 held control plan으로 전시한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/55-stage15-control-plan/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.stage15 :as stage15]))

(let [plan (stage15/control-plan)]
  (println "plan:" (select-keys plan [:kind :status :reason :backend :stage-range :write-policy]))
  (println "inputs:" (mapv #(select-keys % [:path :exists :hash]) (:inputs plan)))
  (println "commands:" (mapv #(select-keys % [:id :purpose]) (:commands plan)))

  (assert (= :stage15-control-plan (:kind plan)))
  (assert (= :held (:status plan)))
  (assert (= :stage15-gates-not-executed (:reason plan)))
  (assert (= :read-only-backend (:write-policy plan)))
  (assert (pos? (:command-count plan)))
  (assert (every? :exists (:inputs plan))))

(println)
(println "결론: pnix-clj stage15 plan은 외부 backend 실행을 자동화하지 않고 owner-visible plan으로 고정한다.")
(shutdown-agents)

