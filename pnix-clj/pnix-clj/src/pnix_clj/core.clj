(ns pnix-clj.core
  (:require [pnix-clj.clj-meta-executor :as host-executor]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.mirror :as mirror]
            [pnix-clj.parser :as parser]
            [pnix-clj.receipt :as receipt]))

(def lane-classification
  {:lane :core
   :scope :public-runtime-orchestration
   :role :parse-eval-lower-compile-run-source-spine
   :product-runtime :allowed
   :semantic-authority :coordinates-core-lanes
   :mutation :forbidden
   :admission :none-on-basic-path
   :determinism :required-upstream
   :allowed-output :runtime-result
   :verification-api 'verify-source})

(def ^:private semantic-failure-phases
  #{:parse :eval :builtin :import :effect :effect-contract :interop
    :interop-contract :capability :host-compile})

(def ^:private legacy-reason->error-class
  {:unbound-var :unknown-variable
   :missing-attr :attribute-missing
   :call-target-not-callable :not-callable
   :infinite-recursion :cycle-detected})

(defn- normalize-runtime-result
  "Seal legacy lane markers at the product boundary. Structured semantic
  errors are Failed; transition-budget exhaustion is Suspended. Unstructured
  owner/evidence admission values are intentionally outside this conversion."
  [result]
  (let [status (:status result)
        reason (:reason result)
        error (:error result)
        phase (:phase error)
        error-class (let [candidate (or (:class error) (:reason error) reason)]
                      (when (keyword? candidate)
                        (get legacy-reason->error-class candidate candidate)))]
    (cond
      (and (= :held status) (= :fuel-exhausted reason))
      (-> result
          (assoc :status :suspended
                 :reason :resource-budget-exhausted
                 :resource-reason :transition-budget-exhausted)
          (assoc-in [:error :phase] :eval)
          (assoc-in [:error :class] :resource-budget-exhausted))

      (and (contains? #{:held :error} status)
           (contains? semantic-failure-phases phase))
      (cond-> (assoc result :status :failed)
        error-class (assoc-in [:error :class] error-class))

      :else result)))

(defn parse-source
  [source]
  (normalize-runtime-result (parser/parse-source source)))

(def ^:private evaluator-stack-bytes
  ;; 1GB. The lane thread exists to give evaluation a
  ;; FIXED stack, and it also resets depth for each `import`ed module — but at
  ;; 64MB it was strictly smaller than the outer deep-evaluation stack that
  ;; wraps parse+eval, so the deep stack was thrown away the moment evaluation
  ;; began. It overflowed on deeply-nested imported modules
  ;; (stdlib/lib/program.schema.px) because `import` PARSES the module while
  ;; the outer eval is already deep: peak = eval-depth + parse-depth, where
  ;; either alone fits. Virtual memory, committed only on use.
  (* 1024 1024 1024))

;; True while we are already running on the big deep-eval stack; the
;; evaluator lane then runs INLINE instead of hopping to a smaller thread.
(def ^:private ^:dynamic *on-deep-stack?* false)

(defn- eval-ast-lane
  "Run the recursive semantic evaluator on a fixed stack so corpus behavior does
  not depend on the Clojure launcher default thread stack. `base-env` overrides
  the default global env (used by scopedImport to add its scope on top)."
  ([ast]
   (eval-ast-lane ast nil))
  ([ast base-env]
   (let [run #(try
                (if base-env
                  (evaluator/eval-ast ast base-env)
                  (evaluator/eval-ast ast))
                (catch Throwable _
                  {:status :failed
                   :reason :evaluator-lane-threw
                   :error {:phase :eval
                           :class :host-evaluator-exception}}))]
     (if *on-deep-stack?*
       (run)
       (let [result (promise)
             runner (bound-fn [] (deliver result (run)))
             thread (Thread. nil
                             ^Runnable runner
                             "pnix-clj-evaluator-lane"
                             (long evaluator-stack-bytes))]
         (.start thread)
         @result)))))

(defn- eval-ast-lane-whnf
  "Evaluator lane that does NOT deep-realize the result (see
  `evaluator/eval-ast-whnf`). Used by `import`: Nix's import is lazy, so
  forcing an imported module's whole value diverges on the mutually-recursive
  schema modules Nix imports fine."
  ([ast] (eval-ast-lane-whnf ast nil))
  ([ast base-env]
   (let [run #(try
                (if base-env
                  (evaluator/eval-ast-whnf ast base-env)
                  (evaluator/eval-ast-whnf ast))
                (catch Throwable _
                  {:status :failed
                   :reason :evaluator-lane-threw
                   :error {:phase :eval
                           :class :host-evaluator-exception}}))]
     (if *on-deep-stack?*
       (run)
       (let [result (promise)
             runner (bound-fn [] (deliver result (run)))
             thread (Thread. nil
                             ^Runnable runner
                             "pnix-clj-evaluator-lane"
                             (long evaluator-stack-bytes))]
         (.start thread)
         @result)))))

(defn- in-memory-import-resolver
  "Resolve `import <target>` against pnix-clj.evaluator/*import-modules*, an
  in-memory map of target-string -> pnix source. No filesystem access: this is
  the pnix module-loader seam exercised in-process. Import cycles are held
  (:import-cycle), unknown targets are held (:import-module-not-found); a found
  module is parsed and evaluated on a fresh evaluator lane with its target
  pushed onto the import context so nested cycles are detected. `scope` (non-nil
  only for scopedImport) is a forced attrset map added on top of the global env
  for the imported module — base globals stay available, scope keys shadow."
  [context target scope]
  (let [structured? (map? context)
        chain (vec (if structured? (:chain context) context))
        origin (if structured? (:origin context) (last chain))
        target (evaluator/contextual-import-target
                (if origin [origin] []) target)]
    (cond
      (some #(= target %) chain)
      {:status :failed :reason :import-cycle
       :error {:phase :resolution
               :class :import-cycle
               :evidence {:target target :chain (conj chain target)}}
       :target target}

      (contains? evaluator/*import-modules* target)
      (let [{:keys [status ast] :as parsed}
            (parse-source (get evaluator/*import-modules* target))]
        (if (= :ok status)
          ;; scope does NOT propagate through nested plain `import`, so the
          ;; module body evaluates with the base resolver (scope=nil) already
          ;; in force via the dynamic binding; only THIS module's global env is
          ;; augmented.
          (let [module-context (conj chain target)
                module-env (assoc (if scope
                                    (merge evaluator/default-env scope)
                                    evaluator/default-env)
                                  evaluator/import-context-key
                                  target)]
            (binding [evaluator/*import-context* module-context
                      evaluator/*import-origin* target]
              (eval-ast-lane-whnf ast module-env)))
          parsed))

      :else
      {:status :failed :reason :import-module-not-found
       :error {:phase :resolution
               :class :import-module-not-found
               :evidence {:target target}}
       :target target})))

(defn- filesystem-import-resolver
  "Resolve `import <target>` by reading `.px` source from disk. Same shape as
  in-memory-import-resolver (cycle detection, module-env/scope wiring) but the
  module store is the real filesystem instead of a supplied map, keyed on
  canonical absolute paths so the same file imported under different
  relative spellings still shares one cycle-chain entry. Used by eval-file,
  never bound by bare eval-source (which stays no-filesystem-access by
  default -- see *import-modules*)."
  [context target scope]
  (let [structured? (map? context)
        chain (vec (if structured? (:chain context) context))
        origin (if structured? (:origin context) (last chain))
        resolved (evaluator/contextual-import-target
                  (if origin [origin] []) target)
        file (java.io.File. (str resolved))
        canonical (try (.getCanonicalPath file)
                       (catch java.io.IOException _ (.getAbsolutePath file)))]
    (cond
      (some #(= canonical %) chain)
      {:status :failed :reason :import-cycle
       :error {:phase :resolution
               :class :import-cycle
               :evidence {:target canonical :chain (conj chain canonical)}}
       :target canonical}

      (not (.isFile file))
      {:status :failed :reason :import-module-not-found
       :error {:phase :resolution
               :class :import-module-not-found
               :evidence {:target canonical}}
       :target canonical}

      :else
      (let [{:keys [status ast] :as parsed} (parse-source (slurp file))]
        (if (= :ok status)
          (let [module-context (conj chain canonical)
                module-env (assoc (if scope
                                    (merge evaluator/default-env scope)
                                    evaluator/default-env)
                                  evaluator/import-context-key
                                  canonical)]
            (binding [evaluator/*import-context* module-context
                      evaluator/*import-origin* canonical]
              (eval-ast-lane-whnf ast module-env)))
          parsed)))))

(def ^:private deep-stack-bytes
  "Stack size for the dedicated evaluation thread. The JVM default (~512KB-1MB)
  overflows around 1k-deep nesting where real Nix handles 100k+ (oracle-probed:
  nested parens/lists/lets to 100k all evaluate in nix-instantiate) -- the
  pnix ⊇ nix contract needs that headroom (oracle: 10k nested lets cost
  ~50KB of evaluator frames per level -> 512MB was not enough). Virtual
  memory, committed only on use."
  (* 1024 1024 1024))

(def ^:private clj-meta-stack-bytes
  ;; Compiled Clojure is a contrast lane, not the owner of recursive PNIX
  ;; semantics. Keep enough stack for ordinary generated forms while bounding
  ;; a compiled recursive cycle to 64MB instead of letting it consume the 1GB
  ;; source-evaluation stack.
  (* 64 1024 1024))


(defn- call-on-deep-stack
  "Run f on a dedicated big-stack thread (dynamic bindings conveyed via
  bound-fn), so deep pnix nesting parses/evaluates where real Nix does.
  A StackOverflowError beyond even that stack becomes a structured eval
  error, never `Held` and never a raw JVM crash.
  Re-entrant calls run inline: one thread hop per outermost entry."
  [f]
  (if *on-deep-stack?*
    (f)
    (let [ret (atom nil)
          body (bound-fn []
                 (reset! ret
                         (try {:value (binding [*on-deep-stack?* true] (f))}
                              (catch StackOverflowError _ {:soe true})
                              (catch Throwable t {:thrown t}))))
          t (Thread. nil ^Runnable body "pnix-deep-eval" deep-stack-bytes)]
      (.start t)
      (.join t)
      (let [{:keys [value soe thrown]} @ret]
        (cond
          soe {:status :error
               :reason :stack-overflow
               :error {:phase :eval
                       :class :stack-overflow
                       :message "stack overflow: recursion beyond the deep-eval stack (real Nix reports a stack overflow here too)"}}
          thrown (throw thrown)
          :else value)))))

(defn- call-on-clj-meta-stack
  "Run compiled clj-meta forms on a bounded stack distinct from the deep
  parser/evaluator stack. A compiled recursive cycle is contrast-lane evidence,
  not permission to consume the common source lane's native stack."
  [f]
  (let [ret (atom nil)
        body (bound-fn []
               (reset! ret
                       (try (f)
                            (catch StackOverflowError _
                              {:status :error
                               :reason :clj-meta-stack-overflow
                               :error {:phase :eval
                                       :class :stack-overflow}})
                            (catch Throwable _
                              {:status :failed
                               :reason :clj-meta-lane-threw
                               :error {:phase :eval
                                       :class :host-compiled-lane-exception}}))))
        t (Thread. nil ^Runnable body "pnix-clj-meta-eval" clj-meta-stack-bytes)]
    (.start t)
    (.join t)
    @ret))

(defn eval-source
  [source]
  (call-on-deep-stack
   (fn []
     (let [{:keys [status ast] :as parsed} (parse-source source)]
       (if (= :ok status)
         (normalize-runtime-result
          (binding [evaluator/*source-text* (str source)
                    evaluator/*source-file* (or evaluator/*import-origin*
                                                "<pnix-px>")]
            (assoc (if (seq evaluator/*import-modules*)
                     (binding [evaluator/*import-resolver* in-memory-import-resolver]
                       (if (seq evaluator/*import-context*)
                         (eval-ast-lane
                          ast
                          (assoc evaluator/default-env
                                 evaluator/import-context-key
                                 (last evaluator/*import-context*)))
                         (eval-ast-lane ast)))
                     (eval-ast-lane ast))
                   :parse-result parsed)))
         parsed)))))

(defn eval-file
  "Host-language import of a `.px` file: reads and evaluates it with real
  filesystem import resolution wired in, so `import <target>` statements
  inside the file resolve relative to the file's own directory (matching
  eval-source's *import-modules* in-memory lane, but reading from disk).
  Host-bound (JVM/clj); not a portable multi-host bytecode package."
  [path]
  (let [canonical (.getCanonicalPath (java.io.File. (str path)))]
    (binding [evaluator/*import-resolver* filesystem-import-resolver
              evaluator/*import-context* [canonical]
              evaluator/*import-origin* canonical]
      (eval-source (slurp canonical)))))

(defn eval-source-with-imports
  "Like eval-source but resolves `import <target>` against the supplied
  in-memory module map (target-string -> pnix source). Pure: no filesystem
  access. Returns the same result shape as eval-source."
  [source modules]
  (binding [evaluator/*import-modules* modules]
    (eval-source source)))

(defn eval-source-strict-audit
  "Evaluate `source` with identical (lenient) behavior to eval-source, while
  collecting strictness-audit evidence: operations that WOULD fail under strict
  Nix typing (a non-boolean if/assert condition or ! operand, and + coercing a
  string with a non-string). Returns
  {:result <eval-source result> :strict-violations [event ...]}. This is
  audit-only: it never changes the evaluation result."
  ([source]
   (eval-source-strict-audit source nil))
  ([source import-modules]
   (let [audit (atom [])
         result (binding [evaluator/*strict-audit* audit]
                  (if (seq import-modules)
                    (binding [evaluator/*import-modules* import-modules]
                      (eval-source source))
                    (eval-source source)))]
     {:result result
      :strict-violations @audit})))

(defn eval-source-strict
  "Evaluate `source` with strict Nix typing explicitly bound. Since R2 Phase D
  (2026-07-07) strict IS the default semantics — the former lenient behavior
  was a Clojure host leak, removed — so this is now an explicit alias kept for
  callers that want the binding spelled out."
  ([source]
   (eval-source-strict source nil))
  ([source import-modules]
   (binding [evaluator/*strict* true]
     (if (seq import-modules)
       (binding [evaluator/*import-modules* import-modules]
         (eval-source source))
       (eval-source source)))))

(defn lower-source
  [source]
  (let [{:keys [status ast] :as parsed} (parse-source source)]
    (if (= :ok status)
      (normalize-runtime-result
       (assoc (lowering/lower-ast ast) :parse-result parsed))
      parsed)))

(defn compile-source
  "Compatibility name for parse/lower/direct-host execution.

  Basic execution is not conditioned on proof or determinism receipts. Call an
  explicit proof namespace when compile evidence is required."
  [source]
  (let [source-str (str source)
        source-hash (hash/sha256 source-str)
        parse-result (parse-source source-str)
        base {:kind :pnix-clj.compile-source
              :schema :pnix-clj.compile-source.v0
              :source source-str
              :source-hash source-hash
              :parse-result parse-result}]
    (if (not= :ok (:status parse-result))
      (assoc base
             :status (:status parse-result)
             :reason (:reason parse-result)
             :ast-hash nil
             :lowering-result nil
             :lowered-form nil
             :lowered-form-hash nil
             :clj-meta-result nil
             :compile-receipt nil)
      (let [ast (:ast parse-result)
            ast-hash (hash/data-hash ast)
            lowering-result (lowering/lower-ast ast)]
        (if (not= :ok (:status lowering-result))
          (assoc base
                 :status (:status lowering-result)
                 :reason (:reason lowering-result)
                 :error (:error lowering-result)
                 :ast-hash ast-hash
                 :lowering-result lowering-result
                 :lowered-form nil
                 :lowered-form-hash nil
                 :clj-meta-result nil
                 :compile-receipt nil)
          (let [clj-meta-result (host-executor/eval-lowered (:form lowering-result))
                ok? (= :ok (:status clj-meta-result))]
            (assoc base
                   :status (if ok? :ok (:status clj-meta-result))
                   :reason (if ok?
                             :pnix-source-host-execution-ready
                             (:reason clj-meta-result))
                   :ast-hash ast-hash
                   :lowering-result lowering-result
                   :lowered-form (:form lowering-result)
                   :lowered-form-hash (:form-hash lowering-result)
                   :bytecode-hash nil
                   :clj-meta-result (select-keys clj-meta-result
                                                 [:status :reason :error :value :mode
                                                  :diagnostics
                                                  :execution-api])
                   :compile-receipt nil)))))))

(defn run-source
  "Execute PNIX source through the basic semantic path. Meta-circular host
  mechanisms remain available, but proof receipts and mirror verdicts do not
  decide this result. Use verify-source for the explicit multi-lane report."
  [input]
  (let [{:keys [source-id source import-modules] :as source-row}
        (if (map? input)
          input
          {:source-id :inline :source input})
        source-str (str source)
        source-meta (dissoc source-row :source :oracle-result)
        result (if (seq import-modules)
                 (eval-source-with-imports source-str import-modules)
                 (eval-source source-str))]
    (merge {:source-id source-id
            :source-meta source-meta
            :source source-str
            :source-hash (hash/sha256 source-str)}
           result)))

(defn verify-source
  "Run the explicit cross-lane verification report. This API may produce
  receipts and mirror evidence, but it is not the basic execution authority."
  [input]
  (call-on-deep-stack
   (fn []
  (let [{:keys [source-id source oracle-result import-modules] :as source-row}
        (if (map? input)
          input
          {:source-id :inline :source input})
        source-meta (dissoc source-row :source :oracle-result)
        source-str (str source)
        source-hash (hash/sha256 source-str)
        parse-result (call-on-deep-stack #(parse-source source-str))]
    (if (not= :ok (:status parse-result))
      (let [lane-summary (receipt/lane-summary {:parse-result parse-result})
            v (receipt/verdict {:parse-result parse-result})]
        (merge {:source-id source-id
                :source-meta source-meta
                :source source-str
                :source-hash source-hash
                :ast-hash nil
                :eval-result nil
                :lowered-form-hash nil
                :clojure-mirror nil
                :clj-meta-result nil
                :mirror-run nil
                :px-runtime-hash nil
                :pnix-mirror nil
                :cross-mirror-verdict nil
                :oracle-result oracle-result
                :bytecode-hash nil
                :lane-summary lane-summary
                :receipts [parse-result]}
               v))
      (let [ast (:ast parse-result)
            ast-hash (call-on-deep-stack #(hash/data-hash ast))
            eval-result (normalize-runtime-result
                         (if (seq import-modules)
                           (binding [evaluator/*import-modules* import-modules
                                     evaluator/*import-resolver* in-memory-import-resolver]
                             (eval-ast-lane ast))
                           (eval-ast-lane ast)))
            lowering-result (normalize-runtime-result
                             (call-on-deep-stack
                              #(if (seq import-modules)
                                 (binding [lowering/*import-modules* import-modules]
                                   (lowering/lower-ast ast))
                                 (lowering/lower-ast ast))))
            clj-meta-result (if (= :ok (:status lowering-result))
                              (call-on-clj-meta-stack
                               #(host-executor/eval-lowered (:form lowering-result)))
                              {:status :failed
                               :reason :lowering-not-available
                               :error {:phase :host-compile
                                       :class :lowering-not-available}})
            mirror-run (mirror/run-mirror {:source source-str
                                           :source-hash source-hash
                                           :ast-hash ast-hash
                                           :eval-result eval-result
                                           :lowering-result lowering-result
                                           :clj-meta-result clj-meta-result
                                           :import-modules import-modules})
            stage15-control (:stage15-control mirror-run)
            runtime-artifact (:runtime-artifact mirror-run)
            clojure-mirror (:clojure-mirror mirror-run)
            px-runtime (:px-runtime mirror-run)
            pnix-mirror (:pnix-mirror mirror-run)
            cross-mirror-verdict (:cross-mirror-verdict mirror-run)
            verdict (receipt/verdict {:parse-result parse-result
                                      :eval-result eval-result
                                      :lowering-result lowering-result
                                      :clj-meta-result clj-meta-result
                                      :oracle-result oracle-result
                                      :px-runtime px-runtime
                                      :pnix-mirror pnix-mirror})
            lane-summary (receipt/lane-summary {:parse-result parse-result
                                                :eval-result eval-result
                                                :lowering-result lowering-result
                                                :clj-meta-result clj-meta-result
                                                :clojure-mirror clojure-mirror
                                                :stage15-control stage15-control
                                                :px-runtime px-runtime
                                                :pnix-mirror pnix-mirror})]
        (merge {:source-id source-id
                :source-meta source-meta
                :source source-str
                :source-hash source-hash
                :ast ast
                :ast-hash ast-hash
                :eval-result eval-result
                :lowering-result lowering-result
                :lowered-form (:form lowering-result)
                :lowered-form-hash (:form-hash lowering-result)
                :clojure-mirror clojure-mirror
                :clj-meta-result clj-meta-result
                :stage15-control stage15-control
                :mirror-run mirror-run
                :px-runtime-hash (:hash runtime-artifact)
                :px-runtime px-runtime
                :pnix-mirror pnix-mirror
                :cross-mirror-verdict cross-mirror-verdict
                :oracle-result oracle-result
                :bytecode-hash nil
                :lane-summary lane-summary
                :receipts (cond-> [parse-result
                                    eval-result
                                    lowering-result]
                             oracle-result
                             (conj (assoc oracle-result :kind :oracle))

                             true
                             (conj clojure-mirror
                                   px-runtime
                                   pnix-mirror
                                   cross-mirror-verdict))}
               verdict)))))))

(defn report
  [sources]
  (let [receipts (mapv (fn [source]
                         (if (map? source)
                           (verify-source source)
                           (verify-source {:source-id :inline :source source})))
                       sources)]
    (assoc (receipt/summarize receipts) :receipts receipts)))
