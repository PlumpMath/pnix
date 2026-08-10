(ns pnix-clj.runtime-plan
  (:require [pnix-clj.px-runtime :as px-runtime]))

(def lane-classification
  {:lane :proof-only
   :scope :px-runtime-plan-printer
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :allowed-output :runtime-plan-text-report})

(defn -main
  [& _]
  (let [{:keys [entry artifact-count edge-count reason missing-imports edges boundary
                entry-parse bootstrap
                resource-root container-path]}
        (px-runtime/runtime-run-plan)]
    (println "pnix-clj runtime plan")
    (println "resource-root:" resource-root)
    (println "container-path:" container-path)
    (println "boundary:" (pr-str (select-keys boundary
                                              [:status :reason :allowed-roots
                                               :external-runtime-roots-forbidden
                                               :parent-checkouts-runtime-dependency])))
    (println "entry-parse:" (pr-str (select-keys entry-parse
                                                  [:status :reason :ast-op])))
    (println "bootstrap:" (pr-str (select-keys bootstrap
                                                [:status :reason
                                                 :evaluated-artifact-count
                                                 :value-summary])))
    (println "entry:" (pr-str entry))
    (println "artifacts:" artifact-count "edges:" edge-count "reason:" reason)
    (when (seq missing-imports)
      (println "missing imports:" (pr-str missing-imports)))
    (doseq [{:keys [from import to status]} edges]
      (println (format "  [%s] %s imports %s -> %s"
                       (name status) from import (or to "<missing>"))))
    (shutdown-agents)
    (when (seq missing-imports)
      (System/exit 1))))
