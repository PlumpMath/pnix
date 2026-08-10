(ns pnix-clj.capabilities
  "Machine-generated capability index (roadmap M4), role-translated from
  pnix-hy's --capabilities: an anti-duplication lookup of what this repo can
  already do, derived ONLY from code (no hand-written text, no timestamps),
  rendered to docs/CAPABILITIES.md and drift-checked in the gate — if the code
  gains or loses a capability without regenerating the doc, the gate fails."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.receipt :as receipt]
            [pnix-clj.report-artifact :as report-artifact]))

(def lane-classification
  {:lane :core
   :scope :capability-index-generator
   :role :deterministic-public-api-and-report-index
   :product-runtime :allowed
   :semantic-authority :index-only
   :mutation :generated-doc-write-only
   :admission :drift-check-gated
   :determinism :required
   :allowed-output :capability-index-or-drift-verdict})

(def ^:private doc-path "docs/CAPABILITIES.md")

(def ^:private api-namespaces
  "Public-API surfaces worth indexing (fixed, sorted at render)."
  ['pnix-clj.arith-proof
   'pnix-clj.cas
   'pnix-clj.store
   'pnix-clj.bool-proof
   'pnix-clj.cached-eval
   'pnix-clj.core
   'pnix-clj.form-analysis
   'pnix-clj.futamura
   'pnix-clj.evaluator
   'pnix-clj.interop
   'pnix-clj.lowering
   'pnix-clj.mirror
   'pnix-clj.mirror-chain
   'pnix-clj.parser
   'pnix-clj.persist
   'pnix-clj.purity
   'pnix-clj.replay
   'pnix-clj.property-fuzzer
   'pnix-clj.search
   'pnix-clj.receipt
   'pnix-clj.reflect
   'pnix-clj.safe-eval
   'pnix-clj.snapshot
   'pnix-clj.specialize
   'pnix-clj.synthesize
   'pnix-clj.tower
   'pnix-clj.unparse
   'pnix-clj.witness
   'pnix-clj.cegis
   'pnix-clj.generate
   'pnix-clj.self-improve
   'pnix-clj.self-mod-gate
   'pnix-clj.witnessed-run
   'pnix-clj.wiki])

(defn- deps-aliases
  []
  (->> (read-string (slurp "deps.edn"))
       :aliases
       keys
       (map name)
       sort
       vec))

(defn- ns-public-names
  [sym]
  (require sym)
  (->> (ns-publics sym) keys (map name) sort vec))

(defn index
  "The capability index as data (everything sorted; deterministic)."
  []
  (let [builtins (vec (sort (keys (get evaluator/default-env "builtins"))))
        unprefixed (vec (sort (remove #{"builtins"}
                                      (keys evaluator/default-env))))]
    {:kind :pnix-capability-index
     :schema :pnix-clj.capability-index.v0
     :lanes (mapv name receipt/lane-order)
     :report-artifacts (vec (sort (map name report-artifact/supported-kinds)))
     :deps-aliases (deps-aliases)
     :builtin-count (count builtins)
     :builtins builtins
     :unprefixed-default-scope unprefixed
     :public-api (into (sorted-map)
                       (map (fn [n] [(str n) (ns-public-names n)]))
                       api-namespaces)}))

(defn- wrap-names
  [names per-line]
  (->> (partition-all per-line names)
       (map #(str/join " " %))
       (map #(str "  " %))
       (str/join "\n")))

(defn render
  "Render the index to markdown. Deterministic: same code -> same bytes."
  [{:keys [lanes report-artifacts deps-aliases builtin-count builtins
           unprefixed-default-scope public-api]}]
  (str "# pnix-clj capabilities (generated — do not edit)\n\n"
       "Regenerate with `clojure -M:capabilities`; the gate runs\n"
       "`clojure -M:capabilities-check` and fails on drift. Use this index to\n"
       "check whether something already exists before implementing it.\n\n"
       "## Lanes (receipt/lane-order)\n\n"
       (str/join "\n" (map #(str "- " %) lanes)) "\n\n"
       "## Report artifacts (report-artifact/supported-kinds)\n\n"
       (str/join "\n" (map #(str "- " %) report-artifacts)) "\n\n"
       "## Deps aliases\n\n"
       (wrap-names deps-aliases 6) "\n\n"
       "## Builtins (" builtin-count ")\n\n"
       (wrap-names builtins 6) "\n\n"
       "## Unprefixed default scope\n\n"
       (wrap-names unprefixed-default-scope 8) "\n\n"
       "## Public API\n\n"
       (str/join "\n\n"
                 (map (fn [[ns-name fns]]
                        (str "### " ns-name "\n\n" (wrap-names fns 5)))
                      public-api))
       "\n"))

(defn generate!
  []
  (let [content (render (index))]
    (io/make-parents doc-path)
    (spit doc-path content)
    {:status :ok :path doc-path :hash (hash/sha256 content)}))

(defn check
  "Drift check: the committed doc must equal a fresh render."
  []
  (let [expected (render (index))
        actual (when (.exists (io/file doc-path)) (slurp doc-path))]
    (cond
      (nil? actual)
      {:status :failed :reason :capabilities-doc-missing :path doc-path}

      (= expected actual)
      {:status :ok :path doc-path :hash (hash/sha256 expected)}

      :else
      {:status :failed :reason :capabilities-doc-drift
       :path doc-path
       :expected-hash (hash/sha256 expected)
       :actual-hash (hash/sha256 actual)})))

(defn -main
  [& args]
  (let [{:keys [status path hash reason] :as r}
        (if (= "check" (first args)) (check) (generate!))]
    (println (format "pnix-clj capabilities: %s status=%s path=%s %s"
                     (if (= "check" (first args)) "check" "generate")
                     (name status) path
                     (or hash (str "reason=" reason))))
    (shutdown-agents)
    (when (not= :ok status) (System/exit 1))))
