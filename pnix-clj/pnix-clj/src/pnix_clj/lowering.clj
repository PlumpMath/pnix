(ns pnix-clj.lowering
  (:require [clojure.string :as str]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.json]
            [pnix-clj.math]
            [pnix-clj.parser :as parser]
            [pnix-clj.version]))

(def lane-classification
  {:lane :core
   :scope :ast-to-host-form-lowering
   :role :pnix-ast-to-clojure-host-form-bridge
   :product-runtime :allowed
   :semantic-authority :requires-clj-meta-receipt
   :mutation :lower-cache-state-only
   :admission :forbidden
   :determinism :required
   :allowed-output :lowered-host-form-or-held-frontier})

(declare lower-ast path-value? nix-equal)

(def lowering-policy
  :expr-core-v1)

(def ^:dynamic *force-on-read-vars*
  #{})

(def ^:dynamic *lexical-vars*
  #{})

(def ^:dynamic *lexical-renames*
  "pnix-var-symbol -> emitted-Clojure-symbol, for names that must be emitted
  under a different symbol than their pnix identifier. Used by scopedImport to
  bind scope keys to collision-proof parameter names (`x` -> `x*scope`; `*` is
  not a legal pnix identifier char) so the injecting `fn` cannot lexically
  capture a same-named free var inlined from a nested import."
  {})

(def ^:dynamic *with-scope-syms*
  [])

(def ^:dynamic *with-depth*
  0)

(def ^:dynamic *import-modules*
  "In-memory pnix module map (target-string -> pnix source) used only when the
  caller explicitly binds it. Empty means imports stay a lowering frontier."
  {})

(def ^:dynamic *import-context*
  [])

(def ^:private lower-cache
  (atom {}))

(def ^:private lower-cache-stats*
  (atom {:hits 0
         :misses 0}))

(defn lower-cache-key
  [ast]
  {:schema :pnix-clj.lower-cache-key.v0
   :ast-hash (hash/data-hash ast)
   :policy lowering-policy
   :force-on-read-vars (vec (sort-by str *force-on-read-vars*))
   :lexical-vars (vec (sort-by str *lexical-vars*))
   :with-scope-syms (vec (map str *with-scope-syms*))
   :import-modules-hash (when (seq *import-modules*)
                          (hash/data-hash *import-modules*))
   :import-context (when (seq *import-modules*)
                     (vec *import-context*))})

(defn clear-lower-cache!
  []
  (reset! lower-cache {})
  (reset! lower-cache-stats* {:hits 0
                              :misses 0})
  nil)

(defn lower-cache-stats
  []
  (assoc @lower-cache-stats*
         :entries (count @lower-cache)))

(defn- ok-form
  [form]
  {:status :ok
   :form form
   :form-hash (hash/data-hash form)
   :policy lowering-policy
   :source-string-codegen? false})

(defn lazy-slot
  [f]
  (delay (f)))

(defn force-slot
  [value]
  (if (delay? value)
    (force value)
    value))

(defn force-normal
  [value]
  (let [value (force-slot value)]
    (cond
      (vector? value)
      (mapv force-normal value)

      (map? value)
      (into (empty value)
            (map (fn [[k v]]
                   [k (force-normal v)]))
            value)

      :else
      value)))

(defn lookup-with-scopes
  [scopes name]
  (if-let [scope (some (fn [scope]
                         (let [scope (force-slot scope)]
                           (when (and (map? scope) (contains? scope name))
                             scope)))
                       scopes)]
    (force-slot (get scope name))
    (throw (ex-info "unbound variable"
                    {:reason :unbound-var
                     :name name}))))

(defn assert-function
  [value]
  (if (fn? value)
    value
    (throw (ex-info "call target is not callable"
                    {:reason :call-target-not-callable
                     :target-kind (some-> value class .getName)
                     :value value}))))

(defn- lazy-slot-form
  [form]
  (list `lazy-slot (list 'fn [] form)))

(defn- force-slot-form
  [form]
  (list `force-slot form))

(defn- force-normal-form
  [form]
  (list `force-normal form))

(defn- evaluator-builtin
  "Run one of the EVALUATOR's builtins on fully-realized values — the lowered
  lane delegates string-context semantics to the single implementation instead
  of mirroring it (zero drift). A held result becomes a thrown info map, which
  this lane reports as a held form execution."
  [builtin-name & args]
  (let [f (get-in evaluator/default-env ["builtins" builtin-name])
        r (reduce (fn [acc a]
                    (let [rr (evaluator/apply-callable acc a)]
                      (if (= :ok (:status rr))
                        (:value rr)
                        (throw (ex-info (str builtin-name " held")
                                        {:reason (:reason rr)
                                         :builtin builtin-name})))))
                  f args)]
    r))

(defn host-builtin
  "Generic delegation: apply the EVALUATOR's builtin (single implementation,
  zero drift) to fully-realized arguments. Used by lowered forms for builtins
  whose results are plain data (derivation values, placeholders, ...)."
  [builtin-name & args]
  (apply evaluator-builtin builtin-name (map force-normal args)))

(defn pattern-actual
  "Realize a pattern-lambda argument to WHNF (an attrset map in the lowered
  value model). Non-maps pass through; pattern-guard mirrors the evaluator's
  D19 application-time checks."
  [v]
  (force-slot v))

(defn pattern-guard
  "D19 application-time checks for a lowered pattern lambda, in the host
  evaluator's (oracle-confirmed) order: the argument must be an attrset;
  every REQUIRED formal is checked in pattern order BEFORE the extra-key
  check; without ellipsis an extra argument key is an error. Throws plain
  ex-info — NOT :pnix/catchable, so lowered tryEval does not catch these,
  exactly like real Nix."
  [m param-names required ellipsis?]
  (when-not (evaluator/attrset-value? m)
    (throw (ex-info "pattern lambda argument is not an attrset"
                    {:reason :lambda-pattern-arg-not-attrset})))
  (when-let [missing (first (remove #(contains? m %) required))]
    (throw (ex-info (str "missing pattern argument " missing)
                    {:reason :missing-lambda-pattern-arg
                     :param missing})))
  (when-not ellipsis?
    (when-let [extra (first (sort (remove (set param-names) (keys m))))]
      (throw (ex-info (str "unexpected pattern argument " extra)
                      {:reason :unexpected-lambda-pattern-arg
                       :arg extra}))))
  m)

(defn function-args
  "Value-based functionArgs: pattern metadata rides on the lowered fn's
  Clojure metadata (:pnix/function-args); simple lambdas and builtins have
  none and report {} — mirroring the evaluator, including through variables
  (the old syntactic special-case silently returned {} for `functionArgs f`).
  Non-functions hold like the evaluator (never a silent {})."
  [f]
  (let [v (force-slot f)]
    (if (fn? v)
      (or (:pnix/function-args (meta v)) {})
      (throw (ex-info "functionArgs target is not a function"
                      {:reason :function-args-target-not-callable})))))


;; ── builtins as VALUES: the lazy bridge ─────────────────────────────
;; A bare `builtins.X` (or `builtins` itself) in value position cannot be
;; emitted per-builtin like call-position forms. Instead the whole builtin
;; set DELEGATES to the evaluator (single implementation, zero drift) via a
;; bidirectional LAZY bridge: lowered slots cross as evaluator thunks,
;; lowered fns as :lazy-host-fn callables (argument arrives RAW), and
;; evaluator thunks/callables come back as slots/fns — so delegated
;; builtins keep exactly the evaluator's forcing behavior (`length` never
;; forces elements, `map` stays element-lazy, ...).

(declare eval-value->lowered builtins-attrset)

(defn- throw-held
  [reason data]
  (throw (ex-info (str "held: " (name reason))
                  (cond-> (assoc data :reason reason)
                    ;; catchability crosses the bridge: a delegated builtin
                    ;; that held with throw/assert stays catchable by the
                    ;; lowered tryEval, everything else propagates.
                    (contains? #{:throw-builtin-called :assertion-failed}
                               reason)
                    (assoc :pnix/catchable true)))))

(defn throw-value
  "`throw` as a lowered VALUE (bare var / alias position) — the catchable
  error class, exactly what the lowered tryEval catches."
  [m]
  (let [m (force-slot m)]
    (if (or (string? m) (evaluator/ctx-string? m))
      (throw (ex-info (evaluator/string-content m)
                      {:pnix/catchable true
                       :reason :throw-builtin-called}))
      (throw (ex-info "throw argument is not a string"
                      {:reason :throw-argument-not-string})))))

(defn abort-value
  "`abort` as a lowered VALUE — NOT catchable (Nix tryEval never catches
  abort)."
  [m]
  (let [m (force-slot m)]
    (if (or (string? m) (evaluator/ctx-string? m))
      (throw (ex-info (evaluator/string-content m) {}))
      (throw (ex-info "abort argument is not a string"
                      {:reason :abort-argument-not-string})))))

(defn- result->lowered
  [r]
  (if (= :ok (:status r))
    (eval-value->lowered (:value r))
    (throw-held (or (:reason r) :bridge-eval-held) (dissoc r :status))))

(defn lowered-value->eval
  [v]
  (cond
    (delay? v)
    (evaluator/make-value-thunk
     :lowered-slot
     (fn []
       (try
         {:status :ok :value (lowered-value->eval (force v))}
         (catch clojure.lang.ExceptionInfo e
           (if (:pnix-fuel-exhausted (ex-data e))
             (throw e)
             (let [data (ex-data e)]
               (cond-> {:status :failed
                        :reason (or (:reason data) :lowered-slot-threw)}
                 (:pnix/catchable data)
                 (assoc :pnix/catchable true)))))
         (catch Throwable _
           {:status :failed
            :reason :lowered-slot-threw
            :error {:phase :lowering
                    :class :lowered-slot-failed}}))))

    (fn? v)
    (evaluator/lazy-host-fn
     :lowered-fn
     (fn [arg]
       (try
         {:status :ok :value (lowered-value->eval (v (eval-value->lowered arg)))}
         (catch clojure.lang.ExceptionInfo e
           (if (:pnix-fuel-exhausted (ex-data e))
             (throw e)
             (let [data (ex-data e)]
               (cond-> {:status :failed
                        :reason (or (:reason data) :lowered-fn-threw)}
                 (:pnix/catchable data)
                 (assoc :pnix/catchable true)))))
         (catch Throwable _
           {:status :failed
            :reason :lowered-fn-threw
            :error {:phase :lowering
                    :class :lowered-function-failed}}))))

    (vector? v)
    (mapv lowered-value->eval v)

    (map? v)
    (reduce-kv (fn [m k x] (assoc m k (lowered-value->eval x))) {} v)

    :else v))

(defn nix-binary
  "Run a strict pnix binary operator through the evaluator's checked value
  algebra while preserving lazy lowered slots through the value bridge."
  [operator lhs rhs]
  (result->lowered
   (evaluator/binary-value-result operator
                                  (lowered-value->eval lhs)
                                  (lowered-value->eval rhs))))

(defn- nix-less
  [lhs rhs]
  (let [lhs (force-slot lhs)
        rhs (force-slot rhs)]
    (if (and (vector? lhs) (vector? rhs))
      ;; Keep list slots in the lowered value model so shared function/thunk
      ;; identity survives. Scalar leaves still delegate to the evaluator.
      (loop [xs lhs ys rhs]
        (cond
          (empty? xs) (not (empty? ys))
          (empty? ys) false
          (nix-equal (first xs) (first ys) true)
          (recur (rest xs) (rest ys))
          :else (nix-less (first xs) (first ys))))
      (nix-binary "<" lhs rhs))))

(defn nix-order
  "Nix ordering, including its partial NaN order and identity-aware list
  lexicography."
  [operator lhs rhs]
  (case operator
    "<" (nix-less lhs rhs)
    ">" (nix-less rhs lhs)
    "<=" (not (nix-less rhs lhs))
    ">=" (not (nix-less lhs rhs))))

(defn nix-neg
  "Checked unary negation through the evaluator value algebra."
  [value]
  (result->lowered
   (evaluator/neg-value-result
    (lowered-value->eval (force-slot value)))))

(defn eval-value->lowered
  [v]
  (cond
    (evaluator/value-thunk? v)
    (lazy-slot
     (fn []
       (let [r (evaluator/force-whnf v)]
         (if (= :ok (:status r))
           (eval-value->lowered (:value r))
           (throw-held (or (:reason r) :bridge-eval-held)
                       (dissoc r :status))))))

    (and (map? v) (#{:builtin :closure :host-fn :lazy-host-fn} (:kind v)))
    (let [f (fn [a]
              (result->lowered
               (evaluator/apply-callable v (lowered-value->eval a))))]
      ;; pattern closures keep their functionArgs answer across the bridge
      (if-let [pattern (:param-pattern v)]
        (with-meta f {:pnix/function-args
                      (into {} (map (juxt :name #(some? (:default %))))
                            (:params pattern))})
        f))

    (vector? v)
    (mapv eval-value->lowered v)

    (map? v)
    (reduce-kv (fn [m k x] (assoc m k (eval-value->lowered x))) {} v)

    :else v))

(def ^:private builtins-table
  (delay
    (-> (reduce-kv (fn [m k v] (assoc m k (eval-value->lowered v)))
                   {}
                   (dissoc (get evaluator/default-env "builtins") "builtins"))
        ;; Preserve builtins.builtins without recursively materializing the
        ;; entire table while the table itself is being built.
        (assoc "builtins" (lazy-slot builtins-attrset))
        ;; lane override: lowered fns carry :pnix/function-args METADATA the
        ;; evaluator cannot see — the bridged builtin would silently answer
        ;; {} for lowered pattern lambdas.
        (assoc "functionArgs" function-args)
        ;; lane override: the lowered error classes (catchable vs not) live
        ;; in ex-info tags, not in bridged held results.
        (assoc "throw" throw-value)
        (assoc "abort" abort-value))))

(defn builtins-attrset
  "The full builtin set as a lowered-lane VALUE (same keys as the
  evaluator's `builtins`; every entry delegates through the lazy bridge)."
  []
  @builtins-table)

(defn has-context
  [v]
  (evaluator-builtin "hasContext" (force-normal v)))

(defn get-context
  [v]
  (evaluator-builtin "getContext" (force-normal v)))

(defn discard-string-context
  [v]
  (evaluator-builtin "unsafeDiscardStringContext" (force-normal v)))

(defn append-context
  [s ctx]
  (evaluator-builtin "appendContext" (force-normal s) (force-normal ctx)))

(defn list-length
  [value]
  (let [value (force-slot value)]
    (if (vector? value)
      (count value)
      (throw (ex-info "length argument is not a list"
                      {:reason :length-not-list
                       :value value})))))

(defn list-head
  [value]
  (let [value (force-slot value)]
    (cond
      (not (vector? value))
      (throw (ex-info "head argument is not a list"
                      {:reason :head-not-list
                       :value value}))

      (empty? value)
      (throw (ex-info "head of empty list"
                      {:reason :head-of-empty-list}))

      :else
      (force-slot (first value)))))

(defn list-tail
  [value]
  (let [value (force-slot value)]
    (cond
      (not (vector? value))
      (throw (ex-info "tail argument is not a list"
                      {:reason :tail-not-list
                       :value value}))

      (empty? value)
      (throw (ex-info "tail of empty list"
                      {:reason :tail-of-empty-list}))

      :else
      (vec (rest value)))))

(defn list-last
  [value]
  (let [value (force-slot value)]
    (cond
      (not (vector? value))
      (throw (ex-info "last argument is not a list"
                      {:reason :last-not-list
                       :value value}))

      (empty? value)
      (throw (ex-info "last of empty list"
                      {:reason :last-of-empty-list}))

      :else
      (force-slot (last value)))))

(defn list-init
  [value]
  (let [value (force-slot value)]
    (cond
      (not (vector? value))
      (throw (ex-info "init argument is not a list"
                      {:reason :init-not-list
                       :value value}))

      (empty? value)
      (throw (ex-info "init of empty list"
                      {:reason :init-of-empty-list}))

      :else
      (vec (butlast value)))))

(defn unique-list
  [value]
  (let [value (force-slot value)]
    (if-not (vector? value)
      (throw (ex-info "unique argument is not a list"
                      {:reason :unique-arg-not-list
                       :value value}))
      (loop [remaining value
             out []]
        (if (seq remaining)
          (let [item (first remaining)
                seen? (some #(nix-equal item %) out)]
            (recur (rest remaining)
                   (if seen? out (conj out item))))
          out)))))

(defn find-value
  [needle value]
  (let [value (force-slot value)]
    (if-not (vector? value)
      (throw (ex-info "find second argument is not a list"
                      {:reason :find-arg-not-list
                       :value value}))
      (loop [remaining value]
        (if (seq remaining)
          (let [item (first remaining)]
            (if (nix-equal item needle)
              (force-slot item)
              (recur (rest remaining))))
          nil)))))

(defn substring
  [start length s]
  (let [s (str s)
        slen (count s)
        start* (-> start (max 0) (min slen))
        end (if (neg? length)
              slen
              (min slen (+ start length)))
        end* (max start* end)]
    (subs s start* end*)))

(defn has-prefix?
  [prefix s]
  (let [prefix (str prefix)
        s (str s)
        n (count prefix)]
    (= prefix (subs s 0 (min n (count s))))))

(defn has-suffix?
  [suffix s]
  (let [suffix (str suffix)
        s (str s)
        n (count suffix)
        start (max 0 (- (count s) n))]
    (= suffix (subs s start))))

(defn has-infix?
  [needle s]
  (.contains (str s) (str needle)))

(defn remove-prefix
  [prefix s]
  (let [prefix (str prefix)
        s (str s)]
    (if (has-prefix? prefix s)
      (subs s (count prefix))
      s)))

(defn remove-suffix
  [suffix s]
  (let [suffix (str suffix)
        s (str s)]
    (if (has-suffix? suffix s)
      (subs s 0 (- (count s) (count suffix)))
      s)))

(defn base-name-of
  [value]
  (let [s (if (path-value? value) (get value "path") (str value))
        i (.lastIndexOf s "/")]
    (if (neg? i)
      s
      (subs s (inc i)))))

(defn dir-of
  [value]
  (let [path? (path-value? value)
        s (if path? (get value "path") (str value))
        i (.lastIndexOf s "/")]
    ((if path?
       (fn [s] {"__pnix_value_kind" "path" "path" s})
       identity)
     (cond
       (neg? i) "."
       (zero? i) "/"
       :else (subs s 0 i)))))

(defn attr-by-path
  [path default attrs]
  (loop [remaining (force-normal path)
         current attrs]
    (if-let [part (first remaining)]
      (let [current (force-slot current)]
        (if (and (map? current) (contains? current part))
          (recur (rest remaining) (get current part))
          default))
      (force-slot current))))

(defn string-to-characters
  [s]
  (mapv str (str s)))

(defn lower-case
  [s]
  (.toLowerCase (str s) java.util.Locale/ROOT))

(defn upper-case
  [s]
  (.toUpperCase (str s) java.util.Locale/ROOT))

(defn split-string
  [sep s]
  (let [sep (str sep)
        s (str s)
        sep-len (count sep)]
    (if (zero? sep-len)
      (string-to-characters s)
      (loop [start 0
             out []]
        (let [idx (.indexOf s sep start)]
          (if (neg? idx)
            (conj out (subs s start))
            (recur (+ idx sep-len)
                   (conj out (subs s start idx)))))))))

(defn regex-match
  [pattern s]
  (let [m (re-matches (evaluator/nix-regex-pattern pattern) (str s))]
    (cond
      (nil? m) nil
      (vector? m) (vec (rest m))
      :else [])))

(defn regex-split
  [pattern s]
  (let [s (str s)
        m (re-matcher (evaluator/nix-regex-pattern pattern) s)]
    (loop [result []
           last-end 0]
      (if (.find m)
        (let [start (.start m)
              end (.end m)
              groups (mapv #(.group m %)
                           (range 1 (inc (.groupCount m))))]
          (recur (conj result (subs s last-end start) groups) end))
        (conj result (subs s last-end))))))

(defn to-int
  [value]
  (Long/parseLong (.trim (str value))))

(defn path-value?
  [value]
  (and (map? value)
       (= "path" (get value "__pnix_value_kind"))
       (string? (get value "path"))))

(defn attrset-value?
  [value]
  (and (map? value)
       (not (path-value? value))
       (not (contains? #{:builtin :closure :path :thunk} (:kind value)))))

(declare coerce-to-string)

(defn plus
  "Lowered `+`: same semantics as the evaluator's binary + — strings (plain or
  contextful) concatenate with context union via the evaluator's own
  ctx-string constructor/accessors (zero drift); numbers add. String +
  non-string is a TYPE error (R2 Phase D: the old silent coercion was a
  Clojure host leak, removed 2026-07-07)."
  [l r]
  (let [l (force-slot l)
        r (force-slot r)
        string-like? #(or (string? %) (evaluator/ctx-string? %))]
    (cond
      (and (string-like? l) (string-like? r))
      (evaluator/ctx-string
       (str (evaluator/string-content l) (evaluator/string-content r))
       (concat (evaluator/string-ctx l) (evaluator/string-ctx r)))

      (or (string-like? l) (string-like? r))
      (throw (ex-info "string + non-string is a type error"
                      {:pnix-strict :string-coercion}))

      :else (+ l r))))

(defn require-bool
  "Boolean-position guard for lowered if/assert/! (R2 Phase D): pnix has no
  truthiness — a non-boolean here is a TYPE error exactly like the host
  evaluator's :non-bool-* holds and real Nix's 'expected a Boolean'."
  [v construct]
  (let [v (force-slot v)]
    (if (boolean? v)
      v
      (throw (ex-info (str "non-boolean " construct " position")
                      {:pnix-strict :non-bool
                       :construct construct})))))

(defn template-join
  "Join realized string-template parts: contents concatenate, contexts union
  (a context-free template stays a plain String), mirroring the evaluator's
  template joiner through the same shared accessors."
  [parts]
  (evaluator/ctx-string
   (apply str (map evaluator/string-content parts))
   (into [] (mapcat evaluator/string-ctx) parts)))

(defn coerce-to-string
  [value]
  (let [value (force-normal value)]
    (cond
      (string? value) value
      ;; toString keeps context (evaluator semantics)
      (evaluator/ctx-string? value) value
      (true? value) "1"
      (false? value) ""
      (nil? value) ""
      (path-value? value) (get value "path")
      (integer? value) (str value)
      ;; float coercion must match the evaluator lane's Nix %.6f (D4)
      (float? value) (evaluator/nix-float-str value)
      (number? value) (str value)
      (vector? value) (str/join " " (map coerce-to-string value))
      (and (attrset-value? value) (contains? value "outPath"))
      (coerce-to-string (get value "outPath"))
      :else
      (throw (ex-info "cannot coerce lowered value to string"
                      {:value value})))))

(defn interpolate-to-string
  [value]
  (let [value (force-slot value)]
    (cond
      (string? value) value
      ;; a contextful string interpolates as itself; template-join unions
      (evaluator/ctx-string? value) value
      (path-value? value) (let [path (get value "path")]
                            (evaluator/ctx-string path [path]))

      (attrset-value? value)
      (cond
        (contains? value "__toString")
        (interpolate-to-string ((force-slot (get value "__toString")) value))

        (contains? value "outPath")
        (interpolate-to-string (get value "outPath"))

        :else
        (throw (ex-info "cannot interpolate lowered value as string"
                        {:value value
                         :accepted [:string :path :attrset-__toString
                                    :attrset-outPath]})))

      :else
      (throw (ex-info "cannot interpolate lowered value as string"
                      {:value value
                       :accepted [:string :path :attrset-__toString
                                  :attrset-outPath]})))))

(defn type-of
  [value]
  (cond
    (nil? value) "null"
    (path-value? value) "path"
    (string? value) "string"
    (or (true? value) (false? value)) "bool"
    (integer? value) "int"
    (float? value) "float"
    (vector? value) "list"
    (attrset-value? value) "set"
    (fn? value) "lambda"
    :else "unknown"))

(defn inclusive-range
  [from to]
  (if (> from to)
    []
    (vec (range from (inc to)))))

(defn recursive-update
  [lhs rhs]
  (let [lhs (force-slot lhs)
        rhs (force-slot rhs)]
    (if (and (map? lhs) (map? rhs))
      (reduce (fn [out k]
                (assoc out k
                       (if (and (contains? lhs k) (contains? rhs k))
                         (recursive-update (get lhs k) (get rhs k))
                         (get rhs k))))
              lhs
              (keys rhs))
      rhs)))

(defn nix-equal
  ([lhs rhs]
   (nix-equal lhs rhs false))
  ([lhs rhs recursive-slot?]
   (let [lhs (force-slot lhs)
         rhs (force-slot rhs)]
     (cond
      (and recursive-slot? (identical? lhs rhs))
      true

      (and (number? lhs) (number? rhs))
      (== lhs rhs)

      (or (path-value? lhs) (path-value? rhs))
      (and (path-value? lhs)
           (path-value? rhs)
           (= (get lhs "path") (get rhs "path")))

      (or (fn? lhs) (fn? rhs))
      false

      (and (vector? lhs) (vector? rhs))
      (and (= (count lhs) (count rhs))
           (loop [xs (seq lhs)
                  ys (seq rhs)]
             (if xs
               (and (nix-equal (first xs) (first ys) true)
                    (recur (next xs) (next ys)))
               true)))

      (and (attrset-value? lhs) (attrset-value? rhs))
      (let [lhs-keys (set (keys lhs))
            rhs-keys (set (keys rhs))]
        (and (= lhs-keys rhs-keys)
             (every? (fn [k]
                       (nix-equal (get lhs k) (get rhs k) true))
                     lhs-keys)))

      :else
      (= lhs rhs)))))

(defn generic-closure
  [spec]
  (let [spec (force-slot spec)
        operator (force-slot (get spec "operator"))
        start-set (force-slot (get spec "startSet"))]
    (loop [worklist (vec start-set)
           seen #{}
           out []]
      (if (seq worklist)
        (let [item (force-slot (first worklist))
              rest-work (vec (rest worklist))
              key (force-normal (get item "key"))]
          (if (contains? seen key)
            (recur rest-work seen out)
            (let [next-items (force-slot (operator item))]
              (recur (into rest-work next-items)
                     (conj seen key)
                     (conj out item)))))
        out))))

(defn list-to-attrs
  [xs]
  (reduce (fn [out row]
            (let [row (force-slot row)
                  k (force-normal (get row "name"))]
              (if (contains? out k)
                out
                (assoc out k (get row "value")))))
          {}
          xs))

(defn replace-strings
  [from to s]
  (let [pairs (mapv vector from to)
        s (str s)
        n (count s)]
    (loop [i 0
           out (StringBuilder.)]
      (if (<= i n)
        (if-let [[needle replacement]
                 (some (fn [[needle replacement]]
                         (let [needle (str needle)
                               nl (count needle)]
                           (when (and (<= (+ i nl) n)
                                      (= needle (subs s i (+ i nl))))
                             [needle replacement])))
                       pairs)]
          (if (zero? (count needle))
            (do (.append out (str replacement))
                (if (< i n)
                  (recur (inc i) (.append out (.charAt s i)))
                  (.toString out)))
            (recur (+ i (count needle))
                   (.append out (str replacement))))
          (if (< i n)
            (recur (inc i) (.append out (.charAt s i)))
            (.toString out)))
        (.toString out)))))

(defn- lower-items
  [items]
  (loop [remaining items
         forms []]
    (if-let [item (first remaining)]
      (let [result (lower-ast item)]
        (if (= :ok (:status result))
          (recur (rest remaining) (conj forms (:form result)))
          result))
      (ok-form (mapv lazy-slot-form forms)))))

(defn attr-key-string
  "D20 runtime check for a lowered DYNAMIC attr key: must be a string (the
  evaluator's shared attr-key-value-result — real Nix errors instead of
  coercing). Plain ex-info: NOT :pnix/catchable, and an `or` default does not
  catch it either (the throw happens before any select logic)."
  [v]
  (let [r (evaluator/attr-key-value-result (force-slot v))]
    (if (= :ok (:status r))
      (:value r)
      (throw (ex-info "dynamic attr key is not a string"
                      {:reason (:reason r)})))))

(defn attrset-pairs
  "D20 runtime attrset builder for lowered dynamic-key attrsets: an eval-time
  key collision is an ERROR (real Nix: dynamic attribute already defined),
  never a silent overwrite."
  [kvs]
  (reduce (fn [m [k v]]
            (if (contains? m k)
              (throw (ex-info (str "duplicate attribute " k)
                              {:reason :duplicate-attr :attr k}))
              (assoc m k v)))
          {}
          (partition 2 kvs)))

(defn- lower-attr-key
  [key]
  (cond
    (string? key)
    {:status :ok
     :form key
     :dynamic? false}

    (= :dynamic-attr-key (:kind key))
    (let [result (lower-ast (:expr key))]
      (if (= :ok (:status result))
        {:status :ok
         :form (list `attr-key-string (:form result))
         :dynamic? true}
        result))

    :else
    {:status :failed
     :reason :unsupported-lowering-attr-key
     :key key}))

(defn- lower-attrs
  [attrs]
  (loop [remaining attrs
         literal-forms {}
         pair-forms []
         dynamic? false]
    (if-let [{:keys [key value]} (first remaining)]
      (let [key-result (lower-attr-key key)]
        (if (not= :ok (:status key-result))
          key-result
          (let [value-result (lower-ast value)]
            (if (= :ok (:status value-result))
              (recur (rest remaining)
                     (assoc literal-forms
                            (:form key-result)
                            (lazy-slot-form (:form value-result)))
                     (conj pair-forms
                           (:form key-result)
                           (lazy-slot-form (:form value-result)))
                     (or dynamic? (:dynamic? key-result)))
              value-result))))
      (if dynamic?
        ;; D20: dup-checking builder, never bare hash-map (which would throw
        ;; its own duplicate-key error with the wrong shape — and a silent
        ;; overwrite would be worse).
        (ok-form (list `attrset-pairs (vec pair-forms)))
        (ok-form literal-forms)))))

(defn- lower-recursive-attrs
  [attrs]
  (let [static-keys (keep (fn [{:keys [key]}]
                            (when (string? key)
                              (symbol key)))
                          attrs)]
    (binding [*lexical-vars* (into *lexical-vars* static-keys)]
      (loop [remaining attrs
             pairs []
             literal-forms {}]
        (if-let [{:keys [key value]} (first remaining)]
          (let [key-result (lower-attr-key key)]
            (cond
              (not= :ok (:status key-result))
              key-result

              (:dynamic? key-result)
              {:status :failed
               :reason :recursive-dynamic-attr-key-lowering-not-wired
               :key key}

              :else
              (let [value-result (lower-ast value)]
                (if (= :ok (:status value-result))
                  (let [sym (symbol (:form key-result))]
                    (recur (rest remaining)
                           (conj pairs sym (:form value-result))
                           (assoc literal-forms
                                  (:form key-result)
                                  (lazy-slot-form sym))))
                  value-result))))
          (ok-form (list 'pnix.clj-meta.compiler/lazy-letrec
                         (vec pairs)
                         literal-forms)))))))

(def ^:private bare-default-scope-builtins
  "Nix binds a fixed subset of builtins unprefixed at the top level (the
  evaluator's default-env does the same). An UNSHADOWED bare reference is the
  same callable as its builtins.X select; `let` shadows it, `with` does NOT
  (oracle + host agree: the static base scope wins — D13). throw/abort keep
  their dedicated :var handling; import/scopedImport stay held."
  {"map" :map
   "toString" :toString
   "isNull" :isNull
   "baseNameOf" :baseNameOf
   "dirOf" :dirOf
   "removeAttrs" :removeAttrs})

(defn- builtin-select-name
  [ast]
  (cond
    (and (= :select (:op ast))
         (= :var (get-in ast [:target :op]))
         (= "builtins" (get-in ast [:target :name]))
         (not (contains? *lexical-vars* 'builtins)))
    (keyword (:attr ast))

    ;; D13: a bare default-scope builtin, not lexically shadowed, behaves
    ;; exactly like its builtins.X select in every call special-case.
    (and (= :var (:op ast))
         (contains? bare-default-scope-builtins (:name ast))
         (not (contains? *lexical-vars* (symbol (:name ast)))))
    (get bare-default-scope-builtins (:name ast))

    :else nil))

(defn- builtin-constant-form
  [ast]
  (case (builtin-select-name ast)
    :currentSystem `pnix-clj.version/current-system
    :nixVersion `pnix-clj.version/nix-version
    :storeDir `pnix-clj.version/store-dir
    ;; bare `builtins.functionArgs` as a VALUE (alias/argument position):
    ;; the runtime helper var IS the function, so it survives `let g = ...`
    ;; where the generic select leaves a free `builtins` symbol the host
    ;; lane cannot execute.
    :functionArgs `function-args
    nil))

(defn- call-chain
  [ast]
  (loop [node ast
         args []]
    (if (= :call (:op node))
      (recur (:fn node) (conj args (:arg node)))
      {:callee node
       :args (vec (reverse args))})))

(defn- lower-string-template
  [parts]
  (loop [remaining parts
         forms []]
    (if-let [part (first remaining)]
      (case (:kind part)
        :text
        (recur (rest remaining) (conj forms (:value part)))

        :expr
        (let [result (lower-ast (:expr part))]
          (if (= :ok (:status result))
            (recur (rest remaining)
                   (conj forms (list `interpolate-to-string (:form result))))
            result)))
      (ok-form (list `template-join (vec forms))))))

(defn- lower-args
  [args]
  (loop [remaining args
         forms []]
    (if-let [arg (first remaining)]
      (let [result (lower-ast arg)]
        (if (= :ok (:status result))
          (recur (rest remaining) (conj forms (:form result)))
          result))
      {:status :ok
       :forms forms})))

(defn- lower-import-target
  "Lower an imported module HERMETICALLY: its free vars are its own globals,
  not the importing context's lexical bindings, so the lexical/with scopes are
  reset. `scope-syms` (scopedImport only) are the sole lexical names injected —
  they apply to THIS module only and are NOT inherited by nested plain imports
  (which recurse through here with scope-syms nil), matching Nix's
  non-propagating scopedImport."
  ([target]
   (lower-import-target target nil))
  ([target scope-renames]
   (let [chain (vec *import-context*)
         target (evaluator/contextual-import-target chain target)
         scope-keys (set (keys scope-renames))]
     (cond
       (empty? *import-modules*)
       {:status :failed
        :reason :import-lowering-not-wired
        :target target}

       (some #(= target %) chain)
       {:status :failed
        :reason :import-cycle
        :target target
        :chain (conj chain target)}

       (contains? *import-modules* target)
       (let [{:keys [status ast] :as parsed}
             (parser/parse-source (get *import-modules* target))]
         (if (= :ok status)
           (binding [*import-context* (conj chain target)
                     *lexical-vars* scope-keys
                     *force-on-read-vars* scope-keys
                     *lexical-renames* (or scope-renames {})
                     *with-scope-syms* []]
             (lower-ast ast))
           parsed))

       :else
       {:status :failed
        :reason :import-module-not-found
        :target target}))))

(defn- literal-import-target
  [arg]
  (when (#{:path :string} (:op arg))
    (:value arg)))

(defn- lower-scoped-import
  [args]
  ;; Nix `scopedImport scope path`: scope FIRST, path SECOND (verified against
  ;; nix-instantiate 2.34.7). The scope attrs are added on top of the global
  ;; env for the imported module. We inject them by binding the scope keys as
  ;; lexical (force-on-read) names around the module lowering and emitting a
  ;; zero-shadow function application `((fn [k..] module) v-slot..)`: the value
  ;; slots are lowered in the CALLER context (so a scope value referencing
  ;; another scope key sees the caller binding, NOT the scope one — matching
  ;; the direct lane's forced-attrset merge, which is non-recursive), and the
  ;; module sees the keys as parameters. Laziness is preserved (unused scope
  ;; keys with errors do not fail). Lexical `builtins` follows the same
  ;; injection path as every other scope key.
  (if (not= 2 (count args))
    {:status :failed
     :reason :scoped-import-arity-mismatch
     :expected 2
     :actual (count args)}
    (let [scope (first args)
          target (second args)
          target* (literal-import-target target)]
      (cond
        (not= :attrset (:op scope))
        {:status :failed
         :reason :scoped-import-scope-not-attrset-literal
         :scope scope}

        (nil? target*)
        {:status :failed
         :reason :scoped-import-target-not-literal
         :target target}

        (empty? (:attrs scope))
        (lower-import-target target*)

        :else
        (let [scope-attrs (:attrs scope)
              ;; munge each scope key to a collision-proof parameter symbol
              ;; (`x` -> `x*scope`; `*` is not a legal pnix identifier char), so
              ;; the injecting `fn` cannot lexically capture a same-named free
              ;; var inlined from a nested import — those stay bare and unbound
              ;; (held), matching the direct lane's non-propagating scope.
              scope-renames (into {} (map (fn [{:keys [key]}]
                                            [(symbol key) (symbol (str key "*scope"))])
                                          scope-attrs))
              param-syms (mapv #(get scope-renames (symbol (:key %))) scope-attrs)
              ;; scope VALUES are evaluated at the call site -> lowered in the
              ;; CALLER context (current lexical scope), so a value naming
              ;; another scope key sees the caller binding, matching the direct
              ;; lane's non-recursive merge.
              value-results (mapv #(lower-ast (:value %)) scope-attrs)
              bad-value (first (filter #(not= :ok (:status %)) value-results))]
          (if bad-value
            bad-value
            ;; the module is lowered hermetically with ONLY the scope keys as
            ;; renamed lexical names (lower-import-target resets the rest), so
            ;; nested plain imports do NOT inherit the scope.
            (let [module-result (lower-import-target target* scope-renames)]
              (if (not= :ok (:status module-result))
                module-result
                (ok-form
                 (list* (list 'fn param-syms (:form module-result))
                        (mapv #(lazy-slot-form (:form %)) value-results)))))))))))

(defn- lower-builtin-call
  [ast]
  (let [builtin-name (builtin-select-name (:fn ast))
        {:keys [callee args]} (call-chain ast)
        chain-builtin-name (builtin-select-name callee)]
    (cond
      (and (= :var (:op callee))
           (= "scopedImport" (:name callee)))
      (lower-scoped-import args)

      (= :getAttr builtin-name)
      {:status :failed
       :reason :get-attr-lowering-not-wired
       :builtin "getAttr"
       :policy :recursive-attrset-access}

      (= :getEnv builtin-name)
      {:status :failed
       :reason :get-env-purity-gated
       :builtin "getEnv"
       :effect :env-read
       :policy :pure-evaluator-no-host-environment}

      (= :pathExists builtin-name)
      {:status :failed
       :reason :path-exists-purity-gated
       :builtin "pathExists"
       :effect :path-exists
       :policy :pure-evaluator-no-host-filesystem}

      (= :pnixMounts builtin-name)
      {:status :failed
       :reason :pnix-mounts-extension-not-wired
       :nix-builtin? false
       :extension :pnix-mount-runtime
       :policy :non-faithful-extension-not-nix-coverage}

      (= :readDir builtin-name)
      {:status :failed
       :reason :read-dir-purity-gated
       :builtin "readDir"
       :effect :directory-read
       :policy :pure-evaluator-no-host-filesystem}

      (= :readFile builtin-name)
      {:status :failed
       :reason :read-file-purity-gated
       :builtin "readFile"
       :effect :file-read
       :policy :pure-evaluator-no-host-filesystem}

      (= :unsafeGetAttrPos builtin-name)
      {:status :failed
       :reason :unsafe-get-attr-pos-lowering-not-wired
       :nix-builtin? true
       :policy :unsafe-builtin-trace-frontier}

      (= :attrNames builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'vec (list 'sort (list 'keys (:form arg-result)))))
          arg-result))

      (= :toString builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `coerce-to-string (:form arg-result)))
          arg-result))

      (= :toJSON builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'pnix-clj.json/write-json
                         (force-normal-form (:form arg-result))))
          arg-result))

      (= :fromJSON builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'pnix-clj.json/read-json (:form arg-result)))
          arg-result))

      (= :concatLists builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'vec
                         (list 'apply
                               'concat
                               (list 'mapv
                                     `force-slot
                                     (force-slot-form (:form arg-result))))))
          arg-result))

      (= :concatStrings builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'apply
                         'str
                         (force-normal-form (:form arg-result))))
          arg-result))

      (= :reverseList builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'vec
                         (list 'reverse
                               (force-slot-form (:form arg-result)))))
          arg-result))

      (= :keys builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'vec
                         (list 'sort
                               (list 'keys (:form arg-result)))))
          arg-result))

      (contains? #{:values :attrValues} builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'mapv
                         (:form arg-result)
                         (list 'sort
                               (list 'keys (:form arg-result)))))
          arg-result))

      (= :flatten builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'letfn
                         [(list 'flatten-list ['value]
                                (list 'if
                                      (list 'vector? 'value)
                                      (list 'mapcat 'flatten-list 'value)
                                      ['value]))]
                         (list 'vec
                               (list 'flatten-list
                                     (force-normal-form (:form arg-result))))))
          arg-result))

      (= :splitVersion builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'pnix-clj.version/split-version (:form arg-result)))
          arg-result))

      (= :parseDrvName builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'pnix-clj.version/parse-drv-name (:form arg-result)))
          arg-result))

      (= :throw builtin-name)
      ;; `throw` is the CATCHABLE error class (with assert) — tagged so the
      ;; lowered tryEval catches exactly what Nix catches.
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `throw-value (:form arg-result)))
          arg-result))

      (= :abort builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `abort-value (:form arg-result)))
          arg-result))

      (= :tryEval builtin-name)
      ;; Nix tryEval catches ONLY throw/assert; division by zero, missing
      ;; attrs, aborts, ... propagate. The old `catch Throwable` answered
      ;; { success = false; } for ALL of them — a silent-wrong the direct
      ;; evaluator (held) exposed.
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'try
                         {"success" true
                          "value" (:form arg-result)}
                         (list 'catch 'clojure.lang.ExceptionInfo 'e#
                               (list 'if (list :pnix/catchable
                                               (list 'clojure.core/ex-data 'e#))
                                     {"success" false
                                      "value" false}
                                     (list 'throw 'e#)))))
          arg-result))

      (= :functionArgs builtin-name)
      ;; builtins always report {} (evaluator semantics); a direct builtins.X
      ;; reference is constant-folded because its generic lowering leaves a
      ;; free `builtins` symbol the host lane cannot execute.
      (if (builtin-select-name (:arg ast))
        (ok-form {})
        (let [arg-result (lower-ast (:arg ast))]
          (if (= :ok (:status arg-result))
            (ok-form (list `function-args (:form arg-result)))
            arg-result)))

      (= :not builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'not (:form arg-result)))
          arg-result))

      (= :boolToString builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'if (:form arg-result) "true" "false"))
          arg-result))

      (= :baseNameOf builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `base-name-of (:form arg-result)))
          arg-result))

      (= :dirOf builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `dir-of (:form arg-result)))
          arg-result))

      (= :stringToCharacters builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `string-to-characters (:form arg-result)))
          arg-result))

      (= :toLower builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `lower-case (:form arg-result)))
          arg-result))

      (= :toUpper builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `upper-case (:form arg-result)))
          arg-result))

      (contains? #{:isString :isAttrs :isList :isFunction :isFloat
                   :isBool :isNull :isInt :typeOf}
                 builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (let [a (:form arg-result)]
            (ok-form
             (case builtin-name
               :isString (list 'string? a)
               :isAttrs (list `attrset-value? a)
               :isList (list 'vector? a)
               :isFunction (list 'fn? a)
               :isFloat (list 'float? a)
               :isBool (list 'or (list 'true? a) (list 'false? a))
               :isNull (list 'nil? a)
               :isInt (list 'integer? a)
               :typeOf (list `type-of a))))
          arg-result))

      (= :isPath builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `path-value? (:form arg-result)))
          arg-result))

      (= :last builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `list-last (:form arg-result)))
          arg-result))

      (= :init builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `list-init (:form arg-result)))
          arg-result))

      (= :unique builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `unique-list (:form arg-result)))
          arg-result))

      (= :id builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (:form arg-result))
          arg-result))

      (= :toInt builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `to-int (:form arg-result)))
          arg-result))

      (= :genericClosure builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `generic-closure (:form arg-result)))
          arg-result))

      (= :listToAttrs builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `list-to-attrs (force-slot-form (:form arg-result))))
          arg-result))

      (= :neg builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `host-builtin "neg" (:form arg-result)))
          arg-result))

      (= :abs builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `host-builtin "abs" (:form arg-result)))
          arg-result))

      (= :sqrt builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'Math/sqrt (list 'double (:form arg-result))))
          arg-result))

      (contains? #{:floor :ceil} builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          ;; Delegate exact NixInt -> f64 and f64 -> i64 checks to the
          ;; evaluator's single builtin implementation. Direct Math calls
          ;; saturated out-of-range values and diverged in the compiled lane.
          (ok-form (list `host-builtin
                         (name builtin-name)
                         (:form arg-result)))
          arg-result))

      (= :exp builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'Math/exp (list 'double (:form arg-result))))
          arg-result))

      (= :ln builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'Math/log (list 'double (:form arg-result))))
          arg-result))

      (= :sin builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'Math/sin (list 'double (:form arg-result))))
          arg-result))

      (= :cos builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'Math/cos (list 'double (:form arg-result))))
          arg-result))

      (= :head builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `list-head (:form arg-result)))
          arg-result))

      (= :length builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `list-length (:form arg-result)))
          arg-result))

      (contains? #{:derivation :derivationStrict :placeholder} builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `host-builtin
                         (name builtin-name)
                         (:form arg-result)))
          arg-result))

      (= :storePath builtin-name)
      {:status :failed
       :reason :store-path-purity-gated
       :builtin "storePath"
       :effect :store-read
       :policy :pure-evaluator-no-store}

      (contains? #{:hasContext :getContext :unsafeDiscardStringContext}
                 builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list (case builtin-name
                           :hasContext `has-context
                           :getContext `get-context
                           :unsafeDiscardStringContext `discard-string-context)
                         (:form arg-result)))
          arg-result))

      (= :stringLength builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list 'count (:form arg-result)))
          arg-result))

      (= :tail builtin-name)
      (let [arg-result (lower-ast (:arg ast))]
        (if (= :ok (:status arg-result))
          (ok-form (list `list-tail (:form arg-result)))
          arg-result))

      (and (= :concatStringsSep chain-builtin-name)
           (= 2 (count args)))
      (let [sep-result (lower-ast (first args))]
        (if (not= :ok (:status sep-result))
          sep-result
          (let [coll-result (lower-ast (second args))]
            (if (= :ok (:status coll-result))
              (ok-form (list 'apply
                             'str
                             (list 'interpose
                                   (:form sep-result)
                                   (force-normal-form (:form coll-result)))))
              coll-result))))

      (and (= :substring chain-builtin-name)
           (= 3 (count args)))
      (let [start-result (lower-ast (nth args 0))]
        (if (not= :ok (:status start-result))
          start-result
          (let [length-result (lower-ast (nth args 1))]
            (if (not= :ok (:status length-result))
              length-result
              (let [string-result (lower-ast (nth args 2))]
                (if (= :ok (:status string-result))
                  ;; Delegate byte slicing to the evaluator. The old lowered
                  ;; helper sliced JVM characters and could never produce the
                  ;; PxBytes value that hashString must hash losslessly.
                  (ok-form (list `host-builtin
                                 "substring"
                                 (:form start-result)
                                 (:form length-result)
                                 (:form string-result)))
                  string-result))))))

      (and (= :replaceStrings chain-builtin-name)
           (= 3 (count args)))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[from to s] (:forms args-result)]
            (ok-form (list `replace-strings
                           (force-normal-form from)
                           (force-normal-form to)
                           s)))))

      (and (= 2 (count args))
           (contains? #{:add :sub :mul :div :lessThan
                        :mod :pow :atan2
                        :and :or :eq :lt :le :gt :ge
                        :append :take :drop :removeAttrs :concatMap
                        :get :merge :find :zip :catAttrs
                        :intersectAttrs :compareVersions
                        :bitAnd :bitOr :bitXor
                        :seq :deepSeq :trace
                        :hasPrefix :hasSuffix
                        :optional :optionals :optionalString
                        :removePrefix :removeSuffix
                        :min :max :range :replicate :hasInfix
                        :recursiveUpdate :pipe :splitString :match :split :cons
                        :mapAttrs' :genAttrs :nameValuePair :addErrorContext
                        :all :any :appendContext}
                      chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[a b] (:forms args-result)]
            (ok-form
             (case chain-builtin-name
               :add (list `host-builtin "add" a b)
               :sub (list `host-builtin "sub" a b)
               :mul (list `host-builtin "mul" a b)
               :div (list `host-builtin "div" a b)
               :mod (list `host-builtin "mod" a b)
               :pow (list 'if
                          (list 'and (list 'integer? a) (list 'integer? b))
                          (list 'long (list 'Math/pow (list 'double a) (list 'double b)))
                          (list 'Math/pow (list 'double a) (list 'double b)))
               :atan2 (list 'Math/atan2 (list 'double a) (list 'double b))
               :and (list 'boolean (list 'and a b))
               :or (list 'boolean (list 'or a b))
               :eq (list `nix-equal a b)
               :lt (list `nix-order "<" a b)
               :le (list `nix-order "<=" a b)
               :gt (list `nix-order ">" a b)
               :ge (list `nix-order ">=" a b)
               :compareVersions (list 'pnix-clj.version/compare-versions a b)
               :bitAnd (list 'bit-and a b)
               :bitOr (list 'bit-or a b)
               :bitXor (list 'bit-xor a b)
               :seq (list 'do (force-slot-form a) b)
               :deepSeq (list 'do (force-normal-form a) b)
               :trace b
               :appendContext (list `append-context a b)
               :hasPrefix (list `has-prefix? a b)
               :hasSuffix (list `has-suffix? a b)
               :optional (list 'if a [(lazy-slot-form b)] [])
               :optionals (list 'if a b [])
               :optionalString (list 'if a b "")
               :removePrefix (list `remove-prefix a b)
               :removeSuffix (list `remove-suffix a b)
               :min (list 'if (list '< a b) a b)
               :max (list 'if (list '> a b) a b)
               :range (list `inclusive-range a b)
               :replicate (list 'let ['slot (lazy-slot-form b)]
                            (list 'vec (list 'repeat a 'slot)))
               :hasInfix (list `has-infix? a b)
               :recursiveUpdate (list `recursive-update a b)
               :pipe (list 'reduce
                           (list 'fn ['acc 'f]
                                 (list 'f 'acc))
                           a
                           (force-normal-form b))
               :splitString (list `split-string a b)
               :match (list `regex-match a b)
               :split (list `regex-split a b)
               :cons (list 'vec (list 'cons a b))
               :mapAttrs' (list 'let ['attrs (force-slot-form b)]
                            (list `list-to-attrs
                                  (list 'mapv
                                        (list 'fn ['k]
                                              (list (list a 'k)
                                                    (list 'get 'attrs 'k)))
                                        (list 'sort
                                              (list 'keys 'attrs)))))
               :genAttrs (list 'into
                               {}
                               (list 'map
                                     (list 'fn ['name]
                                           ['name (lazy-slot-form
                                                  (list b 'name))])
                                     (force-normal-form a)))
               :nameValuePair {"name" a "value" b}
               :addErrorContext b
               :all (list 'every? a (force-slot-form b))
               :any (list 'boolean (list 'some a (force-slot-form b)))
               :lessThan (list `nix-order "<" a b)
               :append (list 'vec (list 'concat a b))
               :take (list 'vec (list 'take a b))
               :drop (list 'vec (list 'drop a b))
               :removeAttrs (list 'apply 'dissoc a (force-normal-form b))
               :concatMap (list 'vec (list 'mapcat a (force-slot-form b)))
               :get (force-slot-form (list 'get a b))
               :merge (list 'merge a b)
               :find (list `find-value a b)
               :zip (list 'mapv 'vector a b)
               :catAttrs (list 'reduce
                               (list 'fn ['acc 'row]
                                     (list 'let ['row (force-slot-form 'row)]
                                           (list 'if
                                                 (list 'contains? 'row a)
                                                 (list 'conj
                                                       'acc
                                                       (list 'get 'row a))
                                                 'acc)))
                               []
                               (force-slot-form b))
               :intersectAttrs (list 'select-keys
                                     b
                                     (list 'filter
                                           (list 'fn ['k]
                                                 (list 'contains? a 'k))
                                           (list 'keys b))))))))

      (and (= 3 (count args))
           (= :set chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[attrs key value] (:forms args-result)]
            (ok-form (list 'assoc attrs key value)))))

      (and (= 3 (count args))
           (= :attrByPath chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[path default attrs] (:forms args-result)]
            (ok-form (list `attr-by-path path default attrs)))))

      (and (= 3 (count args))
           (= :concatMapStringsSep chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[sep f xs] (:forms args-result)]
            (ok-form (list 'apply
                           'str
                           (list 'interpose
                                 sep
                                 (list 'map
                                       (list 'fn ['x]
                                             (list 'str (list f 'x)))
                                       (force-slot-form xs))))))))

      (and (= 3 (count args))
           (= :flip chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[f a b] (:forms args-result)]
            (ok-form (list (list f b) a)))))

      (and (= 3 (count args))
           (= :zipListsWith chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[f xs ys] (:forms args-result)]
            (ok-form (list 'let ['f (list `assert-function f)
                                  'xs (force-slot-form xs)
                                  'ys (force-slot-form ys)]
                           (list 'mapv
                                 (list 'fn ['x 'y]
                                       (lazy-slot-form
                                        (list (list 'f 'x) 'y)))
                                 'xs
                                 'ys))))))

      (and (= 3 (count args))
           (= :findFirst chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[pred default xs] (:forms args-result)]
            (ok-form (list 'reduce
                           (list 'fn ['acc 'x]
                                 (list 'if
                                       (list pred 'x)
                                       (list 'reduced 'x)
                                       'acc))
                           default
                           (force-slot-form xs))))))

      (and (= 3 (count args))
           (= :foldr chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[f z xs] (:forms args-result)]
            (ok-form (list 'reduce
                           (list 'fn ['acc 'x]
                                 (list (list f 'x) 'acc))
                           z
                           (list 'reverse (force-slot-form xs)))))))

      (and (= 3 (count args))
           (= :foldlAttrs chain-builtin-name))
      (let [args-result (lower-args args)]
        (if (not= :ok (:status args-result))
          args-result
          (let [[f init attrs] (:forms args-result)]
            (ok-form (list 'let ['attrs (force-slot-form attrs)]
                           (list 'reduce
                                 (list 'fn ['acc 'k]
                                       (list (list (list f 'acc) 'k)
                                             (list 'get 'attrs 'k)))
                                 init
                                 (list 'sort (list 'keys 'attrs))))))))

      (and (= :call (get-in ast [:fn :op]))
           (= :hasAttr (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [key-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status key-result))
          key-result
          (let [attrset-result (lower-ast (:arg ast))]
            (if (= :ok (:status attrset-result))
              (ok-form (list 'contains?
                             (:form attrset-result)
                             (:form key-result)))
              attrset-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :map (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              ;; D9 (D2 parity in this lane): an EMPTY list must not evaluate
              ;; the function argument — Nix map early-returns before
              ;; forceFunction, so the fn form moves inside the non-empty arm.
              (ok-form (list 'let ['xs (force-slot-form (:form coll-result))]
                             (list 'if (list 'empty? 'xs)
                                   []
                                   (list 'let ['f (list `assert-function (:form fn-result))]
                                         (list 'mapv
                                               (list 'fn ['x]
                                                     (lazy-slot-form (list 'f 'x)))
                                               'xs)))))
              coll-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :concatMapStrings (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              (ok-form (list 'apply
                             'str
                             (list 'map
                                   (list 'fn ['x]
                                         (list 'str (list (:form fn-result) 'x)))
                                   (force-slot-form (:form coll-result)))))
              coll-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :count (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [pred-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status pred-result))
          pred-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              (ok-form (list 'count
                             (list 'filter
                                   (:form pred-result)
                                   (force-slot-form (:form coll-result)))))
              coll-result))))

      (and (= :call (get-in ast [:fn :op]))
           (contains? #{:imap0 :imap1} (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))
            builtin-name (builtin-select-name (get-in ast [:fn :fn]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              (let [start (if (= :imap1 builtin-name) 1 0)]
                (ok-form (list 'let ['f (list `assert-function (:form fn-result))
                                      'xs (force-slot-form (:form coll-result))]
                               (list 'mapv
                                     (list 'fn ['i 'x]
                                           (lazy-slot-form
                                            (list (list 'f 'i) 'x)))
                                     (list 'range
                                           start
                                           (list '+ start
                                                 (list 'count 'xs)))
                                     'xs))))
              coll-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :genList (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [count-result (lower-ast (:arg ast))]
            (if (= :ok (:status count-result))
              (ok-form (list 'mapv
                             (list 'fn ['i]
                                   (lazy-slot-form
                                    (list (:form fn-result) 'i)))
                             (list 'range (:form count-result))))
              count-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :zipAttrsWith (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [rows-result (lower-ast (:arg ast))]
            (if (= :ok (:status rows-result))
              (ok-form (list 'let ['f (list `assert-function (:form fn-result))
                                    'rows (list 'mapv
                                               `force-slot
                                               (force-slot-form (:form rows-result)))
                                    'attr-names (list 'sort
                                                      (list 'distinct
                                                            (list 'mapcat
                                                                  'keys
                                                                  'rows)))]
                             (list 'into
                                   {}
                                   (list 'map
                                         (list 'fn ['attr-name]
                                               ['attr-name
                                                (lazy-slot-form
                                                 (list (list 'f 'attr-name)
                                                       (list 'vec
                                                             (list 'keep
                                                                   (list 'fn ['row]
                                                                         (list 'when
                                                                               (list 'contains? 'row 'attr-name)
                                                                               (list 'get 'row 'attr-name)))
                                                                   'rows))))])
                                         'attr-names))))
              rows-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :mapAttrsToList (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [attrs-result (lower-ast (:arg ast))]
            (if (= :ok (:status attrs-result))
              (ok-form (list 'let ['f (list `assert-function (:form fn-result))
                                    'attrs (force-slot-form (:form attrs-result))]
                             (list 'mapv
                                   (list 'fn ['k]
                                         (lazy-slot-form
                                          (list (list 'f 'k)
                                                (list 'get 'attrs 'k))))
                                   (list 'sort
                                         (list 'keys 'attrs)))))
              attrs-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :mapAttrs (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [attrs-result (lower-ast (:arg ast))]
            (if (= :ok (:status attrs-result))
              ;; D9 (D2 parity in this lane): an EMPTY attrset must not
              ;; evaluate the function argument (Nix early-returns).
              (ok-form (list 'let ['attrs (force-slot-form (:form attrs-result))]
                             (list 'if (list 'empty? 'attrs)
                                   {}
                                   (list 'let ['f (list `assert-function (:form fn-result))]
                                         (list 'into
                                               {}
                                               (list 'map
                                                     (list 'fn ['k]
                                                           ['k (lazy-slot-form
                                                                (list (list 'f 'k)
                                                                      (list 'get 'attrs 'k)))])
                                                     (list 'sort
                                                           (list 'keys 'attrs))))))))
              attrs-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :filterAttrs (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [pred-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status pred-result))
          pred-result
          (let [attrs-result (lower-ast (:arg ast))]
            (if (= :ok (:status attrs-result))
              (ok-form (list 'let ['attrs (force-slot-form (:form attrs-result))]
                             (list 'into
                                   {}
                                   (list 'keep
                                         (list 'fn ['k]
                                               (list 'when
                                                     (list (list (:form pred-result) 'k)
                                                           (list 'get 'attrs 'k))
                                                     ['k (list 'get 'attrs 'k)]))
                                         (list 'sort
                                               (list 'keys 'attrs))))))
              attrs-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :groupBy (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [fn-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status fn-result))
          fn-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              (ok-form (list 'reduce
                             (list 'fn ['acc 'item]
                                   (list 'let ['k (list (:form fn-result) 'item)]
                                         (list 'update 'acc 'k (list 'fnil 'conj []) 'item)))
                             {}
                             (force-slot-form (:form coll-result))))
              coll-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :partition (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [pred-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status pred-result))
          pred-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              (ok-form (list 'reduce
                             (list 'fn ['acc 'item]
                                   (list 'update
                                         'acc
                                         (list 'if
                                               (list (:form pred-result) 'item)
                                               "right"
                                               "wrong")
                                         'conj
                                         'item))
                             {"right" []
                              "wrong" []}
                             (force-normal-form (:form coll-result))))
              coll-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :filter (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [pred-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status pred-result))
          pred-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              ;; D9 (D2 parity in this lane): an EMPTY list must not evaluate
              ;; the predicate form (Nix early-returns before forceFunction).
              (ok-form (list 'let ['xs (force-slot-form (:form coll-result))]
                             (list 'if (list 'empty? 'xs)
                                   []
                                   (list 'filterv
                                         (:form pred-result)
                                         'xs))))
              coll-result))))

      (and (= :call (get-in ast [:fn :fn :op]))
           (= :call (get-in ast [:fn :op]))
           (= :call (:op ast))
           (contains? #{:foldl' :foldl}
                      (builtin-select-name (get-in ast [:fn :fn :fn])))
           (:arg (get-in ast [:fn :fn]))
           (:arg (:fn ast))
           (:arg ast))
      (let [step-result (lower-ast (get-in ast [:fn :fn :arg]))]
        (if (not= :ok (:status step-result))
          step-result
          (let [init-result (lower-ast (get-in ast [:fn :arg]))]
            (if (not= :ok (:status init-result))
              init-result
              (let [coll-result (lower-ast (:arg ast))]
                (if (= :ok (:status coll-result))
                  ;; D9 (D2 parity in this lane): the initial accumulator is
                  ;; LAZY in Nix foldl' (an operator that ignores it never
                  ;; evaluates it) — init enters as a lazy slot exactly like
                  ;; list-element slots (the operator body forces on USE), and
                  ;; the final result is forced (the strict fold, covering the
                  ;; empty-list case where the slot itself is returned).
                  (ok-form (force-slot-form
                            (list 'reduce
                                  (list 'fn ['acc 'item]
                                        (list (list (:form step-result) 'acc)
                                              'item))
                                  (lazy-slot-form (:form init-result))
                                  (force-slot-form (:form coll-result)))))
                  coll-result))))))

      (and (= :call (get-in ast [:fn :op]))
           (= :sort (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [cmp-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status cmp-result))
          cmp-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              (ok-form (list 'vec
                             (list 'sort
                                   (list 'fn ['a 'b]
                                         (list (list (:form cmp-result) 'a)
                                               'b))
                                   (force-slot-form (:form coll-result)))))
              coll-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :elemAt (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [coll-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status coll-result))
          coll-result
          (let [idx-result (lower-ast (:arg ast))]
            (if (= :ok (:status idx-result))
              (ok-form (force-slot-form
                        (list 'nth
                              (:form coll-result)
                              (:form idx-result))))
              idx-result))))

      (and (= :call (get-in ast [:fn :op]))
           (= :elem (builtin-select-name (get-in ast [:fn :fn])))
           (:arg (:fn ast))
           (:arg ast))
      (let [needle-result (lower-ast (get-in ast [:fn :arg]))]
        (if (not= :ok (:status needle-result))
          needle-result
          (let [coll-result (lower-ast (:arg ast))]
            (if (= :ok (:status coll-result))
              (ok-form (list 'boolean
                             (list 'some
                                   (list 'fn ['x]
                                         (list `nix-equal 'x (:form needle-result) true))
                                   (force-slot-form (:form coll-result)))))
              coll-result))))

      :else
      nil)))

(defn- lambda-binding?
  [{:keys [value]}]
  (= :lambda (:op value)))

(defn- lower-letfn-binding
  [{:keys [name value]}]
  (if (:param-pattern value)
    {:status :failed
     :reason :pattern-lambda-lowering-not-wired
     :name name}
    (let [param-sym (symbol (:param value))
          body-result (binding [*force-on-read-vars*
                                (conj *force-on-read-vars* param-sym)
                                *lexical-vars*
                                (conj *lexical-vars* param-sym)]
                        (lower-ast (:body value)))]
      (if (= :ok (:status body-result))
        {:status :ok
         :binding (list (symbol name)
                        [param-sym]
                        (:form body-result))}
        body-result))))

(defn- lower-letfn
  [bindings body]
  (let [binding-syms (mapv (comp symbol :name) bindings)]
    (binding [*lexical-vars* (into *lexical-vars* binding-syms)]
      (loop [remaining bindings
             lowered []]
        (if-let [binding (first remaining)]
          (let [result (lower-letfn-binding binding)]
            (if (= :ok (:status result))
              (recur (rest remaining) (conj lowered (:binding result)))
              result))
          (let [body-result (lower-ast body)]
            (if (= :ok (:status body-result))
              (ok-form (list 'letfn (vec lowered) (:form body-result)))
              body-result)))))))

(defn- lower-sequential-let
  [bindings body]
  (loop [remaining bindings
         pairs []]
    (if-let [{:keys [name value]} (first remaining)]
      (let [result (lower-ast value)]
        (if (= :ok (:status result))
          (recur (rest remaining)
                 (conj pairs (symbol name) (:form result)))
          result))
      (let [body-result (lower-ast body)]
        (if (= :ok (:status body-result))
          (ok-form (list 'let (vec pairs) (:form body-result)))
          body-result)))))

(defn- lower-recursive-let
  [bindings body]
  (let [binding-syms (mapv (comp symbol :name) bindings)]
    (binding [*lexical-vars* (into *lexical-vars* binding-syms)]
      (loop [remaining bindings
             pairs []]
        (if-let [{:keys [name value]} (first remaining)]
          (let [result (lower-ast value)]
            (if (= :ok (:status result))
              (recur (rest remaining)
                     (conj pairs (symbol name) (:form result)))
              result))
          (let [body-result (lower-ast body)]
            (if (= :ok (:status body-result))
              (ok-form (list 'pnix.clj-meta.compiler/lazy-letrec
                             (vec pairs)
                             (:form body-result)))
              body-result)))))))

(defn- split-leading-lambdas
  [bindings]
  (split-with lambda-binding? bindings))

(defn- lower-let-with-lambda-prefix
  [bindings body]
  (let [[lambda-bindings later-bindings] (split-leading-lambdas bindings)]
    (if (and (seq lambda-bindings) (seq later-bindings))
      (loop [remaining lambda-bindings
             lowered []]
        (if-let [binding (first remaining)]
          (let [result (lower-letfn-binding binding)]
            (if (= :ok (:status result))
              (recur (rest remaining) (conj lowered (:binding result)))
              result))
          (let [body-result (lower-sequential-let later-bindings body)]
            (if (= :ok (:status body-result))
              (ok-form (list 'letfn (vec lowered) (:form body-result)))
              body-result))))
      (lower-sequential-let bindings body))))

(defn- lower-ast*
  "Lower the current small pnix expression core into a Clojure form for clj-meta."
  [ast]
  (case (:op ast)
    :int
    (ok-form (:value ast))

    :float
    (ok-form (:value ast))

    :bool
    (ok-form (:value ast))

    :null
    (ok-form nil)

    :string
    (ok-form (:value ast))

    :path
    (ok-form {"__pnix_value_kind" "path"
              "path" (:value ast)})

    :string-template
    (lower-string-template (:parts ast))

    :list
    (lower-items (:items ast))

    :attrset
    (if (:recursive ast)
      (lower-recursive-attrs (:attrs ast))
      (lower-attrs (:attrs ast)))

    :var
    (let [sym (symbol (:name ast))
          emit-sym (get *lexical-renames* sym sym)]
      (ok-form (cond
                 (contains? *force-on-read-vars* sym)
                 (force-slot-form emit-sym)

                 (contains? *lexical-vars* sym)
                 emit-sym

                 ;; free `builtins` = the global builtin set as a VALUE.
                 ;; Checked BEFORE with-scopes: statically-known `builtins`
                 ;; wins over `with` (Nix + direct-evaluator semantics; the
                 ;; with-scope lookup answering first was a live lane
                 ;; divergence against the direct evaluator).
                 (= sym 'builtins)
                 (list `builtins-attrset)

                 ;; bare `throw` / `abort` (default-env globals, like the
                 ;; direct evaluator binds them).
                 (= sym 'throw)
                 `throw-value

                 (= sym 'abort)
                 `abort-value

                 ;; D13: a bare default-scope builtin as a VALUE — the same
                 ;; entry builtins.X selects. Checked BEFORE with-scopes:
                 ;; the static base scope wins over `with` (oracle-confirmed)
                 ;; and *lexical-vars* shadowing was already answered above.
                 (contains? bare-default-scope-builtins (:name ast))
                 (list 'get (list `builtins-attrset) (:name ast))

                 (seq *with-scope-syms*)
                 (list `lookup-with-scopes (vec *with-scope-syms*) (:name ast))

                 :else
                 sym)))

    :let
    (lower-recursive-let (:bindings ast) (:body ast))

    :if
    (let [condition-result (lower-ast (:condition ast))]
      (if (not= :ok (:status condition-result))
        condition-result
        (let [then-result (lower-ast (:then ast))]
          (if (not= :ok (:status then-result))
            then-result
            (let [else-result (lower-ast (:else ast))]
              (if (= :ok (:status else-result))
                (ok-form (list 'if
                               (list `require-bool (:form condition-result) "if")
                               (:form then-result)
                               (:form else-result)))
                else-result))))))

    :assert
    (let [condition-result (lower-ast (:condition ast))]
      (if (not= :ok (:status condition-result))
        condition-result
        (let [body-result (lower-ast (:body ast))]
          (if (= :ok (:status body-result))
            (ok-form (list 'if
                           (list `require-bool (:form condition-result) "assert")
                           (:form body-result)
                           (list 'throw
                                 (list 'ex-info
                                       "assertion failed"
                                       {:pnix/catchable true
                                        :reason :assertion-failed}))))
            body-result))))

    :with
    (let [env-result (lower-ast (:env-expr ast))]
      (if (not= :ok (:status env-result))
        env-result
        (let [scope-sym (symbol (str "__pnix_with_scope_" *with-depth*))
              body-result (binding [*with-scope-syms*
                                    (cons scope-sym *with-scope-syms*)
                                    *with-depth*
                                    (inc *with-depth*)]
                            (lower-ast (:body ast)))]
          (if (= :ok (:status body-result))
            (ok-form (list 'let [scope-sym (force-slot-form (:form env-result))]
                           (list 'if
                                 (list 'map? scope-sym)
                                 (:form body-result)
                                 (list 'throw
                                       (list 'ex-info
                                             "with expression is not an attrset"
                                             {:reason :with-not-attrset
                                              :value scope-sym})))))
            body-result))))

    :lambda
    (if-let [pattern (:param-pattern ast)]
      ;; Pattern lambda, D19 semantics (oracle-gated, mirrors the host):
      ;; pattern-guard runs the application-time checks (attrset / required
      ;; formals in pattern order / extra keys unless `...`); each MISSING
      ;; formal binds a LAZY default slot in a KNOT-TIED recursive scope —
      ;; the delay re-reads every formal slot from a delivered promise, so a
      ;; default can reference ANY formal ('({ a ? b, b ? 2 }: a) { }' is 2)
      ;; and an unused default is never evaluated; @as binds the ACTUAL
      ;; argument only. The fn carries :pnix/function-args metadata so
      ;; functionArgs works on VALUES.
      (let [param-names (mapv :name (:params pattern))
            param-syms (mapv symbol param-names)
            required (into [] (comp (remove :default) (map :name))
                           (:params pattern))
            as-name (:as pattern)
            arg-sym 'pattern-arg*
            map-sym 'pattern-map*
            knot-sym 'pattern-knot*
            all-syms (into param-syms (when as-name [(symbol as-name)]))
            fa-meta (into {}
                          (map (fn [{:keys [name default]}]
                                 [name (boolean default)]))
                          (:params pattern))]
        (binding [*force-on-read-vars* (into *force-on-read-vars* param-syms)
                  *lexical-vars* (into *lexical-vars* all-syms)]
          (let [bind-results
                (reduce
                 (fn [acc {:keys [name default]}]
                   (if (not= :ok (:status acc))
                     acc
                     (let [d (when default (lower-ast default))]
                       (if (and d (not= :ok (:status d)))
                         d
                         (update acc :forms conj
                                 (symbol name)
                                 (if d
                                   ;; missing → lazy default in the knot:
                                   ;; re-bind every formal from the promise
                                   ;; before the default form runs.
                                   (list 'if (list 'contains? map-sym name)
                                         (list 'get map-sym name)
                                         (list 'delay
                                               (list 'let
                                                     [(vec param-syms)
                                                      (list 'clojure.core/deref
                                                            knot-sym)]
                                                     (:form d))))
                                   (list 'get map-sym name)))))))
                 {:status :ok :forms []}
                 (:params pattern))]
            (if (not= :ok (:status bind-results))
              bind-results
              (let [body-result (lower-ast (:body ast))]
                (if (not= :ok (:status body-result))
                  body-result
                  (ok-form
                   (list 'with-meta
                         (list 'fn [arg-sym]
                               (list 'let
                                     (-> [map-sym
                                          (list `pattern-guard
                                                (list `pattern-actual arg-sym)
                                                (vec param-names)
                                                required
                                                (boolean (:ellipsis? pattern)))
                                          knot-sym (list 'promise)]
                                         (into (if as-name
                                                 [(symbol as-name) map-sym]
                                                 []))
                                         (into (:forms bind-results))
                                         (into ['_ (list 'deliver knot-sym
                                                         (vec param-syms))]))
                                     (:form body-result)))
                         {:pnix/function-args fa-meta}))))))))
      (let [param-sym (symbol (:param ast))
            body-result (binding [*force-on-read-vars*
                                  (conj *force-on-read-vars* param-sym)
                                  *lexical-vars*
                                  (conj *lexical-vars* param-sym)]
                          (lower-ast (:body ast)))]
        (if (= :ok (:status body-result))
          (ok-form (list 'fn [param-sym] (:form body-result)))
          body-result)))

    :binary
    (let [left-result (lower-ast (:left ast))]
      (if (not= :ok (:status left-result))
        left-result
        (let [right-result (lower-ast (:right ast))]
          (if (not= :ok (:status right-result))
            right-result
            (ok-form (case (:operator ast)
                       "//" (list 'merge
                                  (:form left-result)
                                  (:form right-result))
                       "++" (list 'vec
                                  (list 'concat
                                        (:form left-result)
                                        (:form right-result)))
                       "==" (list `nix-equal
                                  (:form left-result)
                                  (:form right-result))
                       "!=" (list 'not
                                  (list `nix-equal
                                        (:form left-result)
                                        (:form right-result)))
                       "<" (list `nix-order
                                  "<"
                                  (:form left-result)
                                  (:form right-result))
                       ">" (list `nix-order
                                  ">"
                                  (:form left-result)
                                  (:form right-result))
                       "<=" (list `nix-order
                                   "<="
                                   (:form left-result)
                                   (:form right-result))
                       ">=" (list `nix-order
                                   ">="
                                   (:form left-result)
                                   (:form right-result))
                       "+" (list `nix-binary
                                  "+"
                                  (:form left-result)
                                  (:form right-result))
                       "-" (list `nix-binary
                                  "-"
                                  (:form left-result)
                                  (:form right-result))
                       "*" (list `nix-binary
                                  "*"
                                  (:form left-result)
                                  (:form right-result))
                       "/" (list `nix-binary
                                  "/"
                                  (:form left-result)
                                  (:form right-result))
                       "%" (list `nix-binary
                                  "%"
                                  (:form left-result)
                                  (:form right-result))
                       ;; D18: && || -> require boolean operands (real Nix
                       ;; type-errors; bare Clojure and/or was the same
                       ;; truthiness host leak require-bool closed for if).
                       ;; require-bool returns the checked boolean, so and/or
                       ;; still short-circuit exactly like the host lane.
                       "&&" (list 'and
                                  (list `require-bool (:form left-result) "&&")
                                  (list `require-bool (:form right-result) "&&"))
                       "||" (list 'or
                                  (list `require-bool (:form left-result) "||")
                                  (list `require-bool (:form right-result) "||"))
                       "->" (list 'if
                                  (list `require-bool (:form left-result) "->")
                                  (list `require-bool (:form right-result) "->")
                                  true)
                       (list (symbol (:operator ast))
                             (:form left-result)
                             (:form right-result))))))))

    :select
    (if-let [constant-form (builtin-constant-form ast)]
      (ok-form constant-form)
      (let [target-result (lower-ast (:target ast))]
        (if (not= :ok (:status target-result))
          target-result
          (let [attr-result (lower-attr-key (:attr ast))]
            (if (not= :ok (:status attr-result))
              attr-result
              (if-let [default (:default ast)]
                (let [default-result (lower-ast default)]
                  (if (= :ok (:status default-result))
                    (ok-form (list 'let ['target (force-slot-form (:form target-result))
                                         'attr (:form attr-result)]
                                    (list 'if
                                          (list 'and
                                                (list 'map? 'target)
                                                (list 'contains? 'target 'attr))
                                          (force-slot-form
                                           (list 'get 'target 'attr))
                                          (:form default-result))))
                    default-result))
                (ok-form (force-slot-form
                          (list 'get (:form target-result) (:form attr-result))))))))))

    :has-attr
    ;; Nix `?` on a non-attrset is FALSE, not an error (D6) — mirror the
    ;; evaluator lane's guard so `1 ? a` lowers to false too.
    (let [target-result (lower-ast (:target ast))]
      (if (not= :ok (:status target-result))
        target-result
        (let [attr-result (lower-attr-key (:attr ast))]
          (if (= :ok (:status attr-result))
            (ok-form (list 'let ['target (:form target-result)]
                           (list 'and
                                 (list 'map? 'target)
                                 (list 'contains? 'target
                                       (:form attr-result)))))
            attr-result))))

    :not
    (let [result (lower-ast (:expr ast))]
      (if (= :ok (:status result))
        (ok-form (list 'not (list `require-bool (:form result) "!")))
        result))

    :neg
    (let [result (lower-ast (:expr ast))]
      (if (= :ok (:status result))
        (ok-form (list `nix-neg (:form result)))
        result))

    :import
    (lower-import-target (:target ast))

    :call
    (or (lower-builtin-call ast)
        (let [fn-result (lower-ast (:fn ast))]
          (if (not= :ok (:status fn-result))
            fn-result
            (let [arg-result (lower-ast (:arg ast))]
              (if (= :ok (:status arg-result))
                ;; call-by-need application: the argument crosses as a lazy
                ;; slot (params force-slot on read), so `(x: 1) (1 / 0)` never
                ;; forces the argument — Nix semantics, not host strictness.
                ;; Scalars can't throw; bare symbols are already slots or
                ;; realized values — both pass through unwrapped.
                (let [arg-form (:form arg-result)
                      wrapped (if (or (number? arg-form)
                                      (string? arg-form)
                                      (boolean? arg-form)
                                      (nil? arg-form)
                                      (symbol? arg-form))
                                arg-form
                                (lazy-slot-form arg-form))]
                  (ok-form (list (:form fn-result) wrapped)))
                arg-result)))))

    {:status :failed
     :reason :unsupported-lowering-op
     :op (:op ast)}))

(defn lower-ast
  "Lower the current small pnix expression core into a Clojure data form.

  The cache key is the AST hash plus the lowering policy. The result remains
  independent of cache hit/miss so receipts stay deterministic across runs."
  [ast]
  (let [cache-key (lower-cache-key ast)]
    (if-let [cached (get @lower-cache cache-key)]
      (do
        (swap! lower-cache-stats* update :hits inc)
        cached)
      (let [result (assoc (lower-ast* ast)
                          :ast-hash (:ast-hash cache-key)
                          :cache-key cache-key)]
        (swap! lower-cache-stats* update :misses inc)
        (swap! lower-cache assoc cache-key result)
        result))))
