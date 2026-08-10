(ns pnix.clr-meta.compiler-selfhost-source
  "Safe reader and every-node lexical/call census for the CLR selfhost source.

  The validated contract and profile are inputs. This namespace neither owns
  their schema nor performs receipt publication."
  (:require [clojure.set :as set]
            [clojure.string :as str]
            [pnix.clr-meta.runtime-artifact :as artifact]))

(def ^:private utf8 (System.Text.UTF8Encoding. false true))
(def ^:private simple-symbol-pattern #"[A-Za-z_*!+?<>=$%-][A-Za-z0-9_*!+?<>=$%-]*")
(def ^:private generated-metadata-keys
  #{:line :column :end-line :end-column :source-span})

(defn- fail!
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.compiler-selfhost-admission-error.v1
                          :phase :compiler-selfhost-admission
                          :class class}
                         evidence))))

(defn- local-symbol?
  [value]
  (and (symbol? value)
       (nil? (namespace value))
       (boolean (re-matches simple-symbol-pattern (name value)))
       (not (contains? '#{& . /} value))))

(defn- reject-reader-sugar!
  [text]
  ;; The source family intentionally has no reader-macro surface.  Checking
  ;; the raw stream closes expansions such as #() and syntax-quote before the
  ;; ordinary reader can turn them into apparently admitted fn*/call forms.
  (loop [index 0 in-string? false escaped? false in-comment? false]
    (when (< index (count text))
      (let [character (.get_Chars text index)]
        (cond
          in-comment?
          (recur (inc index) false false (not (or (= character \newline)
                                                  (= character \return))))

          in-string?
          (cond
            escaped? (recur (inc index) true false false)
            (= character \\) (recur (inc index) true true false)
            (= character \") (recur (inc index) false false false)
            :else (recur (inc index) true false false))

          (= character \;) (recur (inc index) false false true)
          (= character \") (recur (inc index) true false false)
          (contains? #{\^ \` \~ \@ \#} character)
          (fail! :reader-sugar-not-admitted
                 "metadata and reader-macro sugar are outside the selfhost source family"
                 {:offset index :character (str character)})
          :else (recur (inc index) false false false))))))

(defn read-forms!
  [path max-source-bytes max-top-level-forms]
  (let [bytes (System.IO.File/ReadAllBytes path)]
    (when (> (alength bytes) max-source-bytes)
      (fail! :source-byte-budget
             "selfhost compiler source exceeds the declared byte budget"
             {:maximum max-source-bytes :actual (alength bytes)}))
    (let [text (.GetString utf8 bytes)
          eof (Object.)]
      (reject-reader-sugar! text)
      (try
        (with-open [reader (clojure.lang.LineNumberingTextReader.
                            (System.IO.StringReader. text))]
          (binding [*read-eval* false
                    *data-readers* {}
                    *default-data-reader-fn*
                    (fn [tag _]
                      (fail! :tagged-literal-not-admitted
                             "tagged literals are outside the selfhost source family"
                             {:tag tag}))]
            (loop [forms []]
              (let [form (read {:eof eof :read-cond :preserve} reader)]
                (if (identical? eof form)
                  forms
                  (let [forms (conj forms form)]
                    (when (> (count forms) max-top-level-forms)
                      (fail! :top-level-form-budget
                             "selfhost source exceeds the top-level form budget"
                             {:maximum max-top-level-forms
                              :actual (count forms)}))
                    (recur forms)))))))
        (catch clojure.lang.ExceptionInfo cause
          (throw cause))
        (catch Exception cause
          (fail! :invalid-source
                 "selfhost compiler source is not valid safe Clojure data"
                 {:path path :cause-type (str (type cause))}))))))

(defn- user-metadata
  [value]
  (when (instance? clojure.lang.IObj value)
    (not-empty (apply dissoc (or (meta value) {}) generated-metadata-keys))))

(defn- ensure-no-metadata!
  [value path]
  (when-let [metadata (user-metadata value)]
    (fail! :metadata-not-admitted
           "metadata is outside the selfhost source family"
           {:path path :metadata (pr-str metadata)})))

(defn- sorted-string-frequencies
  [values]
  (into (sorted-map) (frequencies values)))

(defn- call-table
  [rows _kind]
  (into {}
        (map (fn [{:keys [symbol arity]}]
               [symbol #{arity}]))
        rows))

(defn- make-state
  [limits]
  (atom {:limits limits
         :rows []
         :specials []
         :calls []
         :max-depth 0}))

(defn- forbidden-symbol?
  [symbol forbidden]
  (or (contains? (set (:symbols forbidden)) symbol)
      (some #(str/starts-with? (str symbol) %)
            (:symbol-prefixes forbidden))))

(defn- scan-forbidden!
  [form path forbidden]
  (ensure-no-metadata! form path)
  (when (and (seq? form)
             (symbol? (first form))
             (contains? (set (:forms forbidden)) (first form)))
    (fail! :forbidden-form
           "source contains a macro, host form, or fallback form forbidden by C0"
           {:path path :symbol (first form)}))
  (when (and (symbol? form) (forbidden-symbol? form forbidden))
    (fail! :forbidden-symbol
           "source references a host compiler, loader, process, or reflection escape"
           {:path path :symbol form}))
  (when (and (keyword? form)
             (contains? (set (:payloads forbidden)) form))
    (fail! :forbidden-payload
           "source names a baked or opaque artifact payload forbidden by C0"
           {:path path :payload form}))
  (cond
    (map? form)
    (doseq [[index [key value]] (map-indexed vector form)]
      (scan-forbidden! key (conj path "map-key" index) forbidden)
      (scan-forbidden! value (conj path "map-value" index) forbidden))

    (or (vector? form) (set? form) (seq? form))
    (doseq [[index child] (map-indexed vector form)]
      (scan-forbidden! child (conj path index) forbidden)))
  nil)

(defn- add-node!
  ([state path depth kind]
   (add-node! state path depth kind {}))
  ([state path depth kind details]
   (let [{:keys [max-depth max-nodes]} (:limits @state)]
     (when (> depth max-depth)
       (fail! :source-depth-budget
              "selfhost compiler source exceeds the declared depth budget"
              {:path path :maximum max-depth :actual depth}))
     (let [row (merge (sorted-map "kind" (name kind)
                                  "path" path)
                      details)]
       (swap! state
              (fn [current]
                (let [rows (conj (:rows current) row)]
                  (when (> (count rows) max-nodes)
                    (fail! :source-node-budget
                           "selfhost compiler source exceeds the declared node budget"
                           {:path path :maximum max-nodes
                            :actual (count rows)}))
                  (assoc current
                         :rows rows
                         :max-depth (max (:max-depth current) depth)))))))
   nil))

(defn- add-special!
  [state special]
  (swap! state update :specials conj (name special)))

(defn- add-call!
  [state call-kind symbol arity]
  (swap! state update :calls conj
         {:kind (name call-kind) :symbol (str symbol) :arity arity}))

(defn- infer-fn-arity
  [form]
  (when (and (seq? form) (= 'fn* (first form)))
    (let [tail (next form)
          [name tail] (if (symbol? (first tail))
                        [(first tail) (next tail)]
                        [nil tail])
          parameters (first tail)
          body (next tail)]
      (when (and (or (nil? name) (local-symbol? name))
                 (vector? parameters)
                 (every? local-symbol? parameters)
                 (not-any? #{'&} parameters)
                 (= 1 (count body)))
        (count parameters)))))

(defn- collect-definitions!
  [forms namespace-symbol limits]
  (when (empty? forms)
    (fail! :empty-source "selfhost compiler source has no namespace envelope" {}))
  (let [namespace-form (first forms)]
    (ensure-no-metadata! namespace-form ["top-level" 0])
    (when-not (and (seq? namespace-form)
                   (= 2 (count namespace-form))
                   (= 'ns (first namespace-form))
                   (= namespace-symbol (second namespace-form)))
      (fail! :namespace-envelope-mismatch
             "selfhost compiler source must begin with its exact two-item ns envelope"
             {:expected namespace-symbol :actual (pr-str namespace-form)})))
  (let [definition-forms (subvec forms 1)]
    (when (> (count definition-forms) (:max-definitions limits))
      (fail! :definition-budget
             "selfhost compiler source exceeds the definition budget"
             {:maximum (:max-definitions limits)
              :actual (count definition-forms)}))
    (reduce-kv
     (fn [definitions offset form]
       (let [index (inc offset)
             path ["top-level" index]]
         (ensure-no-metadata! form path)
         (when-not (and (seq? form)
                        (= 3 (count form))
                        (= 'def (first form)))
           (fail! :top-level-form-not-def
                  "only exact three-item def forms may follow the namespace envelope"
                  {:path path :form (pr-str form)}))
         (let [[_ name initializer] form]
           (ensure-no-metadata! name (conj path "name"))
           (when-not (local-symbol? name)
             (fail! :invalid-definition-name
                    "selfhost definitions require simple unqualified symbols"
                    {:path path :name name}))
           (when (contains? definitions name)
             (fail! :duplicate-definition
                    "selfhost source must not redefine a global"
                    {:path path :name name}))
           (assoc definitions name {:arity (infer-fn-arity initializer)
                                    :path path}))))
     (array-map)
     definition-forms)))

(defn- definition-initializers
  [forms]
  (into {}
        (map (fn [form] [(second form) (nth form 2)]))
        (subvec forms 1)))

(defn- named-function-body
  [definitions name]
  (let [initializer (get definitions name)]
    (when-not (and (seq? initializer)
                   (= 'fn* (first initializer))
                   (= name (second initializer))
                   (= 4 (count initializer)))
      (fail! :seed-owner-not-function
             "support/intrinsic seed owner must be an exact named fn*"
             {:owner name}))
    (nth initializer 3)))

(defn- linear-let-bindings
  [body owner]
  (when-not (and (seq? body) (= 'let* (first body))
                 (= 3 (count body)) (vector? (second body))
                 (even? (count (second body))))
    (fail! :seed-let-shape
           "support/intrinsic seed must be one exact linear let*"
           {:owner owner}))
  (let [bindings (second body)
        rows (mapv (fn [index]
                     {:name (nth bindings (* 2 index))
                      :value (nth bindings (inc (* 2 index)))})
                   (range (quot (count bindings) 2)))]
    (when (empty? rows)
      (fail! :empty-seed "support/intrinsic seed must not be empty"
             {:owner owner}))
    (when-not (= (:name (last rows)) (nth body 2))
      (fail! :seed-result-not-last
             "seed let* must return its final environment binding"
             {:owner owner :actual (nth body 2)}))
    rows))

(defn- seed-receipt
  [kind rows]
  (let [semantic {"kind" kind
                  "rows" rows}]
    (assoc semantic
           "semantic_sha256"
           (artifact/sha256-string (artifact/manifest-json semantic)))))

(defn- verify-seed-bindings!
  [contract forms]
  (let [definitions (definition-initializers forms)
        support-bindings
        (linear-let-bindings
         (named-function-body definitions 'seed-support-calls)
         'seed-support-calls)
        support-actual
        (mapv
         (fn [index {:keys [name value]}]
           (let [previous (if (zero? index)
                            'env
                            (:name (nth support-bindings (dec index))))]
             (when-not (and (seq? value)
                            (= 'bind-support-call (first value))
                            (= 4 (count value))
                            (= previous (second value))
                            (string? (nth value 2))
                            (instance? System.Int64 (nth value 3)))
               (fail! :support-seed-row
                      "support seed row must extend the preceding environment"
                      {:index index :binding name :value (pr-str value)}))
             {"arity" (long (nth value 3))
              "symbol" (nth value 2)}))
         (range)
         support-bindings)
        support-expected
        (mapv (fn [{:keys [symbol arity]}]
                {"arity" arity "symbol" (str symbol)})
              (get-in contract [:support-abi :calls]))
        intrinsic-bindings
        (linear-let-bindings
         (named-function-body definitions 'seed-intrinsics)
         'seed-intrinsics)
        intrinsic-actual
        (mapv
         (fn [index {:keys [name value]}]
           (let [previous (if (zero? index)
                            'env
                            (:name (nth intrinsic-bindings (dec index))))]
             (when-not (and (seq? value)
                            (= 'pnix.clr-meta.compiler-support.data.v1/env-bind
                               (first value))
                            (= 6 (count value))
                            (= previous (second value))
                            (string? (nth value 2))
                            (= "intrinsic" (nth value 3))
                            (= (nth value 2) (nth value 4))
                            (instance? System.Int64 (nth value 5)))
               (fail! :intrinsic-seed-row
                      "intrinsic seed row must bind its exact symbol and arity"
                      {:index index :binding name :value (pr-str value)}))
             {"arity" (long (nth value 5))
              "symbol" (nth value 2)}))
         (range)
         intrinsic-bindings)
        intrinsic-expected
        (mapv (fn [{:keys [symbol arity]}]
                {"arity" arity "symbol" (str symbol)})
              (:intrinsics contract))]
    (when-not (= support-expected support-actual)
      (fail! :support-seed-contract-mismatch
             "seed-support-calls does not exactly bind the contract ABI"
             {:expected support-expected :actual support-actual}))
    (when-not (= intrinsic-expected intrinsic-actual)
      (fail! :intrinsic-seed-contract-mismatch
             "seed-intrinsics does not exactly bind the contract intrinsic table"
             {:expected intrinsic-expected :actual intrinsic-actual}))
    [(seed-receipt "support-abi" support-actual)
     (seed-receipt "intrinsics" intrinsic-actual)]))

(defn- operator-branch-matches
  [initializer input-symbol definitions]
  (->> (tree-seq coll? seq initializer)
       (filter seq?)
       (keep (fn [candidate]
               (when (and (= 'if (first candidate))
                          (= 4 (count candidate)))
                 (let [test (second candidate)
                       consequent (nth candidate 2)]
                   (when (and (seq? test)
                              (= 3 (count test))
                              (= input-symbol (nth test 2))
                              (symbol? consequent)
                              (contains? definitions consequent))
                     (get definitions consequent))))))))

(defn- verify-mutation-sites!
  [contract forms]
  (let [definitions (definition-initializers forms)]
    (mapv
     (fn [site]
       (let [definition (:def site)
             initializer (get definitions definition ::missing)]
         (when (= ::missing initializer)
           (fail! :mutation-definition-missing
                  "contract mutation site names no source definition"
                  {:site (:id site) :definition definition}))
         (case (:selector site)
           :string-literal
           (do
             (when-not (= (:expected site) initializer)
               (fail! :mutation-literal-mismatch
                      "source identity literal differs from its C0 mutation site"
                      {:site (:id site) :expected (:expected site)
                       :actual initializer}))
             (let [semantic {"definition" (str definition)
                             "id" (name (:id site))
                             "observed" initializer
                             "selector" "string-literal"}]
               (assoc semantic
                      "semantic_sha256"
                      (artifact/sha256-string (artifact/manifest-json semantic)))))

           :operator-branch
           (let [matches (operator-branch-matches
                          initializer (:input-symbol site) definitions)]
             (when-not (= [(:expected-opcode site)] matches)
               (fail! :mutation-lowering-mismatch
                      "operator lowering branch is absent, duplicated, or changed"
                      {:site (:id site) :input-symbol (:input-symbol site)
                       :expected (:expected-opcode site)
                       :actual (vec matches)}))
             (let [semantic {"definition" (str definition)
                             "id" (name (:id site))
                             "input_symbol" (:input-symbol site)
                             "observed" (first matches)
                             "selector" "operator-branch"}]
               (assoc semantic
                      "semantic_sha256"
                      (artifact/sha256-string (artifact/manifest-json semantic))))))))
     (:mutation-sites contract))))

(defn- verify-definition-closure!
  [definitions forms profile]
  (let [actual-order (mapv second (subvec forms 1))
        expected-order (get-in profile [:top-level :definitions])]
    (when-not (= expected-order actual-order)
      (fail! :definition-closure-mismatch
             "source definitions must equal the exact ordered profile closure"
             {:expected expected-order :actual actual-order})))
  (doseq [{:keys [symbol arity]} (concat (get-in profile [:calls :kernel-global])
                                         (get-in profile [:calls :lexical :calls]))]
    (let [actual (get-in definitions [symbol :arity] ::missing)]
      (when-not (= arity actual)
        (fail! :defined-call-arity-mismatch
               "declared kernel/recursive call arity differs from its fn* definition"
               {:symbol symbol :expected arity :actual actual}))))
  (let [entry (get-in profile [:top-level :entry])]
    (when (nil? (get-in definitions [entry :arity]))
      (fail! :entry-not-function
             "selfhost source entry must be a statically aritied fn* definition"
             {:entry entry})))
  definitions)

(defn- verify-lowering-coverage!
  [contract definitions construct-counts]
  (mapv
   (fn [{:keys [construct owner]}]
     (let [owner-arity (get-in definitions [owner :arity] ::missing)
           observed (get construct-counts construct 0)]
       (when (= ::missing owner-arity)
         (fail! :lowering-owner-missing
                "lowering coverage owner is absent from the source closure"
                {:construct construct :owner owner}))
       (when (nil? owner-arity)
         (fail! :lowering-owner-not-function
                "lowering coverage owner must be a named fn* definition"
                {:construct construct :owner owner}))
       (when-not (pos? observed)
         (fail! :lowering-construct-not-observed
                "declared lowering construct is absent from source census"
                {:construct construct :owner owner}))
       (let [semantic {"construct" (name construct)
                       "observed_nodes" observed
                       "owner" (str owner)
                       "owner_arity" owner-arity
                       "owner_is_function" true}]
         (assoc semantic
                "semantic_sha256"
                (artifact/sha256-string
                 (artifact/manifest-json semantic))))))
   (:lowering-coverage contract)))

(declare analyze-expression!)

(defn- analyze-parameters!
  [parameters path depth context]
  (let [{:keys [state limits]} context]
    (ensure-no-metadata! parameters path)
    (when-not (vector? parameters)
      (fail! :fn-parameters-not-vector
             "fn* parameters must be an exact vector"
             {:path path :actual-type (str (type parameters))}))
    (when (> (count parameters) (:max-parameters limits))
      (fail! :parameter-budget
             "fn* parameters exceed the declared budget"
             {:path path :maximum (:max-parameters limits)
              :actual (count parameters)}))
    (when-not (= (count parameters) (count (distinct parameters)))
      (fail! :duplicate-parameter
             "fn* parameters must be unique"
             {:path path}))
    (add-node! state path depth :parameter-vector
               {"count" (count parameters)})
    (reduce-kv
     (fn [locals index parameter]
       (let [parameter-path (conj path index)]
         (ensure-no-metadata! parameter parameter-path)
         (when-not (local-symbol? parameter)
           (fail! :invalid-parameter
                  "fn* parameters must be simple symbols; variadics are not admitted"
                  {:path parameter-path :parameter parameter}))
         (add-node! state parameter-path (inc depth) :lexical-binding
                    {"symbol" (str parameter)})
         (assoc locals parameter nil)))
     (:locals context)
     parameters)))

(defn- literal-kind
  [value]
  (cond
    (nil? value) :nil
    (boolean? value) :boolean
    (string? value) :string
    (instance? System.Int64 value) :system-int64
    (integer? value) :non-int64-integer
    :else nil))

(defn- analyze-literal!
  [form path depth context]
  (let [{:keys [state literal-kinds]} context
        kind (literal-kind form)]
    (when-not (and kind (contains? literal-kinds kind))
      (fail! :literal-not-admitted
             "literal is outside the exact selfhost profile"
             {:path path :kind kind :value-type (str (type form))}))
    (add-node! state path depth kind)
    nil))

(defn- analyze-symbol!
  [symbol path depth context]
  (let [{:keys [state locals globals]} context]
    (when-not (local-symbol? symbol)
      (fail! :qualified-or-interop-symbol
             "qualified, member, and interop symbols are not admitted as values"
             {:path path :symbol symbol}))
    (cond
      (contains? locals symbol)
      (if (nil? (get locals symbol))
        (add-node! state path depth :lexical-symbol {"symbol" (str symbol)})
        (fail! :function-value-not-admitted
               "named functions may occur only in statically aritied call position"
               {:path path :symbol symbol}))

      (contains? globals symbol)
      (if (nil? (get-in globals [symbol :arity]))
        (add-node! state path depth :defined-global {"symbol" (str symbol)})
        (fail! :function-value-not-admitted
               "global functions may occur only in statically aritied call position"
               {:path path :symbol symbol}))

      :else
      (fail! :unknown-symbol
             "symbol is neither a lexical binding nor a defined global"
             {:path path :symbol symbol}))))

(defn- analyze-fn!
  [form path depth context]
  (let [{:keys [state allow-fn?]} context
        tail (next form)
        [function-name tail] (if (symbol? (first tail))
                               [(first tail) (next tail)]
                               [nil tail])
        parameters (first tail)
        body (next tail)]
    (when-not allow-fn?
      (fail! :nested-fn-not-admitted
             "fn* is admitted only as a top-level def initializer"
             {:path path}))
    (when-not (and parameters (= 1 (count body)))
      (fail! :fn-shape
             "fn* admits exactly one parameter vector and one body expression"
             {:path path :form (pr-str form)}))
    (add-node! state path depth :special-fn)
    (add-special! state :fn*)
    (add-node! state (conj path "operator") (inc depth) :special-marker
               {"symbol" "fn*"})
    (when function-name
      (ensure-no-metadata! function-name (conj path "name"))
      (when-not (local-symbol? function-name)
        (fail! :invalid-fn-name
               "named fn* recursion requires a simple symbol"
               {:path path :name function-name}))
      (add-node! state (conj path "name") (inc depth) :lexical-binding
                 {"symbol" (str function-name)}))
    (let [arity (when (vector? parameters) (count parameters))
          base-body-context (assoc context :allow-fn? false)
          body-context (if function-name
                         (assoc-in base-body-context [:locals function-name] arity)
                         base-body-context)
          locals (analyze-parameters! parameters (conj path "parameters")
                                      (inc depth) body-context)]
      (analyze-expression! (first body) (conj path "body") (inc depth)
                           (assoc body-context :locals locals)))))

(defn- analyze-if!
  [form path depth context]
  (let [{:keys [state]} context]
    (when-not (= 4 (count form))
      (fail! :if-arity "if requires test, then, and else expressions"
             {:path path :arity (dec (count form))}))
    (add-node! state path depth :special-if)
    (add-special! state :if)
    (add-node! state (conj path "operator") (inc depth) :special-marker
               {"symbol" "if"})
    (analyze-expression! (nth form 1) (conj path "test") (inc depth) context)
    (analyze-expression! (nth form 2) (conj path "then") (inc depth) context)
    (analyze-expression! (nth form 3) (conj path "else") (inc depth) context)))

(defn- analyze-let!
  [form path depth context]
  (let [{:keys [state limits]} context]
    (when-not (= 3 (count form))
      (fail! :let-shape
             "let* requires one binding vector and one body expression"
             {:path path :arity (dec (count form))}))
    (let [bindings (second form)
          body (nth form 2)
          binding-path (conj path "bindings")]
      (ensure-no-metadata! bindings binding-path)
      (when-not (and (vector? bindings) (even? (count bindings)))
        (fail! :let-bindings-shape
               "let* bindings must be an even vector"
               {:path binding-path :actual-type (str (type bindings))}))
      (let [binding-count (quot (count bindings) 2)]
        (when (> binding-count (:max-bindings limits))
          (fail! :binding-budget
                 "let* bindings exceed the declared budget"
                 {:path binding-path :maximum (:max-bindings limits)
                  :actual binding-count}))
        (add-node! state path depth :special-let)
        (add-special! state :let*)
        (add-node! state (conj path "operator") (inc depth) :special-marker
                   {"symbol" "let*"})
        (add-node! state binding-path (inc depth) :binding-vector
                   {"count" binding-count})
        (let [locals
              (reduce
               (fn [locals index]
                 (let [name (nth bindings (* index 2))
                       initializer (nth bindings (inc (* index 2)))
                       name-path (conj binding-path index "name")
                       value-path (conj binding-path index "value")]
                   (ensure-no-metadata! name name-path)
                   (when-not (local-symbol? name)
                     (fail! :invalid-binding-name
                            "let* binding names must be simple symbols"
                            {:path name-path :name name}))
                   (when (contains? locals name)
                     (fail! :duplicate-local-binding
                            "let* must not shadow a binding in the same lexical frame"
                            {:path name-path :name name}))
                   (add-node! state name-path (+ depth 2) :lexical-binding
                              {"symbol" (str name)})
                   (analyze-expression! initializer value-path (+ depth 2)
                                        (assoc context :locals locals))
                   (assoc locals name (infer-fn-arity initializer))))
               (:locals context)
               (range binding-count))]
          (analyze-expression! body (conj path "body") (inc depth)
                               (assoc context :locals locals)))))))

(defn- analyze-do!
  [form path depth context]
  (let [{:keys [state]} context]
    (when (< (count form) 2)
      (fail! :do-empty "do requires at least one expression" {:path path}))
    (add-node! state path depth :special-do)
    (add-special! state :do)
    (add-node! state (conj path "operator") (inc depth) :special-marker
               {"symbol" "do"})
    (doseq [[index expression] (map-indexed vector (next form))]
      (analyze-expression! expression (conj path "expression" index)
                           (inc depth) context))))

(defn- require-call-arity!
  [symbol allowed actual path call-kind]
  (when-not (contains? allowed actual)
    (fail! :call-arity-not-admitted
           "call arity is outside the exact selfhost ABI"
           {:path path :symbol symbol :call-kind call-kind
            :expected (vec (sort allowed)) :actual actual})))

(defn- analyze-call!
  [form path depth context]
  (let [{:keys [state locals globals support-calls sink-calls kernel-calls
                lexical-calls intrinsic-calls]} context
        operator (first form)
        arguments (next form)
        arity (count arguments)]
    (ensure-no-metadata! operator (conj path "operator"))
    (when-not (symbol? operator)
      (fail! :computed-call-not-admitted
             "call position must contain one statically known symbol"
             {:path path :operator-type (str (type operator))}))
    (let [[call-kind allowed]
          (cond
            (contains? locals operator)
            (let [known-arity (get locals operator)]
              (when (nil? known-arity)
                (fail! :higher-order-call-not-admitted
                       "parameters and non-function locals may not be called"
                       {:path path :symbol operator}))
              (when-not (contains? lexical-calls operator)
                (fail! :lexical-call-not-admitted
                       "local call is outside named-recursion allowlist"
                       {:path path :symbol operator}))
              [:lexical #{known-arity}])
            (contains? sink-calls operator) [:sink (get sink-calls operator)]
            (contains? support-calls operator) [:support (get support-calls operator)]
            (contains? intrinsic-calls operator) [:intrinsic (get intrinsic-calls operator)]
            (contains? kernel-calls operator) [:kernel-global (get kernel-calls operator)]
            (contains? globals operator)
            (fail! :global-call-not-admitted
                   "defined global is not in the exact kernel call allowlist"
                   {:path path :symbol operator})
            (namespace operator)
            (fail! :support-call-not-admitted
                   "qualified call is outside the exact support ABI"
                   {:path path :symbol operator})
            :else
            (fail! :unknown-call
                   "call operator is not a lexical, kernel, or support function"
                   {:path path :symbol operator}))]
      (require-call-arity! operator allowed arity path call-kind)
      (add-node! state path depth :call
                 {"arity" arity
                  "call_kind" (name call-kind)
                  "symbol" (str operator)})
      (add-call! state call-kind operator arity)
      (add-node! state (conj path "operator") (inc depth) :call-marker
                 {"symbol" (str operator)})
      (doseq [[index argument] (map-indexed vector arguments)]
        (analyze-expression! argument (conj path "argument" index)
                             (inc depth) context)))))

(defn- analyze-expression!
  [form path depth context]
  (ensure-no-metadata! form path)
  (cond
    (or (nil? form) (boolean? form) (string? form) (integer? form))
    (analyze-literal! form path depth context)

    (symbol? form)
    (analyze-symbol! form path depth context)

    (seq? form)
    (do
      (when (empty? form)
        (fail! :empty-list-not-admitted
               "empty lists are outside the selfhost expression family"
               {:path path}))
      (case (first form)
        fn* (analyze-fn! form path depth context)
        if (analyze-if! form path depth context)
        let* (analyze-let! form path depth context)
        do (analyze-do! form path depth context)
        quote (fail! :quote-not-admitted
                     "quote is outside the selfhost source family"
                     {:path path})
        var (fail! :var-not-admitted
                   "var quote is outside the selfhost source family"
                   {:path path})
        (analyze-call! form path depth context)))

    :else
    (fail! :expression-kind-not-admitted
           "collection literals, host objects, and opaque values are not admitted"
           {:path path :value-type (str (type form))})))

(defn analyze!
  [forms contract profile]
  (let [limits (:limits profile)
        namespace-symbol (get-in contract [:source :namespace])
        definitions (-> (collect-definitions! forms namespace-symbol limits)
                        (verify-definition-closure! forms profile))
        forbidden (:forbidden contract)
        support-rows (get-in contract [:support-abi :calls])
        call-map (into {} (map (fn [{:keys [symbol arity]}]
                                [symbol #{arity}])) support-rows)
        sink-symbols (set (map :symbol (filter #(= :pesink (:partition %))
                                               support-rows)))
        support-calls (apply dissoc call-map sink-symbols)
        sink-calls (select-keys call-map sink-symbols)
        kernel-calls (call-table
                      (get-in profile [:calls :kernel-global])
                      :profile-kernel-calls)
        lexical-calls (call-table
                       (get-in profile [:calls :lexical :calls])
                       :profile-lexical-calls)
        intrinsic-calls (call-table
                         (get-in profile [:calls :intrinsic])
                         :profile-intrinsics)
        literal-kinds (->> (get-in profile [:literals :value-kinds])
                           (map #(if (= :integer %) :system-int64 %))
                           set)
        state (make-state limits)
        base-context {:state state
                      :limits limits
                      :globals definitions
                      :locals {}
                      :support-calls support-calls
                      :sink-calls sink-calls
                      :kernel-calls kernel-calls
                      :lexical-calls lexical-calls
                      :intrinsic-calls intrinsic-calls
                      :allow-fn? false
                      :literal-kinds literal-kinds}]
    (doseq [[index form] (map-indexed vector forms)]
      (scan-forbidden! form ["top-level" index] forbidden))
    (let [namespace-form (first forms)]
      (add-node! state ["top-level" 0] 1 :namespace-envelope)
      (add-node! state ["top-level" 0 "operator"] 2 :namespace-marker
                 {"symbol" "ns"})
      (add-node! state ["top-level" 0 "namespace"] 2 :namespace-symbol
                 {"symbol" (str (second namespace-form))}))
    (doseq [[offset form] (map-indexed vector (subvec forms 1))]
      (let [index (inc offset)
            [_ name initializer] form
            path ["top-level" index]]
        (add-node! state path 1 :definition {"symbol" (str name)})
        (add-node! state (conj path "operator") 2 :definition-marker
                   {"symbol" "def"})
        (add-node! state (conj path "name") 2 :global-binding
                   {"symbol" (str name)})
        (analyze-expression! initializer (conj path "initializer") 2
                             (assoc base-context :allow-fn? true))))
    (let [{:keys [rows specials calls max-depth]} @state
          node-kinds (sorted-string-frequencies (map #(get % "kind") rows))
          special-counts (sorted-string-frequencies specials)
          call-symbols (sorted-string-frequencies
                        (map (fn [{:keys [kind symbol arity]}]
                               (str kind ":" symbol "/" arity))
                             calls))
          declared-specials (set (map name (get-in profile [:expressions :special-forms])))
          actual-specials (set specials)
          declared-supports (set (map #(str (:symbol %))
                                      (get-in contract [:support-abi :calls])))
          actual-supports (set (map :symbol (filter #(contains? #{"support" "sink"}
                                                                (:kind %))
                                                    calls)))
          declared-kernel (set (map #(str (:symbol %))
                                    (remove #(= :entry (:role %))
                                            (get-in profile
                                                    [:calls :kernel-global]))))
          actual-kernel (set (map :symbol (filter #(= "kernel-global" (:kind %))
                                                  calls)))
          declared-lexical (set (map #(str (:symbol %))
                                     (get-in profile [:calls :lexical :calls])))
          actual-lexical (set (map :symbol (filter #(= "lexical" (:kind %))
                                                   calls)))
          declared-intrinsic (set (map #(str (:symbol %))
                                       (get-in profile [:calls :intrinsic])))
          actual-intrinsic (set (map :symbol
                                     (filter #(= "intrinsic" (:kind %))
                                             calls)))
          construct-counts
          {:ns (get node-kinds "namespace-envelope" 0)
           :def-constant (count (filter #(nil? (get-in % [1 :arity]))
                                        definitions))
           :def-fn (count (filter #(some? (get-in % [1 :arity]))
                                  definitions))
           :fn* (get special-counts "fn*" 0)
           :if (get special-counts "if" 0)
           :let* (get special-counts "let*" 0)
           :do (get special-counts "do" 0)
           :literal (reduce + 0
                            (map #(get node-kinds % 0)
                                 ["nil" "boolean" "string" "system-int64"]))
           :lexical-symbol (get node-kinds "lexical-symbol" 0)
           :global-symbol (get node-kinds "defined-global" 0)
           :call (get node-kinds "call" 0)
           :intrinsic (count (filter #(= "intrinsic" (:kind %)) calls))}]
      (doseq [[kind declared actual]
              [[:special-form declared-specials actual-specials]
               [:support-call declared-supports actual-supports]
               [:kernel-call declared-kernel actual-kernel]
               [:lexical-call declared-lexical actual-lexical]
               [:intrinsic-call declared-intrinsic actual-intrinsic]]]
        (when-not (set/subset? declared actual)
          (fail! :declared-source-coverage-missing
                 "declared source-language lowering surface is absent from the source census"
                 {:kind kind
                  :missing (vec (sort (set/difference declared actual)))})))
      {:rows rows
       :mutation-sites (verify-mutation-sites! contract forms)
       :seed-bindings (verify-seed-bindings! contract forms)
       :lowering-coverage
       (verify-lowering-coverage! contract definitions construct-counts)
       :accounting
       {"source_opaque_payload_nodes" 0
        "calls" call-symbols
        "definitions" (count definitions)
        "source_interpreted_nodes" 0
        "max_depth" max-depth
        "node_kinds" node-kinds
        "nodes" (count rows)
        "source_opaque_nodes" 0
        "source_rejected_nodes" 0
        "special_forms" special-counts
        "top_level_forms" (count forms)
        "source_unknown_nodes" 0}})))
