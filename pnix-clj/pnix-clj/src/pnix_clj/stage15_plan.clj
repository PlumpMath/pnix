(ns pnix-clj.stage15-plan
  (:require [clojure.string :as str]
            [pnix-clj.stage15 :as stage15]))

(def lane-classification
  {:lane :proof-only
   :scope :stage15-control-plan-printer
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :allowed-output :stage15-control-plan-text-report})

(defn -main
  [& _]
  (let [{:keys [backend-root backend-role stage-range mirror-spine truth-boundary
                write-policy reason inputs commands missing-inputs]}
        (stage15/control-plan)]
    (println "pnix-clj stage15 control plan")
    (println "backend:" backend-root)
    (println "backend-role:" backend-role)
    (println "stage-range:" (pr-str stage-range))
    (println "mirror-spine:" (pr-str mirror-spine))
    (println "truth-boundary:" truth-boundary)
    (println "write-policy:" write-policy "reason:" reason)
    (println "inputs:")
    (doseq [{:keys [path exists hash]} inputs]
      (println (format "  [%s] %s %s"
                       (if exists "OK" "MISS")
                       path
                       (or hash "<missing>"))))
    (println "commands:")
    (doseq [{:keys [id command purpose]} commands]
      (println (format "  %s: cd ../clj-meta && %s  ; %s"
                       (name id)
                       (str/join " " command)
                       purpose)))
    (shutdown-agents)
    (when (seq missing-inputs)
      (System/exit 1))))
