(ns pnix-clj.lane-registry
  "Generated lane registry: reads each top-level pnix-clj lane-classification
  and renders a deterministic Markdown registry. This is the human-facing map
  of the pnix-clj constitution: core / proof-only / experimental boundaries."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :lane-classification-registry
   :role :render-pnix-clj-source-lane-policy-map
   :product-runtime :allowed
   :semantic-authority :registry-only
   :mutation :generated-doc-write-only
   :admission :drift-check-gated
   :determinism :required
   :allowed-output :lane-registry-document-or-drift-verdict})

(def doc-path
  "docs/LANE_REGISTRY.md")

(defn- top-level-source-files []
  (->> (.listFiles (io/file "src/pnix_clj"))
       (filter #(.isFile %))
       (filter #(str/ends-with? (.getName %) ".clj"))
       (sort-by #(.getName %))
       vec))

(defn- ns-symbol-from-file [f]
  (let [text (slurp f)
        [_ ns-name] (re-find #"\(ns\s+([^\s\)]+)" text)]
    (symbol ns-name)))

(defn- lane-block [text]
  (let [start (.indexOf text "(def lane-classification")]
    (when (not (neg? start))
      (let [end (.indexOf text "\n\n" start)]
        (subs text start (if (neg? end) (count text) end))))))

(defn- read-lane-map [f]
  (let [block (lane-block (slurp f))]
    (when block
      (binding [*read-eval* false]
        (nth (read-string block) 2)))))

(defn registry-rows []
  (mapv (fn [f]
          (let [m (read-lane-map f)]
            (merge {:file (.getName f)
                    :namespace (str (ns-symbol-from-file f))}
                   m)))
        (top-level-source-files)))

(defn lane-counts [rows]
  (into (sorted-map)
        (frequencies (map :lane rows))))

(defn- cell [x]
  (-> (if (nil? x) "" (name x))
      (str/replace "|" "\\|")))

(defn- table-row [xs]
  (str "| " (str/join " | " (map cell xs)) " |"))

(defn render
  ([] (render (registry-rows)))
  ([rows]
   (let [counts (lane-counts rows)
         sorted-rows (sort-by (juxt :lane :namespace) rows)]
     (str "# pnix-clj lane registry (generated — do not edit)\n\n"
          "Regenerate with `clojure -M:lane-registry`.\n\n"
          "This document is generated from top-level `src/pnix_clj/*.clj` "
          "`lane-classification` maps. It records the constitutional boundary "
          "between core runtime substrate, proof-only evidence lanes, and "
          "experimental self-change/dev lanes.\n\n"

          "## Counts\n\n"
          (str/join "\n"
                    (for [[lane n] counts]
                      (str "- `" lane "`: " n)))
          "\n\n"

          "## Registry\n\n"
          (table-row ["lane" "namespace" "scope" "product-runtime"
                      "semantic-authority" "admission" "allowed-output"])
          "\n"
          (table-row ["---" "---" "---" "---" "---" "---" "---"])
          "\n"
          (str/join
           "\n"
           (for [r sorted-rows]
             (table-row [(:lane r)
                         (:namespace r)
                         (:scope r)
                         (:product-runtime r)
                         (:semantic-authority r)
                         (:admission r)
                         (:allowed-output r)])))
          "\n\n"
          "## Registry hash\n\n"
          "`" (hash/sha256 (pr-str sorted-rows)) "`\n"))))

(defn generate! []
  (let [content (render)]
    (io/make-parents doc-path)
    (spit doc-path content)
    {:status :ok
     :path doc-path
     :hash (hash/sha256 content)
     :row-count (count (registry-rows))
     :counts (lane-counts (registry-rows))}))

(defn check []
  (let [expected (render)
        actual (when (.exists (io/file doc-path)) (slurp doc-path))]
    (cond
      (nil? actual)
      {:status :failed
       :reason :lane-registry-doc-missing
       :path doc-path}

      (= expected actual)
      {:status :ok
       :path doc-path
       :hash (hash/sha256 expected)
       :row-count (count (registry-rows))
       :counts (lane-counts (registry-rows))}

      :else
      {:status :failed
       :reason :lane-registry-doc-drift
       :path doc-path
       :expected-hash (hash/sha256 expected)
       :actual-hash (hash/sha256 actual)})))

(defn -main [& args]
  (let [{:keys [status path hash reason row-count counts]}
        (if (= "check" (first args)) (check) (generate!))]
    (println
     (format "pnix-clj lane-registry: status=%s path=%s rows=%s counts=%s %s"
             (name status)
             path
             (or row-count "-")
             (pr-str counts)
             (or hash (str "reason=" reason))))
    (shutdown-agents)
    (when (not= :ok status)
      (System/exit 1))))
