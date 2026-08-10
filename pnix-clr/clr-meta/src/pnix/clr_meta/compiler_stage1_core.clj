(ns pnix.clr-meta.compiler-stage1-core
  "Pure, bounded lowering core for the first CLR compiler profile.

  This namespace owns form validation and opcode selection.  It performs no
  filesystem access, process execution, host evaluation, or CLR emission."
  )

(def profile-schema :pnix.clr-meta.compiler-profile.v1)
(def profile-id :pnix.clr-meta.checked-i64-expression.v1)
(def max-nodes 256)
(def max-depth 64)
(def max-source-bytes 4096)
(def max-reader-depth 80)

(def expected-profile
  {:schema profile-schema
   :id profile-id
   :input-contract {:kind :strict-edn-one-form
                    :parameter 'arg
                    :metadata :rejected
                    :max-source-bytes max-source-bytes
                    :max-reader-depth max-reader-depth
                    :max-nodes max-nodes
                    :max-depth max-depth}
   :value-domain [:system-int64]
   :special-forms []
   :primitive-calls [{:symbol '+ :arity 2 :semantics :checked-i64-add}
                     {:symbol '- :arity 2 :semantics :checked-i64-subtract}
                     {:symbol '* :arity 2 :semantics :checked-i64-multiply}]
   :error-contract {:unsupported :structured-rejection
                    :overflow :system-overflow-exception}
   :output-abi {:kind :managed-console
                :arguments [:system-int64]
                :stdout :system-int64-newline
                :entry "ClrMeta.Stage1.Program/Main"
                :callable "ClrMeta.Stage1.Program/Run"}
   :unsupported-policy :reject-no-fallback})

(def ^:private operator->instruction
  {'+ :add-ovf
   '- :sub-ovf
   '* :mul-ovf})

(defn compiler-failure
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.compiler-stage1-error.v1
                          :phase :compiler-stage1-lowering
                          :class class}
                         evidence))))

(defn validate-profile
  "Accept only the complete, versioned Stage1 profile."
  [profile]
  (when-not (= expected-profile profile)
    (compiler-failure :profile-contract-mismatch
                      "compiler profile does not equal the admitted Stage1 profile"
                      {:expected-id profile-id
                       :actual-id (when (map? profile) (:id profile))}))
  profile)

(defn- i64-literal?
  [value]
  (instance? System.Int64 value))

(defn- combine-accounting
  [kind depth children]
  (let [nodes (inc (reduce + 0 (map #(get-in % [:accounting :nodes]) children)))
        deepest (reduce max depth (map #(get-in % [:accounting :max-depth]) children))
        kinds (apply merge-with +
                     {kind 1}
                     (map #(get-in % [:accounting :node-kinds]) children))]
    (when (> nodes max-nodes)
      (compiler-failure :node-budget-exceeded
                        "Stage1 source exceeds the admitted node budget"
                        {:max-nodes max-nodes :actual-nodes nodes}))
    {:nodes nodes
     :max-depth deepest
     :node-kinds kinds}))

(declare lower*)

(defn- lower-call
  [form depth]
  (when-not (= 3 (count form))
    (compiler-failure :unsupported-arity
                      "Stage1 primitive calls require exactly two operands"
                      {:form (pr-str form)
                       :arity (dec (count form))}))
  (let [[operator left-form right-form] form
        instruction (get operator->instruction operator)]
    (when (and (instance? clojure.lang.IObj operator)
               (seq (meta operator)))
      (compiler-failure :metadata-not-admitted
                        "operator metadata is outside the exact Stage1 profile"
                        {:metadata (pr-str (meta operator))}))
    (when-not instruction
      (compiler-failure :unsupported-operator
                        "form operator is outside the admitted Stage1 profile"
                        {:operator (pr-str operator)}))
    (let [left (lower* left-form (inc depth))
          right (lower* right-form (inc depth))
          kind (keyword (str "call-" (case operator + "add" - "subtract" * "multiply")))]
      {:instructions (into (into (:instructions left) (:instructions right))
                           [[instruction]])
       :accounting (combine-accounting kind depth [left right])})))

(defn- lower*
  [form depth]
  (when (> depth max-depth)
    (compiler-failure :depth-budget-exceeded
                      "Stage1 source exceeds the admitted nesting depth"
                      {:max-depth max-depth :actual-depth depth}))
  (when (and (instance? clojure.lang.IObj form)
             (seq (meta form)))
    (compiler-failure :metadata-not-admitted
                      "metadata is outside the exact Stage1 form profile"
                      {:metadata (pr-str (meta form))}))
  (cond
    (i64-literal? form)
    {:instructions [[:ldc-i8 (long form)]]
     :accounting (combine-accounting :i64-literal depth [])}

    (integer? form)
    (compiler-failure :non-i64-integer
                      "integer literal is not represented as System.Int64"
                      {:literal (str form)
                       :value-type (str (type form))})

    (number? form)
    (compiler-failure :integer-out-of-range
                      "numeric literal is outside the System.Int64 domain"
                      {:literal (str form)
                       :value-type (str (type form))})

    (= 'arg form)
    {:instructions [[:ldarg-0]]
     :accounting (combine-accounting :parameter depth [])}

    (symbol? form)
    (compiler-failure :unsupported-symbol
                      "symbol is outside the admitted Stage1 profile"
                      {:symbol (str form)})

    (seq? form)
    (lower-call form depth)

    :else
    (compiler-failure :unsupported-form
                      "value is outside the admitted Stage1 form domain"
                      {:value-type (str (type form))
                       :form (pr-str form)})))

(defn lower
  "Lower one already-read admitted form to a canonical direct-IL plan."
  [profile form]
  (validate-profile profile)
  (let [lowered (lower* form 1)
        accounting (:accounting lowered)]
    {:schema :pnix.clr-meta.compiler-stage1-ir.v1
     :profile profile-id
     :instructions (:instructions lowered)
     :accounting (assoc accounting
                        :directly-lowered (:nodes accounting)
                        :runtime-primitive-nodes 0
                        :rejected-nodes 0
                        :unknown-nodes 0)}))
