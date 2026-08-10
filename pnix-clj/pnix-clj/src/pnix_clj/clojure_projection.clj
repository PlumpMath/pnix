(ns pnix-clj.clojure-projection
  "pnix -> Clojure projection report over a fixture corpus (the lowering lane's surface, pinned per case)."
  (:require [clojure.java.io :as io]
            [clojure.walk :as walk]
            [pnix.clj-meta.host-reflection :as host-reflection]
            [pnix-clj.clojure-projection.host :as projection-host]
            [pnix-clj.error :as err]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.px-runtime :as px-runtime]))

(def lane-classification
  {:lane :proof-only
   :scope :clojure-projection-fixture-corpus
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :projection-authority :evidence-only
   :allowed-output :clojure-projection-report})

(def cases-resource
  "pnix_clj/clojure_projection/cases.edn")

(def projection-root
  "pnixc-pnix")

(def projection-relative-path
  "clojure/projection.px")

(def ^:private max-macroexpand-steps
  128)

(defn- macroexpand-case-source
  [source]
  (if (map? source)
    (pr-str source)
    source))

(defn- macroexpand-form-source
  [source]
  (if (map? source)
    (:form source)
    source))

(defn- macroexpand-setup-sources
  [source]
  (let [setup (when (map? source)
                (:setup source))]
    (cond
      (nil? setup) []
      (sequential? setup) (vec setup)
      :else [setup])))

(defn- macroexpand-features
  [source]
  (if (map? source)
    (vec (:features source))
    []))

(defn fixture-set
  []
  (if-let [resource (io/resource cases-resource)]
    (read-string (slurp resource))
    (throw (ex-info "clojure-projection fixtures missing"
                    {:resource cases-resource}))))

(defn cases
  []
  (let [{:keys [lineage cases]} (fixture-set)]
    (mapv (fn [{:keys [name reader-source host-form-source
                       control-flow-source control-flow-trace-source
                       macroexpand-source dynamic-binding-source
                       java-interop-source reflection-source
                       classloader-source namespace-resolution-source
                       host-object-construction-source
                       polymorphism-source metadata-source
                       state-effect-source lazy-evaluation-source
                       concurrency-source coordination-source]
                :as case}]
            (assoc case
                   :source-id (keyword "clojure-projection" name)
                   :batch :clojure-projection/host-term-shape-slice
                   :source-kind (cond
                                  reader-source :reader-source
                                  host-form-source :host-form-source
                                  control-flow-source :control-flow-source
                                  control-flow-trace-source :control-flow-trace-source
                                  macroexpand-source :macroexpand-source
                                  dynamic-binding-source :dynamic-binding-source
                                  java-interop-source :java-interop-source
                                  reflection-source :reflection-source
                                  classloader-source :classloader-source
                                  namespace-resolution-source :namespace-resolution-source
                                  host-object-construction-source :host-object-construction-source
                                  polymorphism-source :polymorphism-source
                                  metadata-source :metadata-source
                                  state-effect-source :state-effect-source
                                  lazy-evaluation-source :lazy-evaluation-source
                                  concurrency-source :concurrency-source
                                  coordination-source :coordination-source
                                  :else :unknown-source)
                   :source (or reader-source
                               host-form-source
                               control-flow-source
                               (:form control-flow-trace-source)
                               (some-> macroexpand-source
                                       macroexpand-case-source)
                               dynamic-binding-source
                               (:form java-interop-source)
                               (:form reflection-source)
                               (:form classloader-source)
                               (:form namespace-resolution-source)
                               (:form host-object-construction-source)
                               (:form polymorphism-source)
                               (:form metadata-source)
                               (:form state-effect-source)
                               (:form lazy-evaluation-source)
                               (:form concurrency-source)
                               (:form coordination-source)
                               (pr-str (dissoc case :name)))
                   :source-origin (:source lineage)
                   :source-lineage lineage
                   :fixture-hash (hash/sha256 (pr-str case))))
          cases)))

(declare project-reader-value)

(defn- scalar-term
  [kind value]
  {"tag" "Scalar"
   "host" "clojure"
   "kind" kind
   "value" value})

(defn- named-term
  [tag value]
  {"tag" tag
   "host" "clojure"
   "ns" (namespace value)
   "name" (name value)})

(defn- stable-items
  [items]
  (sort-by pr-str items))

(defn- namespace-name
  [ns-value]
  (some-> ns-value ns-name str))

(defn project-var-value
  [var-value]
  (let [snapshot-result (host-reflection/snapshot :var var-value)]
    (or (:snapshot snapshot-result)
        {"tag" "Var"
         "host" "clojure"
         "ns" (or (some-> var-value meta :ns namespace-name)
                  (some-> var-value meta :name namespace)
                  "")
         "name" (str var-value)
         "dynamic" false})))

(defn project-namespace-value
  [ns-value]
  (let [snapshot-result (host-reflection/snapshot :namespace ns-value)]
    (or (:snapshot snapshot-result)
        {"tag" "Namespace"
         "host" "clojure"
         "name" (namespace-name ns-value)})))

(defn project-throwable-value
  [throwable]
  (let [snapshot-result (host-reflection/snapshot :throwable throwable)]
    (if (= :ok (:status snapshot-result))
      (let [snapshot (:snapshot snapshot-result)]
        (assoc snapshot
               "data" (some-> (ex-data throwable) project-reader-value)
               "cause_class" (some-> throwable .getCause class .getName)))
      {"tag" "Exception"
       "host" "clojure"
       "class" (.getName (class throwable))
       "message" (.getMessage throwable)
       "data" (some-> (ex-data throwable) project-reader-value)
       "cause_class" (some-> throwable .getCause class .getName)})))

(defn- class-term
  [class-value]
  (let [snapshot-result (host-reflection/snapshot :class class-value)]
    (or (:snapshot snapshot-result)
        {"tag" "JavaClass"
         "host" "clojure"
         "name" (.getName class-value)
         "simple_name" (.getSimpleName class-value)
         "package" (some-> class-value .getPackage .getName)
         "primitive" (.isPrimitive class-value)
         "array" (.isArray class-value)
         "interface" (.isInterface class-value)})))

(defn- java-object-term
  [value]
  (let [snapshot-result (host-reflection/snapshot :java-object value)]
    (or (:snapshot snapshot-result)
        {"tag" "JavaObject"
         "host" "clojure"
         "class" (.getName (class value))
         "string" (try
                    (str value)
                    (catch Throwable _
                      nil))
         "projection_note" "jvm-object-envelope"})))

(defn control-flow-receipt-term
  [{:keys [source form-kind source-hash status value exception effects]}]
  {"tag" "ControlFlowReceipt"
   "host" "clojure"
   "status" status
   "form_kind" form-kind
   "source_hash" source-hash
   "source" source
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))
   "effects" (mapv project-reader-value (or effects []))})

(defn- normalize-auto-gensyms
  [form]
  (walk/postwalk
   (fn [value]
     (if (symbol? value)
       (if-let [[_ prefix] (re-matches #"(.+)__\d+__auto__$" (name value))]
         (symbol (namespace value) (str prefix "__AUTO__"))
         value)
       value))
   form))

(defn- macroexpand-step
  [path index before after]
  (let [before* (normalize-auto-gensyms before)
        after* (normalize-auto-gensyms after)]
    {:path path
     :phase "macroexpand-1"
     :index index
     :before before*
     :after after*
     :before-hash (hash/data-hash before*)
     :after-hash (hash/data-hash after*)}))

(defn- expand-one-form
  [path form]
  (loop [current form
         index 0
         steps []]
    (when (>= index max-macroexpand-steps)
      (throw (ex-info "macroexpand trace exceeded step limit"
                      {:path path
                       :limit max-macroexpand-steps
                       :form (normalize-auto-gensyms current)})))
    (let [expanded (macroexpand-1 current)]
      (if (= expanded current)
        {:form current
         :steps steps}
        (recur expanded
               (inc index)
               (conj steps
                     (macroexpand-step path index current expanded)))))))

(declare macroexpand-all-trace*)

(defn- rebuild-seq
  [form items]
  (cond
    (list? form) (apply list items)
    (seq? form) (seq items)
    :else items))

(defn- macroexpand-all-trace*
  [path form]
  (let [{expanded :form root-steps :steps} (if (seq? form)
                                            (expand-one-form path form)
                                            {:form form
                                             :steps []})]
    (cond
      (seq? expanded)
      (let [children (map-indexed (fn [idx item]
                                    (macroexpand-all-trace* (conj path idx) item))
                                  expanded)]
        {:form (rebuild-seq expanded (map :form children))
         :steps (into root-steps (mapcat :steps children))})

      (vector? expanded)
      (let [children (map-indexed (fn [idx item]
                                    (macroexpand-all-trace* (conj path idx) item))
                                  expanded)]
        {:form (mapv :form children)
         :steps (into root-steps (mapcat :steps children))})

      (map? expanded)
      (let [entries (map-indexed
                     (fn [idx [k v]]
                       {:key (macroexpand-all-trace* (conj path idx :key) k)
                        :value (macroexpand-all-trace* (conj path idx :value) v)})
                     expanded)]
        {:form (into (empty expanded)
                     (map (fn [{:keys [key value]}]
                            [(:form key) (:form value)]))
                     entries)
         :steps (into root-steps
                      (mapcat (fn [{:keys [key value]}]
                                (concat (:steps key) (:steps value)))
                              entries))})

      (set? expanded)
      (let [children (map-indexed (fn [idx item]
                                    (macroexpand-all-trace* (conj path idx) item))
                                  expanded)]
        {:form (into (empty expanded) (map :form) children)
         :steps (into root-steps (mapcat :steps children))})

      :else
      {:form expanded
       :steps root-steps})))

(defn- macroexpand-all-trace
  [form]
  (let [{:keys [form steps]} (macroexpand-all-trace* [] form)]
    {:final-form (normalize-auto-gensyms form)
     :steps steps}))

(defn- eval-setup-sources!
  [target setup-sources]
  (doseq [source setup-sources]
    (let [form (binding [*read-eval* false
                         *ns* target]
                 (read-string source))]
      (binding [*ns* target]
        (eval form)))))

(defn macroexpand-receipt-term
  [{:keys [source setup-sources features phase source-hash status source-form
           expanded-form final-form steps exception]}]
  {"tag" "MacroexpandReceipt"
   "host" "clojure"
   "status" status
   "phase" phase
   "expansion_policy" "macroexpand-all-step-trace"
   "source_hash" source-hash
   "source" source
   "setup" (mapv str (or setup-sources []))
   "features" (mapv str (or features []))
   "source_term" (when source-form
                   (project-reader-value (normalize-auto-gensyms source-form)))
   "expanded_term" (when expanded-form
                     (project-reader-value (normalize-auto-gensyms expanded-form)))
   "final_term" (when final-form
                  (project-reader-value (normalize-auto-gensyms final-form)))
   "step_count" (count (or steps []))
   "steps" (mapv project-reader-value (or steps []))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn dynamic-binding-receipt-term
  [{:keys [source source-hash status var-value binding-value value exception]}]
  {"tag" "DynamicBindingReceipt"
   "host" "clojure"
   "status" status
   "source_hash" source-hash
   "source" source
   "var" (when var-value
           (project-var-value var-value))
   "binding_value" (project-reader-value binding-value)
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn- class-name
  [value]
  (some-> value class .getName))

(defn java-interop-receipt-term
  [{:keys [source source-hash status op target member args value exception]}]
  {"tag" "JavaInteropReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "target" target
   "member" member
   "source_hash" source-hash
   "source" source
   "args" (mapv project-reader-value args)
   "arg_classes" (mapv class-name args)
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn reflection-receipt-term
  [{:keys [source source-hash status op target-class member args value exception]}]
  {"tag" "ReflectionReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "target_class" target-class
   "member" member
   "source_hash" source-hash
   "source" source
   "args" (mapv project-reader-value args)
   "arg_classes" (mapv class-name args)
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn classloader-receipt-term
  [{:keys [source source-hash status op context args value exception]}]
  {"tag" "ClassloaderReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "context" context
   "source_hash" source-hash
   "source" source
   "args" (mapv project-reader-value args)
   "arg_classes" (mapv class-name args)
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn namespace-resolution-receipt-term
  [{:keys [source source-hash status op ns requested alias resolve-symbol
           value exception]}]
  {"tag" "NamespaceResolutionReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "ns" ns
   "requested" requested
   "alias" alias
   "symbol" resolve-symbol
   "source_hash" source-hash
   "source" source
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn- interface-names
  [value]
  (->> (.getInterfaces (class value))
       (map #(.getName %))
       sort
       vec))

(defn host-object-construction-receipt-term
  [{:keys [source source-hash status construction-kind type-name value exception]}]
  {"tag" "HostObjectConstructionReceipt"
   "host" "clojure"
   "status" status
   "construction_kind" construction-kind
   "type_name" type-name
   "source_hash" source-hash
   "source" source
   "result_class" (when (= "ok" status)
                    (class-name value))
   "interfaces" (if (= "ok" status)
                  (interface-names value)
                  [])
   "record" (boolean (and (= "ok" status)
                          (record? value)))
   "object" (when (= "ok" status)
              (java-object-term value))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception_class" (when exception
                       (class-name exception))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn polymorphism-dispatch-receipt-term
  [{:keys [source source-hash status dispatch-kind dispatch-name dispatch-value
           args value exception]}]
  {"tag" "PolymorphismDispatchReceipt"
   "host" "clojure"
   "status" status
   "dispatch_kind" dispatch-kind
   "dispatch_name" dispatch-name
   "dispatch_value" (some-> dispatch-value str)
   "source_hash" source-hash
   "source" source
   "args" (mapv project-reader-value (or args []))
   "arg_classes" (mapv class-name (or args []))
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn metadata-receipt-term
  [{:keys [source source-hash status op target-kind target metadata value
           exception]}]
  {"tag" "MetadataReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "target_kind" target-kind
   "target_class" (when target
                    (class-name target))
   "source_hash" source-hash
   "source" source
   "target" (when target
              (project-reader-value target))
   "metadata" (when metadata
                (project-reader-value metadata))
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn state-effect-receipt-term
  [{:keys [source source-hash status op target-kind target initial final value
           effects exception]}]
  {"tag" "StateEffectReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "target_kind" target-kind
   "target_class" (when target
                    (class-name target))
   "source_hash" source-hash
   "source" source
   "initial" (project-reader-value initial)
   "final" (project-reader-value final)
   "effects" (mapv project-reader-value (or effects []))
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn lazy-evaluation-receipt-term
  [{:keys [source source-hash status op target-kind target realized-count
           effects value exception]}]
  {"tag" "LazyEvaluationReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "target_kind" target-kind
   "target_class" (when target
                    (class-name target))
   "source_hash" source-hash
   "source" source
   "realized_count" (or realized-count 0)
   "effects" (mapv project-reader-value (or effects []))
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn concurrency-receipt-term
  [{:keys [source source-hash status op target-kind target completed effects
           value exception]}]
  {"tag" "ConcurrencyReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "target_kind" target-kind
   "target_class" (when target
                    (class-name target))
   "source_hash" source-hash
   "source" source
   "completed" (boolean completed)
   "effects" (mapv project-reader-value (or effects []))
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn coordination-receipt-term
  [{:keys [source source-hash status op target-kind target initial final
           completed effects value exception]}]
  {"tag" "CoordinationReceipt"
   "host" "clojure"
   "status" status
   "op" op
   "target_kind" target-kind
   "target_class" (when target
                    (class-name target))
   "source_hash" source-hash
   "source" source
   "initial" (project-reader-value initial)
   "final" (project-reader-value final)
   "completed" (boolean completed)
   "effects" (mapv project-reader-value (or effects []))
   "result_class" (when (= "ok" status)
                    (class-name value))
   "exception_class" (when exception
                       (class-name exception))
   "result" (when (= "ok" status)
              (project-reader-value value))
   "exception" (when exception
                 (project-throwable-value exception))})

(defn- control-flow-source-term
  [source-id source]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source source
    :prefix "pnix-clj.projection-control"
    :effect :host-eval
    :kind :clojure-projection-control-flow}
   (fn [target form]
     (control-flow-receipt-term
      {:source source
       :form-kind "closed-form"
       :source-hash (hash/sha256 source)
       :status "ok"
       :value (binding [*ns* target]
                (eval form))}))
   (fn [t]
     (control-flow-receipt-term
      {:source source
       :form-kind "closed-form"
       :source-hash (hash/sha256 source)
       :status "error"
       :exception t}))))

(defn- control-flow-trace-source-term
  [source-id {:keys [form form-kind]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-control-trace"
    :effect :host-eval
    :kind :clojure-projection-control-trace}
   (fn [target parsed-form]
     (let [value (binding [*ns* target]
                   (eval parsed-form))
           ;; Only unwrap when the value is actually a trace wrapper that
           ;; carries :result (matching the sibling receipt handlers); a plain
           ;; map result must be projected as-is, not turned into nil effects.
           trace-wrapper? (and (map? value) (contains? value :result))
           result-value (if trace-wrapper? (:result value) value)
           effects (if trace-wrapper? (:effects value) [])]
       (control-flow-receipt-term
        {:source form
         :form-kind (or form-kind "closed-form")
         :source-hash (hash/sha256 form)
         :status "ok"
         :value result-value
         :effects effects})))
   (fn [t]
     (control-flow-receipt-term
      {:source form
       :form-kind (or form-kind "closed-form")
       :source-hash (hash/sha256 form)
       :status "error"
       :exception t}))))

(defn- macroexpand-source-term
  [source-id source]
  (let [form-source (macroexpand-form-source source)
        setup-sources (macroexpand-setup-sources source)
        features (macroexpand-features source)
        source-repr (macroexpand-case-source source)]
    (projection-host/host-form-crossing
     {:source-id source-id
      :source form-source
      :prefix "pnix-clj.projection-macroexpand"
      :effect :macroexpand
      :kind :clojure-projection-macroexpand}
     (fn [target form]
       (binding [*ns* target]
         (eval-setup-sources! target setup-sources)
         (let [expanded-form (macroexpand-1 form)
               trace (macroexpand-all-trace form)]
           (macroexpand-receipt-term
            {:source source-repr
             :setup-sources setup-sources
             :features features
             :phase "macroexpand-all-trace"
             :source-hash (hash/sha256 source-repr)
             :status "ok"
             :source-form form
             :expanded-form expanded-form
             :final-form (:final-form trace)
             :steps (:steps trace)}))))
     (fn [t]
       (macroexpand-receipt-term
        {:source source-repr
         :setup-sources setup-sources
         :features features
         :phase "macroexpand-all-trace"
         :source-hash (hash/sha256 source-repr)
         :status "error"
         :source-form nil
         :exception t})))))

(defn- resolve-var-in
  [target var-name]
  (binding [*ns* target]
    (resolve (symbol var-name))))

(defn- dynamic-binding-source-term
  [source-id {:keys [form var binding-value]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-dynamic"
    :effect :dynamic-binding
    :kind :clojure-projection-dynamic-binding}
   (fn [target parsed-form]
     (let [var-value (resolve-var-in target var)]
       (dynamic-binding-receipt-term
        {:source form
         :source-hash (hash/sha256 form)
         :status "ok"
         :var-value var-value
         :binding-value binding-value
         :value (binding [*ns* target]
                  (eval parsed-form))})))
   (fn [t]
     (dynamic-binding-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :var-value nil
       :binding-value binding-value
       :exception t}))))

(defn- java-interop-source-term
  [source-id {:keys [form op target member args]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-java"
    :effect :host-call
    :kind :clojure-projection-java-interop}
   (fn [eval-target parsed-form]
     (java-interop-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "ok"
       :op op
       :target target
       :member member
       :args args
       :value (binding [*ns* eval-target]
                (eval parsed-form))}))
   (fn [t]
     (java-interop-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :target target
       :member member
       :args args
       :exception t}))))

(defn- reflection-source-term
  [source-id {:keys [form op target-class member args]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-reflection"
    :effect :reflection
    :kind :clojure-projection-reflection}
   (fn [eval-target parsed-form]
     (reflection-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "ok"
       :op op
       :target-class target-class
       :member member
       :args args
       :value (binding [*ns* eval-target]
                (eval parsed-form))}))
   (fn [t]
     (reflection-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :target-class target-class
       :member member
       :args args
       :exception t}))))

(defn- classloader-source-term
  [source-id {:keys [form op context args]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-classloader"
    :effect :classloader
    :kind :clojure-projection-classloader}
   (fn [eval-target parsed-form]
     (classloader-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "ok"
       :op op
       :context context
       :args args
       :value (binding [*ns* eval-target]
                (eval parsed-form))}))
   (fn [t]
     (classloader-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :context context
       :args args
       :exception t}))))

(defn- namespace-resolution-source-term
  [source-id {:keys [form op requested alias symbol]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-namespace"
    :effect :resolve-var
    :kind :clojure-projection-namespace-resolution}
   (fn [eval-target parsed-form]
     (namespace-resolution-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "ok"
       :op op
       :ns (str (ns-name eval-target))
       :requested requested
       :alias alias
       :resolve-symbol symbol
       :value (binding [*ns* eval-target]
                (eval parsed-form))}))
   (fn [t]
     (namespace-resolution-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :ns nil
       :requested requested
       :alias alias
       :resolve-symbol symbol
       :exception t}))))

(defn- host-object-construction-source-term
  [source-id {:keys [form construction-kind type-name]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-object"
    :effect :host-call
    :kind :clojure-projection-host-object}
   (fn [eval-target parsed-form]
     (host-object-construction-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "ok"
       :construction-kind construction-kind
       :type-name type-name
       :value (binding [*ns* eval-target]
                (eval parsed-form))}))
   (fn [t]
     (host-object-construction-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :construction-kind construction-kind
       :type-name type-name
       :exception t}))))

(defn- polymorphism-source-term
  [source-id {:keys [form dispatch-kind dispatch-name dispatch-value args]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-polymorphism"
    :effect :host-call
    :kind :clojure-projection-polymorphism}
   (fn [eval-target parsed-form]
     (let [raw-value (binding [*ns* eval-target]
                       (eval parsed-form))
           observed (if (and (map? raw-value) (contains? raw-value :result))
                      raw-value
                      {:result raw-value
                       :args args
                       :dispatch-value dispatch-value})]
       (polymorphism-dispatch-receipt-term
        {:source form
         :source-hash (hash/sha256 form)
         :status "ok"
         :dispatch-kind dispatch-kind
         :dispatch-name dispatch-name
         :dispatch-value (or (:dispatch-value observed) dispatch-value)
         :args (or (:args observed) args)
         :value (:result observed)})))
   (fn [t]
     (polymorphism-dispatch-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :dispatch-kind dispatch-kind
       :dispatch-name dispatch-name
       :dispatch-value dispatch-value
       :args args
       :exception t}))))

(defn- metadata-source-term
  [source-id {:keys [form op target-kind]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-metadata"
    :effect :host-call
    :kind :clojure-projection-metadata}
   (fn [eval-target parsed-form]
     (let [raw-value (binding [*ns* eval-target]
                       (eval parsed-form))
           observed (if (and (map? raw-value) (contains? raw-value :result))
                      raw-value
                      {:result raw-value})
           target (:target observed)
           metadata (if (contains? observed :metadata)
                      (:metadata observed)
                      (some-> target meta))]
       (metadata-receipt-term
        {:source form
         :source-hash (hash/sha256 form)
         :status "ok"
         :op op
         :target-kind target-kind
         :target target
         :metadata metadata
         :value (:result observed)})))
   (fn [t]
     (metadata-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :target-kind target-kind
       :exception t}))))

(defn- state-effect-source-term
  [source-id {:keys [form op target-kind]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-state"
    :effect :global-mutation
    :kind :clojure-projection-state-effect}
   (fn [eval-target parsed-form]
     (let [raw-value (binding [*ns* eval-target]
                       (eval parsed-form))
           observed (if (and (map? raw-value) (contains? raw-value :result))
                      raw-value
                      {:result raw-value})]
       (state-effect-receipt-term
        {:source form
         :source-hash (hash/sha256 form)
         :status "ok"
         :op op
         :target-kind target-kind
         :target (:target observed)
         :initial (:initial observed)
         :final (:final observed)
         :effects (:effects observed)
         :value (:result observed)})))
   (fn [t]
     (state-effect-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :target-kind target-kind
       :exception t}))))

(defn- lazy-evaluation-source-term
  [source-id {:keys [form op target-kind]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-lazy"
    :effect :host-call
    :kind :clojure-projection-lazy-evaluation}
   (fn [eval-target parsed-form]
     (let [raw-value (binding [*ns* eval-target]
                       (eval parsed-form))
           observed (if (and (map? raw-value) (contains? raw-value :result))
                      raw-value
                      {:result raw-value})]
       (lazy-evaluation-receipt-term
        {:source form
         :source-hash (hash/sha256 form)
         :status "ok"
         :op op
         :target-kind target-kind
         :target (:target observed)
         :realized-count (:realized-count observed)
         :effects (:effects observed)
         :value (:result observed)})))
   (fn [t]
     (lazy-evaluation-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :target-kind target-kind
       :exception t}))))

(defn- concurrency-source-term
  [source-id {:keys [form op target-kind]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-concurrency"
    :effect :thread
    :kind :clojure-projection-concurrency}
   (fn [eval-target parsed-form]
     (let [raw-value (binding [*ns* eval-target]
                       (eval parsed-form))
           observed (if (and (map? raw-value) (contains? raw-value :result))
                      raw-value
                      {:result raw-value})]
       (concurrency-receipt-term
        {:source form
         :source-hash (hash/sha256 form)
         :status "ok"
         :op op
         :target-kind target-kind
         :target (:target observed)
         :completed (:completed observed)
         :effects (:effects observed)
         :value (:result observed)})))
   (fn [t]
     (concurrency-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :target-kind target-kind
       :exception t}))))

(defn- coordination-source-term
  [source-id {:keys [form op target-kind]}]
  (projection-host/host-form-crossing
   {:source-id source-id
    :source form
    :prefix "pnix-clj.projection-coordination"
    :effect :thread
    :kind :clojure-projection-coordination}
   (fn [eval-target parsed-form]
     (let [raw-value (binding [*ns* eval-target]
                       (eval parsed-form))
           observed (if (and (map? raw-value) (contains? raw-value :result))
                      raw-value
                      {:result raw-value})]
       (coordination-receipt-term
        {:source form
         :source-hash (hash/sha256 form)
         :status "ok"
         :op op
         :target-kind target-kind
         :target (:target observed)
         :initial (:initial observed)
         :final (:final observed)
         :completed (:completed observed)
         :effects (:effects observed)
         :value (:result observed)})))
   (fn [t]
     (coordination-receipt-term
      {:source form
       :source-hash (hash/sha256 form)
       :status "error"
       :op op
       :target-kind target-kind
       :exception t}))))

(defn project-reader-value
  [value]
  (cond
    (nil? value)
    (scalar-term "nil" nil)

    (or (true? value) (false? value))
    (scalar-term "bool" value)

    (integer? value)
    (scalar-term "int" value)

    (decimal? value)
    (scalar-term "bigdec" (str value))

    (float? value)
    (scalar-term "float" value)

    (string? value)
    (scalar-term "string" value)

    (char? value)
    (scalar-term "char" (str value))

    (ratio? value)
    (scalar-term "ratio" (pr-str value))

    (var? value)
    (project-var-value value)

    (instance? clojure.lang.Namespace value)
    (project-namespace-value value)

    (instance? Throwable value)
    (project-throwable-value value)

    (instance? Class value)
    (class-term value)

    (symbol? value)
    (named-term "Symbol" value)

    (keyword? value)
    (named-term "Keyword" value)

    (list? value)
    {"tag" "List"
     "host" "clojure"
     "form" true
     "items" (mapv project-reader-value value)}

    (vector? value)
    {"tag" "Vector"
     "host" "clojure"
     "items" (mapv project-reader-value value)}

    (sequential? value)
    {"tag" "List"
     "host" "clojure"
     "form" false
     "items" (mapv project-reader-value value)}

    (map? value)
    {"tag" "Map"
     "host" "clojure"
     "pairs" (mapv (fn [[k v]]
                     {"key" (project-reader-value k)
                      "value" (project-reader-value v)})
                   (sort-by (comp pr-str key) value))}

    (set? value)
    {"tag" "Set"
     "host" "clojure"
     "items" (mapv project-reader-value (stable-items value))}

    :else
    (java-object-term value)))

(defn- case-term
  [case]
  (cond
    (:reader-source case)
    (let [reader-result (projection-host/read-host-value (:reader-source case))]
      (if (= :ok (:status reader-result))
        (assoc reader-result :value (project-reader-value (:value reader-result)))
        reader-result))

    (:host-form-source case)
    (let [host-result (projection-host/host-eval-source (:source-id case)
                                                        (:host-form-source case))]
      (if (= :ok (:status host-result))
        (assoc host-result :value (project-reader-value (:value host-result)))
        host-result))

    (:control-flow-source case)
    (control-flow-source-term (:source-id case)
                              (:control-flow-source case))

    (:control-flow-trace-source case)
    (control-flow-trace-source-term (:source-id case)
                                    (:control-flow-trace-source case))

    (:macroexpand-source case)
    (macroexpand-source-term (:source-id case)
                             (:macroexpand-source case))

    (:dynamic-binding-source case)
    (dynamic-binding-source-term (:source-id case)
                                 (:dynamic-binding-source case))

    (:java-interop-source case)
    (java-interop-source-term (:source-id case)
                              (:java-interop-source case))

    (:reflection-source case)
    (reflection-source-term (:source-id case)
                            (:reflection-source case))

    (:classloader-source case)
    (classloader-source-term (:source-id case)
                             (:classloader-source case))

    (:namespace-resolution-source case)
    (namespace-resolution-source-term (:source-id case)
                                      (:namespace-resolution-source case))

    (:host-object-construction-source case)
    (host-object-construction-source-term (:source-id case)
                                          (:host-object-construction-source case))

    (:polymorphism-source case)
    (polymorphism-source-term (:source-id case)
                              (:polymorphism-source case))

    (:metadata-source case)
    (metadata-source-term (:source-id case)
                          (:metadata-source case))

    (:state-effect-source case)
    (state-effect-source-term (:source-id case)
                              (:state-effect-source case))

    (:lazy-evaluation-source case)
    (lazy-evaluation-source-term (:source-id case)
                                 (:lazy-evaluation-source case))

    (:concurrency-source case)
    (concurrency-source-term (:source-id case)
                             (:concurrency-source case))

    (:coordination-source case)
    (coordination-source-term (:source-id case)
                              (:coordination-source case))

    :else
    (err/failed :projection :clojure-projection-fixture-source-missing)))

(defn projection-runtime
  []
  (px-runtime/runtime-artifact-evaluation projection-root
                                          projection-relative-path))

(defn- apply-validator
  [validator term]
  (if (= :closure (:kind validator))
    (let [result (evaluator/apply-callable validator term)]
      (if (= :ok (:status result))
        (evaluator/force-normal (:value result))
        result))
    (err/failed :projection
                :clojure-projection-validator-missing
                {:validator-kind (:kind validator)})))

(defn validate-term
  [term]
  (let [runtime (projection-runtime)
        validator (get-in runtime [:value "validateTerm"])
        validation-result (when (= :ok (:status runtime))
                            (apply-validator validator term))]
    {:status (if (and (= :ok (:status runtime))
                      (= :ok (:status validation-result))
                      (= "ok" (get-in validation-result [:value "status"])))
               :ok
               :failed)
     :reason (if (and (= :ok (:status runtime))
                      (= :ok (:status validation-result))
                      (= "ok" (get-in validation-result [:value "status"])))
               :clojure-projection-term-valid
               :clojure-projection-term-invalid)
     :runtime-status (:status runtime)
     :runtime-reason (:reason runtime)
     :runtime-artifact (select-keys (:artifact runtime)
                                    [:root :relative-path :hash])
     :validation-status (:status validation-result)
     :validation-value (:value validation-result)}))

(defn- case-row
  [runtime case]
  (let [term-result (case-term case)
        term (when (= :ok (:status term-result))
               (:value term-result))
        validation (when term
                     (validate-term term))
        validation-value (:validation-value validation)
        ok? (and (= :ok (:status runtime))
                 (= :ok (:status term-result))
                 (= :ok (:status validation))
                 (= "ok" (get validation-value "status"))
                 (= (:expected-tag case) (get term "tag"))
                 (or (nil? (:expected-kind case))
                     (= (:expected-kind case) (get term "kind"))))]
    {:kind :clojure-projection-row
     :source-id (:source-id case)
     :source-kind (:source-kind case)
     :source-hash (hash/sha256 (:source case))
     :fixture-hash (:fixture-hash case)
     :status (if ok? :accepted :rejected)
     :reason (if ok?
               :clojure-projection-term-valid
               :clojure-projection-term-invalid)
     :source (:source case)
     :source-result-status (:status term-result)
     :source-result-reason (:reason term-result)
     :host-crossing (when (:witness term-result)
                      (select-keys term-result
                                   [:interop :capability :witness]))
     :term term
     :term-hash (some-> term hash/data-hash)
     :expected {:tag (:expected-tag case)
                :kind (:expected-kind case)}
     :validation-status (:status validation)
     :validation-value validation-value
     :runtime-artifact (select-keys (:artifact runtime)
                                    [:root :relative-path :hash])
     :runtime-status (:status runtime)
     :runtime-reason (:reason runtime)}))

(defn- reason-counts
  [rows status]
  (->> rows
       (filter #(= status (:status %)))
       (keep :reason)
       frequencies
       (into {})))

(defn- row-frontier
  [row]
  (when (not= :accepted (:status row))
    {:source-id (:source-id row)
     :status (:status row)
     :reason (:reason row)
     :runtime-status (:runtime-status row)
     :source-kind (:source-kind row)
     :source-result-status (:source-result-status row)
     :validation-status (:validation-status row)
     :validation-value (:validation-value row)}))

(defn report
  []
  (let [fixtures (fixture-set)
        cases (cases)
        runtime (projection-runtime)
        rows (mapv #(case-row runtime %) cases)
        counts (frequencies (map :status rows))]
    {:kind :clojure-projection-report
     :fixture-kind (:kind fixtures)
     :fixture-schema-version (:schema-version fixtures)
     :lineage (:lineage fixtures)
     :schema (get-in runtime [:value "schema"])
     :runtime-status (:status runtime)
     :runtime-reason (:reason runtime)
     :runtime-artifact (select-keys (:artifact runtime)
                                    [:root :relative-path :hash])
     :runtime-self-test (get-in runtime [:value "selfTest"])
     :fixture-count (count cases)
     :total (count rows)
     :accepted (long (get counts :accepted 0))
     :held (long (get counts :held 0))
     :rejected (long (get counts :rejected 0))
     :reason-counts (->> rows (keep :reason) frequencies (into {}))
     :held-reason-counts (reason-counts rows :held)
     :rejected-reason-counts (reason-counts rows :rejected)
     :host-crossing-count (count (filter :host-crossing rows))
     :first-held (first (filter #(= :held (:status %)) rows))
     :first-rejected (first (filter #(= :rejected (:status %)) rows))
     :first-frontier (first (keep row-frontier rows))
     :fixture-hashes (mapv #(select-keys % [:source-id :fixture-hash])
                           cases)
     :clojure-projection-rows rows}))

(defn -main
  [& _]
  (let [{:keys [fixture-count accepted rejected held reason-counts
                runtime-status runtime-reason runtime-self-test
                clojure-projection-rows first-frontier]}
        (report)]
    (println (format "pnix-clj clojure-projection: fixtures=%d accepted=%d rejected=%d held=%d runtime=%s/%s self-test=%s"
                     fixture-count accepted rejected held
                     (name runtime-status)
                     (name runtime-reason)
                     (get runtime-self-test "status")))
    (println "reason-counts:" (pr-str reason-counts))
    (doseq [{:keys [source-id status reason term validation-value]}
            clojure-projection-rows]
      (println (format "  %s status=%s reason=%s tag=%s validation=%s"
                       (name source-id)
                       (name status)
                       (name reason)
                       (get term "tag")
                       (get validation-value "status"))))
    (when first-frontier
      (println "first frontier:" (pr-str first-frontier)))
    (shutdown-agents)
    (when (or (pos? rejected) (pos? held))
      (System/exit 1))))
