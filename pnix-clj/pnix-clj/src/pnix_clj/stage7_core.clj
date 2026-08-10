(ns pnix-clj.stage7-core
  "Stage-7 core self-* fixtures: the meta-circular self-hosting core sources pinned for regression."
  (:require [clojure.java.io :as io]
            [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :stage7-core-lockin-regression
   :product-runtime :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :allowed-input :stage7-lockin-fixtures
   :allowed-output :stage-closure-report})

(def cases-resource
  "pnix_clj/stage7_core/cases.edn")

(defn lockin-set
  []
  (if-let [resource (io/resource cases-resource)]
    (read-string (slurp resource))
    (throw (ex-info "stage7 core lockins missing"
                    {:resource cases-resource}))))

(defn cases
  []
  (let [{:keys [lineage cases]} (lockin-set)]
    (mapv (fn [{:keys [name source oracle-result] :as case}]
            {:source-id (keyword "stage7-core" name)
             :source source
             :batch :stage7-core/lockins
             :source-origin (:source-file lineage)
             :source-lineage lineage
             :fixture-hash (hash/sha256 (pr-str case))
             :oracle-result oracle-result})
          cases)))

(defn report
  []
  (let [lockins (lockin-set)
        cases (cases)
        pnix-report (pnix/report cases)]
    (assoc pnix-report
           :kind :stage7-core-lockin-report
           :lockin-kind (:kind lockins)
           :lockin-schema-version (:schema-version lockins)
           :lineage (:lineage lockins)
           :fixture-count (count cases)
           :fixture-hashes (mapv #(select-keys % [:source-id :fixture-hash])
                                 cases))))

(defn -main
  [& _]
  (let [{:keys [fixture-count accepted rejected held reason-counts first-frontier]}
        (report)]
    (println (format "pnix-clj stage7 core: fixtures=%d accepted=%d rejected=%d held=%d"
                     fixture-count accepted rejected held))
    (println "reason-counts:" (pr-str reason-counts))
    (when first-frontier
      (println "first frontier:" (pr-str first-frontier)))
    (shutdown-agents)
    (when (pos? rejected)
      (System/exit 1))))
