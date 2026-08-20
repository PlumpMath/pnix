(ns pnix-clj.error
  (:require [clojure.string :as str]))

(def lane-classification
  {:lane :core
   :scope :structured-error-envelope
   :role :structured-failure-and-policy-hold-schema
   :product-runtime :allowed
   :semantic-authority :error-shape-only
   :mutation :forbidden
   :admission :forbidden
   :determinism :required
   :allowed-output :pnix-failure-or-policy-hold-envelope})

(def schema
  :pnix-clj.error.v0)

(def machine-error-schema
  :pnix.machine.eval-error-model.v1)

(def ^:private machine-error-classes
  #{:syntax-error :invalid-machine-state :unsupported-expression
    :assertion-failed :string-interpolation-coercion-failed
    :throw-builtin-called :abort-builtin-called
    :unknown-variable :not-callable :non-boolean-condition
    :missing-heap-cell :invalid-heap-cell :thunk-blackhole
    :invalid-resume :invalid-guest-value :invalid-primitive-ref
    :unknown-primitive :primitive-arity :primitive-manifest-violation
    :primitive-unimplemented :primitive-nonstrict-unimplemented
    :type-error :integer-overflow :division-by-zero :invalid-byte :invalid-regex
    :hash-string-raw-bytes-unsupported
    :hash-string-algorithm-not-string
    :hash-string-algorithm-has-context
    :hash-string-unsupported-algorithm
    :hash-string-data-not-string
    :cycle-detected :import-cycle :import-module-not-found
    :primitive-contract-violation :attribute-missing
    :invalid-attrset-binding :duplicate-attrset-binding
    :throw-argument-not-string :abort-argument-not-string
    :unexpected-lambda-pattern-arg :function-args-target-not-callable
    :head-of-empty-list :tail-of-empty-list :last-of-empty-list :init-of-empty-list
    :invalid-effect-request :effect-request-schema-mismatch
    :effect-request-version-mismatch :effect-operation-registry-mismatch
    :unknown-effect-operation :invalid-effect-args
    :invalid-effect-capability :effect-denied :effect-adapter-error})

(def ^:private class-aliases
  {:unbound-var :unknown-variable
   :missing-attr :attribute-missing
   :select-target-not-attrset :type-error
   :call-target-not-callable :not-callable
   :infinite-recursion :cycle-detected
   :duplicate-attr :duplicate-attrset-binding
   :non-bool-if-condition :non-boolean-condition
   :non-bool-assert-condition :non-boolean-condition
   :non-bool-and-operand :non-boolean-condition
   :non-bool-or-operand :non-boolean-condition
   :non-bool-implies-operand :non-boolean-condition
   :non-bool-not-operand :non-boolean-condition})

(defn- machine-phase
  [phase reason]
  (cond
    (and (keyword? reason) (str/ends-with? (name reason) "purity-gated"))
    :effect

    (= phase :builtin)
    :eval

    (contains? #{:parse :resolution :eval :effect :effect-contract
                 :observation :primitive-contract} phase)
    phase

    :else
    :eval))

(defn- machine-class
  [phase reason details]
  (let [detail-class (:class details)]
    (cond
      (= "java.util.regex.PatternSyntaxException" detail-class) :invalid-regex
      (contains? machine-error-classes detail-class) detail-class
      (contains? machine-error-classes reason) reason
      (contains? class-aliases reason) (get class-aliases reason)
      (and (keyword? reason) (str/ends-with? (name reason) "purity-gated"))
      :effect-denied
      (= phase :parse) :syntax-error
      (= phase :builtin) :type-error
      :else :unsupported-expression)))

(def ^:private guest-message-classes
  "Classes whose `:message` is GUEST data — the string the evaluated program
  itself passed to `builtins.throw` / `builtins.abort`. Unlike host throwable
  text, it is produced by the source and is therefore deterministic, so it
  stays in the evidence (Nix prints it: `builtins.throw \"x\"` => `error: x`)."
  #{:throw-builtin-called :abort-builtin-called})

(defn- stable-evidence
  [details class]
  (apply dissoc (or details {})
         (cond-> [:class :data :throwable :stack :stack-trace]
           (not (contains? guest-message-classes class))
           (conj :message))))

(defn failed
  "Build a deterministic Failed outcome. Host throwable identity and text are
  never semantic evidence; callers retain a legacy reason while the nested
  error uses the closed common-machine phase/class vocabulary."
  ([phase reason]
   (failed phase reason nil))
  ([phase reason details]
   (let [class (machine-class phase reason details)
         evidence (stable-evidence details class)
         error {:schema machine-error-schema
                :phase (machine-phase phase reason)
                :class class
                :evidence evidence}]
     (merge {:status :failed
             :reason reason
             :error error}
            evidence))))

(defn failed-throwable
  ([phase reason throwable]
   (failed-throwable phase reason throwable nil))
  ([phase reason throwable details]
   (failed phase reason
           (cond-> (or details {})
             (instance? java.util.regex.PatternSyntaxException throwable)
             (assoc :class :invalid-regex)))))

(defn value
  "Build a deterministic pnix-clj error envelope. Callers keep legacy top-level
  result keys for existing reports; this envelope is the machine-readable shape."
  ([phase reason]
   (value phase reason nil))
  ([phase reason details]
   (let [details (or details {})
         rest-details (apply dissoc details [:message :class :data])]
     (cond-> {:schema schema
              :kind :pnix-error
              :phase phase
              :reason reason}
       (:message details)
       (assoc :message (:message details))

       (:class details)
       (assoc :class (:class details))

       (contains? details :data)
       (assoc :data (:data details))

       (seq rest-details)
       (assoc :details rest-details)))))

(defn throwable
  ([phase reason ^Throwable t]
   (throwable phase reason t nil))
  ([phase reason _ details]
   (value phase reason details)))

(defn policy-held
  "Build an explicit owner-policy hold. This constructor is not for language,
  runtime, type, parser, ABI, or host exceptions."
  ([phase reason]
   (policy-held phase reason nil))
  ([phase reason details]
   (merge {:status :held
           :reason reason
           :error (value phase reason details)}
          details)))

(defn held
  "Legacy compatibility name. Deterministic callers become Failed; new owner
  policy code must use policy-held explicitly."
  ([phase reason]
   (failed phase reason nil))
  ([phase reason details]
   (failed phase reason details)))

(defn held-throwable
  "Compatibility entry for old callers. A thrown host exception is decidable,
  so it is a Failed outcome rather than an owner-policy Held value."
  ([phase reason t]
   (failed-throwable phase reason t nil))
  ([phase reason t details]
   (failed-throwable phase reason t details)))
