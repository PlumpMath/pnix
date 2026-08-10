(ns pnix-clj.stage15-execute
  "Stage-15 execution lane: runs the staged plan and records its execution receipt."
  (:require [clojure.string :as str]
            [pnix-clj.stage15 :as stage15]))

(def lane-classification
  {:lane :proof-only
   :scope :manual-stage15-execution-entrypoint
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :allowed-output :stage15-execution-receipt})

(defn -main
  [& [ids timeout-ms]]
  (let [{:keys [status reason selected-command-count held-count rows first-held
                receipt-hash] :as report}
        (stage15/execute-plan
         {:command-ids (stage15/parse-command-ids ids)
          :timeout-ms (if (str/blank? (or timeout-ms ""))
                        stage15/default-timeout-ms
                        (parse-long timeout-ms))})]
    (println "pnix-clj stage15 execution")
    (println "status:" status "reason:" reason)
    (println "commands:" selected-command-count "held:" held-count)
    (println "receipt-hash:" receipt-hash)
    (doseq [{:keys [id status exit duration-ms stdout-hash stderr-hash]} rows]
      (println (format "  %s status=%s exit=%s duration-ms=%s stdout=%s stderr=%s"
                       (name id)
                       (name status)
                       (or exit "<none>")
                       duration-ms
                       (or stdout-hash "<none>")
                       (or stderr-hash "<none>"))))
    (when first-held
      (println "first held:" (pr-str first-held)))
    (shutdown-agents)
    (when (not= :ok (:status report))
      (System/exit 1))))
