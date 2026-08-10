(ns pnix-clj.rust-batch
  "Rust-grounded invariance corpus: fixtures ported from the Rust pnix implementation, pinning cross-implementation value agreement."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.core :as pnix]
            [pnix-clj.error :as err]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :cross-implementation-invariance-corpus
   :product-runtime :forbidden
   :rust-product-lane :forbidden
   :mutation :forbidden
   :admission :forbidden
   :allowed-input :imported-rust-grounded-fixtures
   :allowed-output :equivalence-report})

(def batch-resource-root
  "pnix_clj/rust_grounded/invariance_corpus")

(def manifest-resource
  "pnix_clj/rust_grounded/manifest.edn")

(def oracle-resource
  "pnix_clj/rust_grounded/oracles.edn")

(def source-root-note
  batch-resource-root)

(def rust-test-regex
  #"(?m)#\[test\]\s*\nfn\s+([A-Za-z0-9_]+)\s*\(")

(defn- resource-url
  []
  (or (io/resource batch-resource-root)
      (throw (ex-info "rust grounded batch resource root missing"
                      {:resource batch-resource-root}))))

(defn- px-file?
  [^java.io.File f]
  (and (.isFile f)
       (str/ends-with? (.getName f) ".px")))

(defn manifest
  []
  (if-let [resource (io/resource manifest-resource)]
    (read-string (slurp resource))
    (throw (ex-info "rust grounded manifest missing"
                    {:resource manifest-resource}))))

(defn oracle-set
  []
  (if-let [resource (io/resource oracle-resource)]
    (read-string (slurp resource))
    {:kind :rust-grounded-oracle-set-missing
     :oracles {}}))

(defn- oracle-result
  [oracle-set source-id]
  (or (get-in oracle-set [:oracles source-id])
      (err/failed :evidence :rust-grounded-oracle-not-imported
                  {:source-id source-id})))

(defn- rust-test-names
  [source]
  (mapv second (re-seq rust-test-regex source)))

(defn- suite-source-row
  [{:keys [id source-path repo-path source-hash imported-test-count status] :as suite}]
  (if-not (and repo-path source-hash)
    (select-keys suite [:id :source-path :status :role])
    (if-let [resource (io/resource repo-path)]
      (let [source (slurp resource)
            actual-hash (hash/sha256 source)
            tests (rust-test-names source)]
        {:id id
         :status status
         :source-path source-path
         :repo-path repo-path
         :source-hash actual-hash
         :expected-source-hash source-hash
         :hash-matches? (= source-hash actual-hash)
         :line-count (count (str/split-lines source))
         :test-count (count tests)
         :expected-test-count imported-test-count
         :test-count-matches? (= imported-test-count (count tests))
         :test-names tests})
      {:id id
       :status :missing-repo-source
       :source-path source-path
       :repo-path repo-path
       :expected-source-hash source-hash
       :expected-test-count imported-test-count})))

(defn suite-source-inventory
  []
  (mapv suite-source-row (:required-suites (manifest))))

(defn batch-cases
  []
  (let [root (io/file (.toURI (resource-url)))
        source-revision (:source-revision (manifest))
        oracle-set (oracle-set)]
    (->> (file-seq root)
         (filter px-file?)
         (sort-by #(.getName %))
         (mapv (fn [f]
                 (let [source (slurp f)
                       source-id (keyword "rust-grounded"
                                          (str/replace (.getName f) #"\.px$" ""))]
                   {:source-id source-id
                    :source source
                    :batch :rust-grounded/invariance-corpus
                    :fixture-path (str batch-resource-root "/" (.getName f))
                    :source-origin source-root-note
                    :source-revision source-revision
                    :fixture-hash (hash/sha256 source)
                    :oracle-result (oracle-result oracle-set source-id)}))))))

(defn report
  []
  (let [cases (batch-cases)
        pnix-report (pnix/report cases)
        manifest (manifest)
        oracle-set (oracle-set)
        suite-source-inventory (suite-source-inventory)
        imported-suite-sources (filter #(= :imported-source (:status %))
                                       suite-source-inventory)]
    (assoc pnix-report
           :kind :rust-grounded-batch-report
           :manifest-kind (:kind manifest)
           :oracle-kind (:kind oracle-set)
           :oracle-source-revision (:source-revision oracle-set)
           :suite-source-inventory suite-source-inventory
           :imported-suite-source-count (count imported-suite-sources)
           :imported-rust-test-count (reduce + 0 (map :test-count imported-suite-sources))
           :authority-order (:authority-order manifest)
           :pnix-clj-lanes (:pnix-clj-lanes manifest)
           :required-suites (:required-suites manifest)
           :source-revision (:source-revision manifest)
           :batch :rust-grounded/invariance-corpus
           :source-origin source-root-note
           :fixture-count (count cases)
           :fixture-hashes (mapv #(select-keys % [:source-id :fixture-path :fixture-hash])
                                 cases))))

(defn -main
  [& _]
  (let [{:keys [fixture-count accepted rejected held first-held first-rejected
                first-frontier fixture-hashes required-suites reason-counts
                source-revision imported-suite-source-count imported-rust-test-count]}
        (report)]
    (println (format "pnix-clj rust-grounded batch: fixtures=%d accepted=%d rejected=%d held=%d"
                     fixture-count accepted rejected held))
    (println "required suites:")
    (doseq [{:keys [id status imported-count imported-test-count pnix-hy-count]} required-suites]
      (println (format "  %s status=%s imported=%s imported-tests=%s pnix-hy-count=%s"
                       (name id)
                       (name status)
                       (or imported-count "-")
                       (or imported-test-count "-")
                       (or pnix-hy-count "-"))))
    (println "source-origin:" source-root-note)
    (println "source-revision:" (pr-str source-revision))
    (println "imported suite sources:" imported-suite-source-count
             "rust test functions:" imported-rust-test-count)
    (println "reason-counts:" (pr-str reason-counts))
    (doseq [{:keys [source-id fixture-path fixture-hash]} fixture-hashes]
      (println (format "  %s %s %s"
                       (name source-id) fixture-path fixture-hash)))
    (when first-held
      (println "first held:" (pr-str (select-keys first-held
                                                  [:source-id :reason]))))
    (when first-rejected
      (println "first rejected:" (pr-str (select-keys first-rejected
                                                      [:source-id :reason]))))
    (when first-frontier
      (println "first frontier:" (pr-str first-frontier)))
    (shutdown-agents)
    (when (pos? rejected)
      (System/exit 1))))
