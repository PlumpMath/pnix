(ns pnix-clj.strict-audit
  "Strict-Nix-typing audit (audit-only, never changes results): classifies which sources WOULD fail under strict Nix typing; also the strict-gate over strict-ok sources."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]
            [pnix-clj.import-module :as import-module]
            [pnix-clj.mirror-pair :as mirror-pair]
            [pnix-clj.mirror-error :as mirror-error]
            [pnix-clj.oracle :as oracle]
            [pnix-clj.rust-batch :as rust-batch]
            [pnix-clj.stage7-core :as stage7-core]))

(def lane-classification
  {:lane :core
   :scope :strictness-audit-evidence
   :role :classify-strict-nix-typing-frontiers
   :product-runtime :allowed
   :semantic-authority :audit-only
   :behavior-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :determinism :required-upstream
   :allowed-output :strict-audit-or-strict-gate-report})

(def forward-reference-resource
  "pnix_clj/forward_reference/cases.edn")

(def runtime-resource-root
  "pnix_clj/pnix_runtime")

(defn- load-edn-resource
  [resource-path]
  (if-let [resource (io/resource resource-path)]
    (edn/read-string (slurp resource))
    (throw (ex-info "strict audit resource missing"
                    {:resource resource-path}))))

(defn- source-row
  [family source-id source extra]
  (merge {:source-id source-id
          :source (str source)
          :source-family family
          :source-hash (hash/sha256 (str source))}
         extra))

(defn- forward-reference-cases
  []
  (let [{:keys [lineage cases]} (load-edn-resource forward-reference-resource)]
    (mapv (fn [{:keys [name source class] :as case}]
            (source-row :forward-reference
                        (keyword "forward-reference" name)
                        source
                        {:fixture-class class
                         :fixture-hash (hash/sha256 (pr-str case))
                         :source-lineage lineage}))
          cases)))

(defn fixture-source-rows
  []
  (vec
   (concat
    (map (fn [{:keys [source-id source oracle-result] :as case}]
           (source-row :ground-truth-oracle
                       source-id
                       source
                       {:oracle-result oracle-result
                        :fixture-hash (hash/sha256 (pr-str case))}))
         (oracle/ground-truth-cases))
    (map #(source-row :mirror-pair
                      (:source-id %)
                      (:source %)
                      (select-keys % [:batch :oracle-result :fixture-hash
                                      :import-modules]))
         (mirror-pair/cases))
    (map #(source-row :mirror-error
                      (:source-id %)
                      (:source %)
                      (select-keys % [:batch :fixture-hash]))
         (mirror-error/cases))
    (map #(source-row :stage7-core
                      (:source-id %)
                      (:source %)
                      (select-keys % [:batch :oracle-result :fixture-hash]))
         (stage7-core/cases))
    (map #(source-row :import-module
                      (:source-id %)
                      (:source %)
                      (assoc (select-keys % [:import-modules :oracle-result])
                             :fixture-hash (hash/sha256 (pr-str %))))
         (import-module/cases))
    (forward-reference-cases)
    (map #(source-row :rust-grounded
                      (:source-id %)
                      (:source %)
                      (select-keys % [:batch :fixture-path :oracle-result
                                      :fixture-hash :source-revision]))
         (rust-batch/batch-cases)))))

(defn- runtime-root-file
  []
  (if-let [resource (io/resource runtime-resource-root)]
    (io/file (.toURI resource))
    (throw (ex-info "strict audit runtime resource root missing"
                    {:resource runtime-resource-root}))))

(defn- px-file?
  [^java.io.File f]
  (and (.isFile f)
       (str/ends-with? (.getName f) ".px")))

(defn runtime-source-rows
  []
  (let [root (runtime-root-file)
        root-path (.toPath root)]
    (->> (file-seq root)
         (filter px-file?)
         (sort-by #(.getPath %))
         (mapv (fn [^java.io.File f]
                 (let [relative (str (.relativize root-path (.toPath f)))
                       source (slurp f)
                       id-text (-> relative
                                   (str/replace #"\.px$" "")
                                   (str/replace #"[\\/]" "__"))]
                   (source-row :px-runtime
                               (keyword "px-runtime" id-text)
                               source
                               {:runtime-relative-path relative
                                :bytes (.length f)})))))))

(defn source-rows
  ([] (source-rows {}))
  ([{:keys [include-runtime?]
     :or {include-runtime? true}}]
   (cond-> (fixture-source-rows)
     include-runtime? (into (runtime-source-rows)))))

(defn- audit-row
  [{:keys [source-id source import-modules] :as row}]
  (let [{:keys [result strict-violations]}
        (pnix/eval-source-strict-audit source import-modules)
        strict-class (cond
                       (seq strict-violations) :strict-violation
                       (= :ok (:status result)) :strict-ok
                       :else :failed)]
    (-> row
        (dissoc :source)
        (assoc :strict-class strict-class
               :eval-status (:status result)
               :eval-reason (:reason result)
               :strict-violations (vec strict-violations)
               :strict-violation-count (count strict-violations)
               :source-preview (subs source 0 (min 120 (count source))))
        (cond-> (= :ok (:status result))
          (assoc :value-observed? true)
          (not= :ok (:status result))
          (assoc :failed-error (select-keys result [:reason :error]))))))

(defn- count-by
  [f xs]
  (frequencies (keep f xs)))

(defn report
  ([] (report {}))
  ([opts]
   (let [rows (mapv audit-row (source-rows opts))
         events (mapcat :strict-violations rows)
         by-class (frequencies (map :strict-class rows))]
     {:kind :strict-audit-report
      :schema :pnix-clj.strict-audit-report.v0
      :policy :audit-only-no-behavior-change
      :source-count (count rows)
      :strict-ok (get by-class :strict-ok 0)
      :strict-violation (get by-class :strict-violation 0)
      :held (get by-class :held 0)
      :source-family-counts (count-by :source-family rows)
      :violation-count (count events)
      :violation-counts-by-construct (count-by :construct events)
      :violation-counts-by-issue (count-by :issue events)
      :rows rows})))

(defn- strict-gate-row
  [{:keys [source-id source import-modules] :as row}]
  (let [result (pnix/eval-source-strict source import-modules)]
    (-> row
        (dissoc :source)
        (assoc :strict-gate-status (:status result)
               :strict-gate-reason (:reason result)
               :strict-gate-ok? (= :ok (:status result)))
        (cond-> (= :ok (:status result))
          (assoc :value-observed? true)
          (not= :ok (:status result))
          (assoc :held-error (select-keys result [:reason :error]))))))

(defn strict-gate-report
  "Run opt-in strict mode over the rows classified as :strict-ok by Phase B."
  ([] (strict-gate-report {}))
  ([opts]
   (let [rows (source-rows opts)
         classified (mapv audit-row rows)
         strict-ok-ids (set (map :source-id
                                 (filter #(= :strict-ok (:strict-class %))
                                         classified)))
         gate-rows (mapv strict-gate-row
                         (filter #(contains? strict-ok-ids (:source-id %))
                                 rows))
         failed (remove :strict-gate-ok? gate-rows)]
     {:kind :strict-gate-report
      :schema :pnix-clj.strict-gate-report.v0
      :policy :strict-mode-over-strict-ok-corpus
      :classified-source-count (count classified)
      :strict-ok-source-count (count strict-ok-ids)
      :checked (count gate-rows)
      :ok (count (filter :strict-gate-ok? gate-rows))
      :failed (count failed)
      :first-failed (first failed)
      :rows gate-rows})))

(defn -main
  [& [mode]]
  (if (= "gate" mode)
    (let [{:keys [classified-source-count checked ok failed first-failed]}
          (strict-gate-report)]
      (println (format "pnix-clj strict gate: classified=%d checked=%d ok=%d failed=%d"
                       classified-source-count checked ok failed))
      (when first-failed
        (println "first failed:" (pr-str first-failed)))
      (shutdown-agents)
      (when (pos? failed)
        (System/exit 1)))
    (let [{:keys [source-count strict-ok strict-violation held
                  violation-count violation-counts-by-construct
                  violation-counts-by-issue]} (report)]
      (println (format "pnix-clj strict audit: sources=%d strict-ok=%d violations=%d held=%d events=%d"
                       source-count strict-ok strict-violation held violation-count))
      (println "constructs:" (pr-str violation-counts-by-construct))
      (println "issues:" (pr-str violation-counts-by-issue))
      (shutdown-agents))))
