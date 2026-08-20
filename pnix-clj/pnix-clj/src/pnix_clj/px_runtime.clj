(ns pnix-clj.px-runtime
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.error :as err]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.parser :as parser]))

(def lane-classification
  {:lane :core
   :scope :px-runtime-artifact-boundary
   :role :inventory-and-check-owned-px-runtime-artifacts
   :product-runtime :allowed
   :semantic-authority :runtime-artifact-evidence-only
   :autonomous-execution :forbidden
   :external-runtime-roots :forbidden
   :mutation :forbidden
   :admission :forbidden
   :determinism :artifact-hash-required
   :allowed-output :runtime-boundary-or-artifact-report})

(def runtime-resource-root
  "pnix_clj/pnix_runtime")

(def allowed-runtime-roots
  ["pnixc-pnix" "stdlib"])

(defn repo-root
  []
  (.getCanonicalFile (io/file (System/getProperty "user.dir"))))

(defn runtime-container-root
  []
  (if-let [resource (io/resource runtime-resource-root)]
    (io/file (.toURI resource))
    (io/file (repo-root) "resources" runtime-resource-root)))

(defn runtime-roots
  []
  (let [root (runtime-container-root)]
    (mapv #(io/file root %) allowed-runtime-roots)))

(defn runtime-root
  [root-name]
  (when (some #{root-name} allowed-runtime-roots)
    (io/file (runtime-container-root) root-name)))

(defn- px-file?
  [f]
  (and (.isFile f)
       (str/ends-with? (.getName f) ".px")))

(defn- relative-path
  [root f]
  (let [root-path (.getCanonicalPath root)
        file-path (.getCanonicalPath f)]
    (if (str/starts-with? file-path root-path)
      (subs file-path (inc (count root-path)))
      file-path)))

(defn runtime-artifacts
  "Inventory pnix-clj-owned .px runtime artifacts from resources. This never
  depends on original/new pnix or parent checkout directories at runtime."
  []
  (->> (runtime-roots)
       (filter #(.exists %))
       (mapcat (fn [root]
                 (for [f (file-seq root)
                       :when (px-file? f)]
                   {:root (.getName root)
                    :relative-path (relative-path root f)
                    :path (.getCanonicalPath f)
                    :hash (hash/sha256 (slurp f))})))
       (sort-by (juxt :root :relative-path))
       vec))

(defn runtime-artifacts-under
  [root-name]
  (let [root (runtime-root root-name)]
    (if-not (and root (.exists root))
      []
      (->> (for [f (file-seq root)
                 :when (px-file? f)]
             {:root root-name
              :relative-path (relative-path root f)
              :path (.getCanonicalPath f)
              :hash (hash/sha256 (slurp f))})
           (sort-by :relative-path)
           vec))))

(defn artifact-at
  [root-name relative]
  (when-let [root (runtime-root root-name)]
    (let [f (io/file root relative)]
      (when (and (.exists f) (px-file? f))
        {:root root-name
         :relative-path (relative-path root f)
         :path (.getCanonicalPath f)
         :hash (hash/sha256 (slurp f))}))))

(defn select-runtime-artifact
  []
  (or (artifact-at "pnixc-pnix" "exec/runtime.px")
      (artifact-at "stdlib" "lib/lisp/pnix-lisp.px")))

(defn runtime-boundary
  []
  (let [container (runtime-container-root)
        roots (mapv (fn [root-name]
                      (let [root (runtime-root root-name)]
                        {:root root-name
                         :path (.getCanonicalPath root)
                         :exists (.exists root)}))
                    allowed-runtime-roots)
        missing (vec (remove :exists roots))
        entry (select-runtime-artifact)]
    {:kind :px-runtime-boundary
     :status (if (and (empty? missing) entry) :ok :failed)
     :reason (cond
               (seq missing) :px-runtime-boundary-roots-missing
               (not entry) :px-runtime-entry-missing
               :else :px-runtime-boundary-defined)
     :resource-root runtime-resource-root
     :container-path (.getCanonicalPath container)
     :allowed-roots allowed-runtime-roots
     :root-status roots
     :entry (select-keys entry [:root :relative-path :hash])
     :external-runtime-roots-forbidden true
     :parent-checkouts-runtime-dependency false}))

(defn- strip-comments
  [source]
  (->> (str/split-lines source)
       (map #(first (str/split % #"#" 2)))
       (str/join "\n")))

(defn imports
  "Extract direct import forms from a .px source. This is a tracking scanner,
  not a pnix parser; it intentionally records only import wiring."
  [source]
  (->> (re-seq #"\b(?:import|scopedImport)\s+([^;\s]+)" (strip-comments source))
       (mapv (fn [[_ target]] target))))

(defn- artifact-index
  [artifacts]
  (into {} (map (fn [artifact]
                  [[(:root artifact) (:relative-path artifact)] artifact])
                artifacts)))

(defn- normalize-relative-import
  [from-relative import-target]
  (let [parent (io/file from-relative "..")
        target (io/file (.getPath parent) import-target)]
    (-> (.normalize (.toPath target))
        str
        (str/replace #"^\./" ""))))

(defn resolve-import
  [index artifact import-target]
  (when (and (:root artifact)
             (:relative-path artifact)
             (str/starts-with? import-target "."))
    (get index [(:root artifact)
                (normalize-relative-import (:relative-path artifact)
                                           import-target)])))

(defn import-edges
  ([artifact]
   (import-edges (artifact-index (runtime-artifacts-under (:root artifact)))
                 artifact))
  ([index artifact]
   (let [source (slurp (:path artifact))]
     (mapv (fn [target]
             (let [resolved (resolve-import index artifact target)]
               {:from-root (:root artifact)
                :from (:relative-path artifact)
                :import target
                :to-root (:root resolved)
                :to (:relative-path resolved)
                :target-hash (:hash resolved)
                :status (if resolved :resolved :missing)}))
           (imports source)))))

(defn- edge-source-node
  [edge]
  [(:from-root edge) (:from edge)])

(defn- edge-target-node
  [edge]
  (when (= :resolved (:status edge))
    [(:to-root edge) (:to edge)]))

(defn- node-row
  [[root relative-path]]
  {:root root
   :relative-path relative-path})

(defn- graph-cycles
  [adjacency]
  (let [permanent (atom #{})
        temporary (atom #{})
        stack (atom [])
        cycles (atom [])]
    (letfn [(cycle-from-stack [node]
              (let [path @stack
                    tail (vec (drop-while #(not= node %) path))]
                (conj tail node)))
            (visit [node]
              (cond
                (@permanent node)
                nil

                (@temporary node)
                (swap! cycles conj (cycle-from-stack node))

                :else
                (do
                  (swap! temporary conj node)
                  (swap! stack conj node)
                  (doseq [next-node (sort-by pr-str (get adjacency node))]
                    (visit next-node))
                  (swap! stack pop)
                  (swap! temporary disj node)
                  (swap! permanent conj node))))]
      (doseq [node (sort-by pr-str (keys adjacency))]
        (visit node))
      @cycles)))

(defn import-graph-analysis
  "Analyze repo-owned runtime import edges. This is graph/receipt analysis only;
  it does not resolve outside resources/pnix_clj/pnix_runtime."
  [edges]
  (let [resolved (filter #(= :resolved (:status %)) edges)
        missing (filter #(= :missing (:status %)) edges)
        adjacency (reduce (fn [m edge]
                            (let [from (edge-source-node edge)
                                  to (edge-target-node edge)]
                              (cond-> (update m from (fnil identity #{}))
                                to (update from conj to))))
                          {}
                          resolved)
        nodes (set (concat (map edge-source-node edges)
                           (keep edge-target-node edges)))
        cycles (graph-cycles adjacency)]
    {:schema :pnix-clj.px-runtime.import-graph.v0
     :node-count (count nodes)
     :edge-count (count edges)
     :resolved-edge-count (count resolved)
     :missing-edge-count (count missing)
     :cycle-count (count cycles)
     :acyclic? (empty? cycles)
     :cycles (mapv #(mapv node-row %) (take 3 cycles))
     :cycle-policy :failed-not-recursive-import
     :missing-import-policy :failed-not-host-fallback}))

(defn runtime-entry-parse
  [entry]
  (if-not entry
    (merge {:kind :px-runtime-entry-parse}
           (err/failed :bootstrap :px-runtime-artifact-missing))
    (let [source (slurp (:path entry))
          result (parser/parse-source source)]
      (merge {:kind :px-runtime-entry-parse
              :entry (select-keys entry [:root :relative-path :hash])
              :source-hash (hash/sha256 source)
              :status (:status result)
              :reason (if (= :ok (:status result))
                        :px-runtime-entry-parse-ok
                        (:reason result))
              :ast-op (get-in result [:ast :op])
              :span (or (:span result)
                        (get-in result [:ast :span]))}
             (when (not= :ok (:status result))
               {:unsupported-syntax (vec (take 1 (:unsupported-syntax result)))
                :error (:error result)})))))

(defn- runtime-value-summary
  [value]
  (let [describe (-> (get value "describe") evaluator/force-whnf :value)
        primitives (-> (get describe "primitives") evaluator/force-whnf :value)
        verbs (-> (get describe "verbs") evaluator/force-whnf :value)]
    {:top-level-keys (->> (keys value) sort vec)
     :has-spawn? (contains? value "spawn")
     :has-project? (contains? value "project")
     :has-describe? (contains? value "describe")
     :primitive-count (if (map? primitives) (count primitives) 0)
     :verbs (if (vector? verbs) verbs [])}))

(defn- runtime-eval-session
  []
  (let [cache (atom {})
        stats (atom {:hits 0
                     :misses 0
                     :cycles 0})
        cycles (atom [])
        stack (atom [])]
    (letfn [(artifact-id [artifact]
              [(:root artifact) (:relative-path artifact)])
            (cycle-row [artifact]
              (let [cycle-path (conj @stack (artifact-id artifact))
                    row (err/failed :import
                                  :px-runtime-import-cycle
                                  {:artifact (select-keys artifact
                                                          [:root :relative-path
                                                           :hash])
                                   :stack @stack
                                   :cycle-path cycle-path})]
                (swap! stats update :cycles inc)
                (swap! cycles conj row)
                row))
            (failed-row [artifact reason extra]
              (merge (err/failed :bootstrap reason)
                     {:artifact (select-keys artifact
                                             [:root :relative-path :hash])}
                     extra))
            (cache-summary []
              {:schema :pnix-clj.px-runtime.import-cache.v0
               :policy :repo-owned-artifact-id
               :key-fields [:root :relative-path]
               :entry-count (count @cache)
               :hit-count (:hits @stats)
               :miss-count (:misses @stats)
               :cycle-count (:cycles @stats)
               :cycle-policy :failed-not-recursive-import})
            (eval-artifact [artifact]
              (let [id (artifact-id artifact)]
                (cond
                  (contains? @cache id)
                  (do
                    (swap! stats update :hits inc)
                    (get @cache id))

                  (some #{id} @stack)
                  (cycle-row artifact)

                  :else
                  (do
                    (swap! stats update :misses inc)
                    (swap! stack conj id)
                    (try
                      (let [source (slurp (:path artifact))
                            parsed (parser/parse-source source)]
                        (if (not= :ok (:status parsed))
                          (failed-row artifact
                                      :px-runtime-artifact-parse-failed
                                      {:parse-result
                                       (select-keys parsed
                                                    [:status :reason :span
                                                     :unsupported-syntax])})
                          (let [import-resolver
                                (fn [context target & _scope]
                                  (let [from-artifact
                                        (if (and (map? context)
                                                 (contains? context :origin))
                                          (:origin context)
                                          context)
                                        index (artifact-index
                                               (runtime-artifacts-under
                                                (:root from-artifact)))
                                        resolved (resolve-import index
                                                                 from-artifact
                                                                 target)]
                                    (if resolved
                                      (eval-artifact resolved)
                                      (err/failed :import
                                                :px-runtime-import-missing
                                                {:from (select-keys
                                                        from-artifact
                                                        [:root
                                                         :relative-path
                                                         :hash])
                                                 :target target}))))
                                result (binding [evaluator/*import-context* []
                                                 evaluator/*import-origin* artifact
                                                 evaluator/*import-resolver* import-resolver]
                                         (evaluator/eval-ast
                                          (:ast parsed)
                                          (assoc evaluator/default-env
                                                 evaluator/import-context-key
                                                 artifact)))]
                            (when (= :ok (:status result))
                              (swap! cache assoc id result))
                            result)))
                      (catch Throwable t
                        (merge
                         (err/failed-throwable :bootstrap
                                               :px-runtime-bootstrap-threw
                                               t)
                         {:artifact (select-keys artifact
                                                [:root :relative-path :hash])}))
                      (finally
                        (swap! stack pop)))))))]
      {:eval-artifact eval-artifact
       :cache cache
       :cache-summary cache-summary
       :cycles cycles
       :stack stack})))

(defn runtime-bootstrap
  "Parse/evaluate the internal VM composition and its repo-owned imports. This
  proves the .px runtime artifact can bootstrap, but it does not execute a user
  source expression through that runtime."
  ([]
   (runtime-bootstrap (select-runtime-artifact)))
  ([entry]
   (if-not entry
     (merge {:kind :px-runtime-bootstrap}
            (err/failed :bootstrap :px-runtime-artifact-missing))
     (let [{:keys [eval-artifact cache-summary cycles]} (runtime-eval-session)
           entry-parse (runtime-entry-parse entry)
           result (eval-artifact entry)]
       (merge {:kind :px-runtime-bootstrap
               :entry (select-keys entry [:root :relative-path :hash])
               :entry-parse entry-parse
               :import-cache (cache-summary)
               :import-cycles @cycles
               :evaluated-artifact-count (:entry-count (cache-summary))
               :status (:status result)
               :reason (if (= :ok (:status result))
                         :px-runtime-bootstrap-ok
                         (:reason result))}
              (if (= :ok (:status result))
                {:value-summary (runtime-value-summary (:value result))}
                {:failure result}))))))

(defn runtime-artifact-evaluation
  "Evaluate a repo-owned runtime artifact with normal internal import
  resolution. This is for pnix-clj reports that need to inspect a specific .px
  module; it never reads outside resources/pnix_clj/pnix_runtime."
  [root-name relative]
  (let [artifact (artifact-at root-name relative)]
    (if-not artifact
      (merge {:kind :px-runtime-artifact-evaluation
              :artifact {:root root-name
                         :relative-path relative}}
             (err/failed :bootstrap :px-runtime-artifact-missing))
      (let [{:keys [eval-artifact cache-summary cycles]} (runtime-eval-session)
            result (eval-artifact artifact)]
        (merge {:kind :px-runtime-artifact-evaluation
                :artifact (select-keys artifact [:root :relative-path :hash])
                :import-cache (cache-summary)
                :import-cycles @cycles
                :evaluated-artifact-count (:entry-count (cache-summary))
                :status (:status result)
                :reason (if (= :ok (:status result))
                          :px-runtime-artifact-evaluation-ok
                          (:reason result))}
               (if (= :ok (:status result))
                 {:value (:value result)}
                 {:failure result}))))))

(defonce ^:private source-runner-cache
  (atom nil))

(def ^:private runtime-source-stack-bytes
  (* 32 1024 1024))

(defn- public-runner-row
  [runner]
  (select-keys runner [:status :reason :error
                       :runtime-artifact :evaluator-artifact
                       :evaluated-artifact-count :import-cache :import-cycles]))

(defn- runtime-failure
  ([phase class]
   (runtime-failure phase class nil))
  ([phase class evidence]
   (cond-> {:status :failed
            :reason class
            :error {:phase phase
                    :class class}}
     (some? evidence) (assoc-in [:error :evidence] evidence))))

(def ^:private runtime-error-classes
  {"syntax-error" :syntax-error
   "invalid-machine-state" :invalid-machine-state
   "unsupported-expression" :unsupported-expression
   "assertion-failed" :assertion-failed
   "string-interpolation-coercion-failed" :string-interpolation-coercion-failed
   "throw-builtin-called" :throw-builtin-called
   "non-boolean-condition" :non-boolean-condition
   "unbound-var" :unknown-variable
   "unknown-variable" :unknown-variable
   "missing-attr" :attribute-missing
   "attribute-missing" :attribute-missing
   "call-target-not-callable" :not-callable
   "not-callable" :not-callable
   "wrong-type" :type-error
   "type-error" :type-error
   "division-by-zero" :division-by-zero
   "integer-overflow" :integer-overflow
   "invalid-regex" :invalid-regex
   "hash-string-raw-bytes-unsupported" :hash-string-raw-bytes-unsupported
   "hash-string-algorithm-not-string" :hash-string-algorithm-not-string
   "hash-string-algorithm-has-context" :hash-string-algorithm-has-context
   "hash-string-unsupported-algorithm" :hash-string-unsupported-algorithm
   "hash-string-data-not-string" :hash-string-data-not-string
   "infinite-recursion" :cycle-detected
   "cycle-detected" :cycle-detected
   "abort-builtin-called" :abort-builtin-called
   "throw-argument-not-string" :throw-argument-not-string
   "abort-argument-not-string" :abort-argument-not-string
   "unexpected-lambda-pattern-arg" :unexpected-lambda-pattern-arg
   "function-args-target-not-callable" :function-args-target-not-callable
   "head-of-empty-list" :head-of-empty-list
   "tail-of-empty-list" :tail-of-empty-list
   "last-of-empty-list" :last-of-empty-list
   "init-of-empty-list" :init-of-empty-list
   "effect-denied" :effect-denied
   "invalid-guest-value" :invalid-guest-value})

(defn- canonical-runtime-error-class
  [value]
  (let [reason (or (get value "reason")
                   (get-in value ["failure" "class"])
                   (get-in value ["error" "class"])
                   (get-in value ["error" :class]))
        reason-name (cond
                      (keyword? reason) (name reason)
                      (string? reason) reason
                      :else nil)]
    (get runtime-error-classes reason-name :px-runtime-run-error)))

(defn- canonical-runtime-error-phase
  [value]
  (case (get-in value ["failure" "phase"])
    "parse" :parse
    "resolution" :resolution
    "effect" :effect
    "effect-contract" :effect-contract
    "primitive-contract" :primitive-contract
    :eval))

(defn- normalize-runtime-execution-result
  [result fallback-phase fallback-class]
  (cond
    (= :ok (:status result)) result
    (= :failed (:status result)) result
    (= :suspended (:status result)) result
    (= :requested (:status result)) result
    :else
    (runtime-failure fallback-phase
                     (or (get-in result [:error :class])
                         (:reason result)
                         fallback-class))))

(defn- apply-runtime-source
  [run-fn source-str]
  (let [result (promise)
        runner (bound-fn []
                 (deliver result
                          (try
                            (evaluator/apply-callable run-fn source-str)
                            (catch Throwable _
                              (runtime-failure :eval
                                               :px-runtime-run-failed)))))
        thread (Thread. nil
                        ^Runnable runner
                        "pnix-clj-px-runtime-source"
                        (long runtime-source-stack-bytes))]
    (.start thread)
    @result))

(defn- force-runtime-value
  [value phase]
  (let [result (promise)
        runner (bound-fn []
                 (deliver result
                          (try
                            (evaluator/force-normal value)
                            (catch Throwable _
                              (runtime-failure phase
                                               :px-runtime-force-failed)))))
        thread (Thread. nil
                        ^Runnable runner
                        "pnix-clj-px-runtime-force"
                        (long runtime-source-stack-bytes))]
    (.start thread)
    @result))

(defn- in-memory-import-resolver
  [import-modules]
  (let [import-modules (into {} (map (fn [[target source]]
                                      [(str target) source])
                                    import-modules))
        parse-cache (atom {})
        eval-cache (atom {})
        parse-module (fn [target]
                       (or (get @parse-cache target)
                           (when-let [source (get import-modules target)]
                             (let [parsed (parser/parse-source source)]
                               (swap! parse-cache assoc target parsed)
                               parsed))))]
    (letfn [(resolver [context target scope]
              (let [structured? (map? context)
                    chain (vec (map str (or (if structured?
                                              (:chain context)
                                              context)
                                            [])))
                    origin (if structured? (:origin context) (last chain))
                    target (evaluator/contextual-import-target
                            (if origin [origin] []) target)]
                (cond
                  (some #(= target %) chain)
                  (runtime-failure :eval
                                   :import-module-cycle
                                   {:target target :chain chain})

                  (not (contains? import-modules target))
                  (runtime-failure :eval
                                   :import-module-not-found
                                   {:target target})

                  ;; scopedImport with a NON-empty scope. The `.px` wrapper
                  ;; deep-forces the scope before it crosses to the host, so it
                  ;; arrives as plain host values — inject it by merging into
                  ;; the imported module's global env (like the direct lane).
                  ;; Not cached (same target under different scopes differs) and
                  ;; not propagated to nested plain imports. A scope that came
                  ;; through as an error value (an erroring key forced eagerly)
                  ;; propagates as a deterministic evaluation failure.
                  (and (map? scope) (seq scope)
                       (= "error" (get scope "status")))
                  (runtime-failure :eval
                                   :scoped-import-scope-eval-failed
                                   {:target target})

                  (seq scope)
                  (let [{:keys [status] :as parsed} (parse-module target)]
                    (if (not= :ok status)
                      parsed
                      (let [module-context (conj chain target)]
                        (binding [evaluator/*import-context* module-context
                                  evaluator/*import-origin* target
                                  evaluator/*import-modules* import-modules
                                  evaluator/*import-resolver* resolver]
                          (evaluator/eval-ast (:ast parsed)
                                              (assoc (merge evaluator/default-env scope)
                                                     evaluator/import-context-key
                                                   target))))))

                  :else
                  (if-let [cached (get @eval-cache target)]
                    cached
                    (let [{:keys [status] :as parsed} (parse-module target)]
                      (if (not= :ok status)
                        parsed
                        (let [module-context (conj chain target)
                              result (binding [evaluator/*import-context* module-context
                                              evaluator/*import-origin* target
                                              evaluator/*import-modules* import-modules
                                              evaluator/*import-resolver* resolver]
                                       (evaluator/eval-ast
                                        (:ast parsed)
                                               (assoc evaluator/default-env
                                                      evaluator/import-context-key
                                               target)))]
                          (when (= :ok (:status result))
                            (swap! eval-cache assoc target result))
                          result)))))))]
      resolver)))

(defn- force-path-from-pnix-value
  [value]
  (let [result (evaluator/force-whnf value)]
    (when (= :ok (:status result))
      (:value result))))

(defn- normalize-import-target
  [target]
  (let [value (force-path-from-pnix-value target)]
    (cond
      (string? value)
      value

      (not value)
      (str target)

      (map? value)
      (let [kind (force-path-from-pnix-value (get value "__pnix_value_kind"))
            path (force-path-from-pnix-value (get value "path"))]
        (if (and (= "path" kind) (string? path))
          path
          (str value)))

      :else
      (str value))))

(defn- host-runtime-import-fn
  [target]
  (if (nil? evaluator/*import-resolver*)
    (runtime-failure :runtime-infrastructure
                     :import-evaluation-not-wired
                     {:target (str target)})
    (evaluator/*import-resolver* evaluator/*import-context*
                                 (normalize-import-target target)
                                 nil)))

(defn- host-whnf
  "Force a host value to WHNF, returning the value or nil on non-:ok."
  [v]
  (let [r (evaluator/force-whnf v)]
    (when (= :ok (:status r)) (:value r))))

(defn- px-thunk?
  "A px-evaluator Thunk value as seen from the host: the .px evaluator encodes
  laziness as {__pnixc_meta_tag \"Thunk\", force <1-arg callable>}. The
  record ATTRS are themselves lazy host thunks (the px evaluator rides host
  laziness), so the tag must be WHNF-forced before comparing -- forcing it is
  safe (it is the literal string in the record constructor, never user code)."
  [v]
  (and (map? v)
       (contains? v "__pnixc_meta_tag")
       (contains? v "force")
       (= "Thunk" (host-whnf (get v "__pnixc_meta_tag")))))

(declare bridge-px-scope-value)

(defn- bridged-whnf
  "Force one scope slot to a px-record-free WHNF: host thunks force natively;
  a px Thunk record ({__pnixc_meta_tag Thunk, force <callable>}) is unwrapped
  by applying its force callable (a host-evaluator value); iterate until a
  plain value. Then bridge the value IMMEDIATE children lazily, so nested
  laziness survives too. px meta-tagged records (closures/paths) pass through
  opaque, as before."
  [v]
  (let [r (evaluator/force-whnf v)]
    (if (not= :ok (:status r))
      r
      (let [x (:value r)]
        (if (px-thunk? x)
          (let [rr (evaluator/apply-callable (host-whnf (get x "force")) nil)]
            (if (= :ok (:status rr)) (recur (:value rr)) rr))
          {:status :ok
           :value (cond
                    (and (map? x) (contains? x "__pnixc_meta_tag")) x
                    (map? x) (into {} (map (fn [[k m]] [k (bridge-px-scope-value m)])) x)
                    (vector? x) (mapv bridge-px-scope-value x)
                    :else x)})))))

(defn- bridge-px-scope-value
  "LAZY px->host bridge (roadmap px-scope-laziness): wrap a scope slot as a
  HOST call-by-need thunk. Nothing is evaluated at crossing time; only when
  the imported module actually references the attribute does bridged-whnf run
  -- so an erroring UNUSED scope attribute never evaluates: real Nix
  scopedImport laziness on the px lane."
  [v]
  (evaluator/make-value-thunk :px-scope-bridge (fn [] (bridged-whnf v))))

(defn- host-runtime-scoped-import-fn
  ;; Nix `scopedImport scope path`: scope FIRST, path SECOND (the .px runtime
  ;; applies args left-to-right, so the impl receives [scope target]). Both
  ;; lanes delegate to the same host resolver, so scope injection is shared.
  ;; The scope crosses the px->host boundary LAZILY via bridge-px-scope-value.
  [scope target]
  (if (nil? evaluator/*import-resolver*)
    (runtime-failure :runtime-infrastructure
                     :import-evaluation-not-wired
                     {:target (str target) :builtin :scopedImport})
    (evaluator/*import-resolver* evaluator/*import-context*
                                 (normalize-import-target target)
                                 (when scope
                                   (into {} (map (fn [[k v]]
                                                   [k (bridge-px-scope-value v)]))
                                         scope)))))

(defn- runtime-source-runner
  []
  (if-let [cached @source-runner-cache]
    cached
    (let [{:keys [eval-artifact cache-summary cycles]} (runtime-eval-session)
          runtime-artifact (artifact-at "pnixc-pnix" "exec/runtime.px")
          evaluator-artifact (artifact-at "pnixc-pnix" "eval/evaluator.px")]
      (if (or (not runtime-artifact) (not evaluator-artifact))
        (merge
         (runtime-failure
          :runtime-infrastructure
          (if runtime-artifact
            :px-runtime-evaluator-artifact-missing
            :px-runtime-exec-runtime-artifact-missing))
         {:runtime-artifact (some-> runtime-artifact
                                    (select-keys [:root :relative-path :hash]))
          :evaluator-artifact (some-> evaluator-artifact
                                      (select-keys [:root :relative-path :hash]))})
        (let [runtime-result (eval-artifact runtime-artifact)
              evaluator-result (when (= :ok (:status runtime-result))
                                 (eval-artifact evaluator-artifact))
              runtime-with-imports (when (= :ok (:status runtime-result))
                                    (assoc (:value runtime-result)
                                           "import" {:kind :host-fn
                                                     :arity 1
                                                     :impl host-runtime-import-fn
                                                     :args []}
                                           "scopedImport" {:kind :host-fn
                                                          :arity 2
                                                          :impl host-runtime-scoped-import-fn
                                                          :args []}))
              configured-result
              (when (= :ok (:status evaluator-result))
                (evaluator/apply-callable (:value evaluator-result)
                                          {"runtime" runtime-with-imports}))
              run-fn-result (when (= :ok (:status configured-result))
                              (evaluator/force-whnf
                               (get-in configured-result [:value "run"])))
              mirror-fn-result (when (= :ok (:status configured-result))
                                 (evaluator/force-whnf
                                  (get-in configured-result [:value "runMirror"])))
              run-fn (:value run-fn-result)
              mirror-fn (:value mirror-fn-result)
              cache-row (cache-summary)
              runner
              (merge {:runtime-artifact (select-keys runtime-artifact
                                                     [:root :relative-path :hash])
                      :evaluator-artifact (select-keys evaluator-artifact
                                                       [:root :relative-path :hash])
                      :evaluated-artifact-count (:entry-count cache-row)
                      :import-cache cache-row
                      :import-cycles @cycles}
                     (cond
                       (not= :ok (:status runtime-result))
                       (merge (runtime-failure :runtime-infrastructure
                                               :px-runtime-exec-runtime-failed)
                              {:runtime-result
                               (normalize-runtime-execution-result
                                runtime-result
                                :runtime-infrastructure
                                :px-runtime-exec-runtime-failed)})

                       (not= :ok (:status evaluator-result))
                       (merge (runtime-failure :runtime-infrastructure
                                               :px-runtime-evaluator-failed)
                              {:evaluator-result
                               (normalize-runtime-execution-result
                                evaluator-result
                                :runtime-infrastructure
                                :px-runtime-evaluator-failed)})

                       (not= :ok (:status configured-result))
                       (merge (runtime-failure :runtime-infrastructure
                                               :px-runtime-evaluator-config-failed)
                              {:configured-result
                               (normalize-runtime-execution-result
                                configured-result
                                :runtime-infrastructure
                                :px-runtime-evaluator-config-failed)})

                       (not= :ok (:status run-fn-result))
                       (runtime-failure :runtime-infrastructure
                                        :px-runtime-runner-load-failed)

                       (not= :ok (:status mirror-fn-result))
                       (runtime-failure :runtime-infrastructure
                                        :px-runtime-mirror-runner-load-failed)

                       (not= :closure (:kind run-fn))
                       (runtime-failure :runtime-infrastructure
                                        :px-runtime-runner-missing)

                       (not= :closure (:kind mirror-fn))
                       (runtime-failure :runtime-infrastructure
                                        :px-runtime-mirror-runner-missing)

                       :else
                       {:status :ok
                        :reason :px-runtime-source-runner-ok
                        :run-fn run-fn
                        :mirror-fn mirror-fn
                        :runtime-value (:value runtime-result)
                        :evaluator-value (:value evaluator-result)
                        :configured-value (:value configured-result)}))]
          (when (= :ok (:status runner))
            (reset! source-runner-cache runner))
          runner)))))

(defn runtime-source-execution
  ([source]
   (runtime-source-execution source {}))
  ([source import-modules]
   (let [source-str (str source)
         source-hash (hash/sha256 source-str)
         runner (runtime-source-runner)
         import-modules (if (seq import-modules) import-modules {})
         import-resolver (when (seq import-modules)
                          (in-memory-import-resolver import-modules))
         run-source-fn (fn [call-fn source-str]
                        (if (nil? import-resolver)
                          (apply-runtime-source call-fn source-str)
                          (binding [evaluator/*import-modules* import-modules
                                    evaluator/*import-context* []
                                    evaluator/*import-resolver* import-resolver]
                            (apply-runtime-source call-fn source-str))))]
     (if (not= :ok (:status runner))
       (merge {:kind :px-runtime-source-execution
               :source-hash source-hash}
              (public-runner-row runner)
              (normalize-runtime-execution-result
               runner
               :runtime-infrastructure
               :px-runtime-runner-failed))
       (let [raw-run-result (run-source-fn (:run-fn runner) source-str)
             run-result (if (= :ok (:status raw-run-result))
                          (force-runtime-value (:value raw-run-result)
                                               :run-result)
                          raw-run-result)
             raw-mirror-result (when (= :ok (:status run-result))
                                 (run-source-fn (:mirror-fn runner) source-str))
             mirror-result (if (= :ok (:status raw-mirror-result))
                            (force-runtime-value (:value raw-mirror-result)
                                                 :mirror-result)
                            raw-mirror-result)
             error-value? (and (= :ok (:status run-result))
                               (map? (:value run-result))
                               (= "error" (get (:value run-result) "status")))]
         (merge {:kind :px-runtime-source-execution
                 :source-hash source-hash
                 :runtime-artifact (:runtime-artifact runner)
                 :evaluator-artifact (:evaluator-artifact runner)
                 :evaluated-artifact-count (:evaluated-artifact-count runner)}
                (cond
                  (not= :ok (:status run-result))
                  (normalize-runtime-execution-result
                   run-result :eval :px-runtime-run-failed)

                  (not= :ok (:status mirror-result))
                  (runtime-failure :evidence
                                   :px-runtime-mirror-run-failed)

                  (not (and (map? (:value mirror-result))
                            (= "pnixc-pnix.eval.run-mirror.v0"
                               (get (:value mirror-result) "mirror_schema"))))
                  (runtime-failure :evidence
                                   :px-runtime-mirror-receipt-invalid)

                  error-value?
                  (merge
                   (runtime-failure (canonical-runtime-error-phase
                                     (:value run-result))
                                    (canonical-runtime-error-class
                                     (:value run-result)))
                   {:value (:value run-result)
                    :pnix-mirror-receipt (:value mirror-result)})

                  (not= "ok" (get (:value mirror-result) "status"))
                  (runtime-failure :evidence
                                   :px-runtime-mirror-status-not-ok)

                  (not= (:value run-result)
                        (get (:value mirror-result) "value"))
                  (runtime-failure :evidence
                                   :px-runtime-mirror-value-mismatch)

                  :else
                   {:status :ok
                   :reason :px-runtime-source-execution-ok
                   :value (:value run-result)
                   :pnix-mirror-receipt (:value mirror-result)})))))))
(defn runtime-run-plan
  "Build a human-trackable plan for the internal .px runtime entry. The plan is
  ready only after the .px evaluator parses imports and completes bootstrap."
  ([]
   (runtime-run-plan (select-runtime-artifact)))
  ([entry]
   (if-not entry
     {:kind :px-runtime-run-plan
      :status :failed
      :reason :px-runtime-artifact-missing
      :boundary (runtime-boundary)
      :entry-parse (runtime-entry-parse nil)
      :bootstrap (runtime-bootstrap nil)
      :resource-root runtime-resource-root
      :container-path (.getCanonicalPath (runtime-container-root))
      :entry nil
      :artifacts []
      :edges []
      :missing-imports []}
     (let [index (artifact-index (runtime-artifacts-under (:root entry)))
           {:keys [artifacts edges]}
           (loop [queue [entry]
                  seen #{}
                  artifacts []
                  edges []]
             (if-let [artifact (first queue)]
               (let [id [(:root artifact) (:relative-path artifact)]
                     rest-queue (subvec (vec queue) 1)]
                 (if (seen id)
                   (recur rest-queue seen artifacts edges)
                   (let [artifact-edges (import-edges index artifact)
                         next-artifacts (->> artifact-edges
                                             (filter #(= :resolved (:status %)))
                                             (map #(get index [(:to-root %) (:to %)]))
                                             (remove nil?)
                                             vec)]
                     (recur (into rest-queue next-artifacts)
                            (conj seen id)
                            (conj artifacts
                                  (select-keys artifact
                                               [:root :relative-path :hash]))
                            (into edges artifact-edges)))))
               {:artifacts artifacts
                :edges edges}))
           missing (vec (filter #(= :missing (:status %)) edges))
           import-graph (import-graph-analysis edges)
           entry-parse (runtime-entry-parse entry)
           bootstrap (runtime-bootstrap entry)]
       {:kind :px-runtime-run-plan
        :status (if (or (seq missing)
                        (pos? (:cycle-count import-graph))
                        (not= :ok (:status entry-parse))
                        (not= :ok (:status bootstrap)))
                  :failed
                  :ready)
        :reason (cond
                  (seq missing) :px-runtime-imports-missing
                  (pos? (:cycle-count import-graph)) :px-runtime-import-cycle
                  (not= :ok (:status entry-parse)) :px-runtime-entry-parse-failed
                  (not= :ok (:status bootstrap)) :px-runtime-bootstrap-failed
                  :else :px-runtime-run-plan-ready)
        :boundary (runtime-boundary)
        :entry-parse entry-parse
        :bootstrap bootstrap
        :resource-root runtime-resource-root
        :container-path (.getCanonicalPath (runtime-container-root))
        :entry (select-keys entry [:root :relative-path :hash])
        :artifact-count (count artifacts)
        :edge-count (count edges)
        :import-graph import-graph
        :artifacts artifacts
        :edges edges
        :missing-imports missing}))))
