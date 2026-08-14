(ns pnix-clj.evaluator
  (:require [clojure.string :as str]
            [pnix-clj.error :as err]
            [pnix-clj.hash :as hash]
            [pnix-clj.json :as json]
            [pnix-clj.math :as math]
            [pnix-clj.primitive-kernel :as primitive-kernel]
            [pnix-clj.version :as version]))

(deftype PxBytes [bs]
  ;; RAW-BYTE strings (owner-selected track, 2026-07-11): a byte-level op
  ;; produced bytes that are NOT valid UTF-8 — Nix permits such
  ;; intermediates; JVM Strings cannot hold them. `bs` is a vector of longs
  ;; 0..255. Only the oracle-pinned surface operates on these
  ;; (len/slice/concat/eq/lt); everything else is HELD fail-closed. deftype
  ;; (not a map) so attrset detection (map?) never confuses the two.
  Object
  (equals [_ other]
    (and (instance? PxBytes other) (= bs (.bs ^PxBytes other))))
  (hashCode [_] (hash bs)))

(defn- raw-bytes? [v] (instance? PxBytes v))

(defn- str->byte-vec [^String s]
  (mapv #(bit-and (long %) 0xFF) (.getBytes s "UTF-8")))

(defn- byte-vec-of
  "Byte view of a string-like or raw-bytes value; nil otherwise."
  [v]
  (cond
    (raw-bytes? v) (.bs ^PxBytes v)
    (string? v) (str->byte-vec v)
    :else nil))

(defn- bytes-val
  "The revalidating constructor: valid UTF-8 returns to String, else PxBytes."
  [bs]
  (let [arr (byte-array (map unchecked-byte bs))
        dec (.newDecoder java.nio.charset.StandardCharsets/UTF_8)]
    (.onMalformedInput dec java.nio.charset.CodingErrorAction/REPORT)
    (.onUnmappableCharacter dec java.nio.charset.CodingErrorAction/REPORT)
    (try
      (str (.decode dec (java.nio.ByteBuffer/wrap arr)))
      (catch java.nio.charset.CharacterCodingException _
        (PxBytes. (vec bs))))))

(def lane-classification
  {:lane :core
   :scope :semantic-evaluator
   :role :pnix-ast-to-value-evaluation-authority
   :product-runtime :allowed
   :semantic-authority :core-evaluator
   :mutation :forbidden
   :admission :receipt-gated-upstream
   :determinism :required
   :allowed-output :eval-result-or-held-frontier})

(def ^:private posix-regex-class->java
  ;; java.util.regex.Pattern's POSIX classes are US-ASCII unless the Unicode
  ;; character-class flag is enabled. We never enable that flag, matching the
  ;; Nix 2.34.7 oracle for these ERE bracket classes.
  {"alnum" "\\p{Alnum}"
   "alpha" "\\p{Alpha}"
   "blank" "\\p{Blank}"
   "cntrl" "\\p{Cntrl}"
   "digit" "\\p{Digit}"
   "graph" "\\p{Graph}"
   "lower" "\\p{Lower}"
   "print" "\\p{Print}"
   "punct" "\\p{Punct}"
   "space" "\\p{Space}"
   "upper" "\\p{Upper}"
   "xdigit" "\\p{XDigit}"})

(def ^:private posix-regex-class-token
  #"^\[:([a-z]+):\]")

(defn- translate-nix-regex-pattern
  "Translate known POSIX named classes only when they occur inside an ERE
  bracket expression. In particular, `[:space:]` by itself stays unchanged;
  only `[[:space:]]` contains the POSIX class token. Unknown named classes fail
  compilation like Nix; escaped bracket syntax passes through byte-for-byte."
  [pattern]
  (let [source (str pattern)
        n (count source)]
    (loop [i 0
           in-class? false
           class-first? false
           out (StringBuilder.)]
      (if (>= i n)
        (.toString out)
        (let [ch (.charAt source i)]
          (cond
            ;; Preserve escapes as pairs so escaped brackets cannot alter the
            ;; scanner state and escaped text cannot be mistaken for a class.
            (and (= ch \\) (< (inc i) n))
            (do (.append out (subs source i (+ i 2)))
                (recur (+ i 2) in-class? false out))

            (not in-class?)
            (do (.append out (str ch))
                (recur (inc i) (= ch \[) (= ch \[) out))

            ;; At the start of a bracket expression, `^` negates and `]` is a
            ;; literal. Neither closes the class.
            (and class-first? (= ch \^))
            (do (.append out "^")
                (recur (inc i) true true out))

            (and class-first? (= ch \]))
            (do (.append out "]")
                (recur (inc i) true false out))

            (= ch \[)
            (if-let [[token class-name]
                     (re-find posix-regex-class-token (subs source i))]
              (if-let [replacement (get posix-regex-class->java class-name)]
                (do (.append out replacement)
                    (recur (+ i (count token)) true false out))
                (throw (java.util.regex.PatternSyntaxException.
                        (str "Unknown POSIX character class " token)
                        source
                        i)))
              (do (.append out "[")
                  (recur (inc i) true false out)))

            (= ch \])
            (do (.append out "]")
                (recur (inc i) false false out))

            :else
            (do (.append out (str ch))
                (recur (inc i) true false out))))))))

(defn nix-regex-pattern
  "Compile a Nix ERE pattern on the JVM, including standard ASCII POSIX named
  bracket classes such as `[[:space:]]`. Shared by the evaluator and lowered
  Clojure lane so the two execution paths cannot drift."
  [pattern]
  (re-pattern (translate-nix-regex-pattern pattern)))

(declare eval-ast* apply-callable apply-host-fn force-value force-attr attrset-value?
         nix-equal-result finish-builtin nullary-builtin-result default-env strict-type)

(defn path-value
  [s]
  {"__pnix_value_kind" "path"
   "path" (str s)})

(defn path-value?
  [v]
  (and (map? v)
       (= "path" (get v "__pnix_value_kind"))
       (string? (get v "path"))))

(defn- path-or-string
  [v]
  (if (path-value? v)
    (get v "path")
    (str v)))

;; --- String context (pure simulation of Nix string context) ------------------
;;
;; A Nix string carries a context: the set of store-path dependencies that must
;; be realized before the string may be used. Context-free strings stay plain
;; JVM Strings (zero change for every existing program); a string only becomes
;; the tagged map below when its context is non-empty (created by
;; builtins.appendContext today, derivation outPath/drvPath in the derivation
;; slice). The tag uses the same string-keyed "__pnix_value_kind" idiom as
;; path-value so the value stays pnix-representable. Context elements are plain
;; strings; getContext reports them as an attrset of empty info attrsets
;; (pure-simulation scope: we track WHICH dependencies, not yet the
;; path/allOutputs/outputs kinds). Consumers that are not context-aware are
;; denied by default (held :string-context-frontier) instead of silently
;; mangling or dropping context.

(defn ctx-string
  "Build a pnix string with context. Returns a plain String when the context is
  empty, so context-free results stay indistinguishable from ordinary strings."
  [s context]
  (let [ctx (vec (sort (distinct (map str context))))]
    (if (empty? ctx)
      (str s)
      {"__pnix_value_kind" "string-context"
       "string" (str s)
       "context" ctx})))

(defn- forced-entry
  "Fetch a map entry through the thunk seam: lazy attrset slots hold thunks,
  and the ctx-string accessors must see the realized value (a tagged map built
  by a pnix attrset literal — e.g. inside the .px runtime — carries thunks in
  its slots)."
  [v k]
  (let [x (get v k)
        r (force-value x)]
    (when (= :ok (:status r)) (:value r))))

(defn ctx-string?
  [v]
  (and (map? v)
       (= "string-context" (forced-entry v "__pnix_value_kind"))
       (string? (forced-entry v "string"))
       (vector? (forced-entry v "context"))))

(defn- string-like?
  [v]
  (or (string? v) (ctx-string? v)))

(defn string-content
  "Character content of a string-like value; any other value passes through
  unchanged (so this can sit inside generic str-joining code)."
  [v]
  (if (ctx-string? v) (forced-entry v "string") v))

(defn string-ctx
  [v]
  (if (ctx-string? v)
    (mapv (fn [e] (let [r (force-value e)]
                    (when (= :ok (:status r)) (:value r))))
          (forced-entry v "context"))
    []))

(defn- message-error-result
  "Validate the string-only message contract for throw/abort outside the
  monolithic builtin dispatcher, which is close to the JVM method-size limit."
  [name message]
  (if (string-like? message)
    (err/failed :builtin
              (if (= name :throw)
                :throw-builtin-called
                :abort-builtin-called)
              {:builtin name
               :message (string-content message)})
    (err/failed :builtin
              (if (= name :throw)
                :throw-argument-not-string
                :abort-argument-not-string)
              {:builtin name
               :value-type (strict-type message)})))

(defn- nix-to-path
  "Lexically normalize an absolute path string, matching builtins.toPath.
  This does not touch the filesystem or turn the result into a path value."
  [value]
  (let [s (string-content value)]
    (when-not (string? s)
      (throw (ex-info "builtins.toPath: expected a string"
                      {:value-type (strict-type value)})))
    (when-not (str/starts-with? s "/")
      (throw (ex-info (str "string '" s "' doesn't represent an absolute path")
                      {:value s})))
    (let [parts (reduce (fn [out part]
                          (cond
                            (or (= part "") (= part ".")) out
                            (= part "..") (if (seq out) (pop out) out)
                            :else (conj out part)))
                        []
                        (str/split s #"/" -1))
          normalized (if (seq parts) (str "/" (str/join "/" parts)) "/")]
      (if (ctx-string? value)
        (ctx-string normalized (string-ctx value))
        normalized))))

(def ^:dynamic *import-resolver*
  nil)

(def ^:dynamic *import-context*
  nil)

(def ^:dynamic *import-origin*
  nil)

(def import-context-key
  "Reserved evaluator-env key carrying a module's normalized origin target.
  Closures and thunks capture their env, so this preserves relative-import
  provenance after the resolver's dynamic binding has returned."
  ::import-context)

(defn import-resolver-context
  "Build the resolver payload while keeping lexical module origin separate
  from the currently-active import cycle stack."
  [env]
  {:chain (vec (or *import-context* []))
   :origin (or (when env (get env import-context-key))
               *import-origin*)})

(defn contextual-import-target
  "Resolve a relative import spelling against the module that issued it.
  Import module maps are keyed from the outer source directory, while nested
  modules use paths relative to themselves. Keep the result in the same
  outer-relative spelling space so every evaluator/lowering lane selects the
  same module and cycle checks compare normalized targets."
  [context target]
  (let [target (str target)
        empty-parts (make-array String 0)
        target-path (java.nio.file.Paths/get target empty-parts)]
    (if (or (empty? context) (.isAbsolute target-path))
      target
      (let [parent-path (java.nio.file.Paths/get (str (last context)) empty-parts)
            parent-dir (or (.getParent parent-path)
                           (java.nio.file.Paths/get "." empty-parts))
            resolved (str (.normalize (.resolve parent-dir target-path)))]
        (if (or (.startsWith resolved "../")
                (.startsWith resolved "./")
                (.startsWith resolved "/"))
          resolved
          (str "./" resolved))))))

(def ^:dynamic *import-modules*
  "In-memory pnix module map (target-string -> pnix source) consulted by the
  host import resolver. Empty by default so `import <target>` stays held with
  :import-evaluation-not-wired unless a caller supplies modules. No filesystem
  access: this is the pnix module-loader seam, exercised purely in-process."
  {})

(def ^:dynamic *fuel*
  "When bound to a (volatile! n) step budget, every eval-ast* entry decrements
  it; hitting zero throws a tagged exception that eval-ast-with-fuel converts
  to a held :fuel-exhausted. Default nil = unlimited, zero overhead paths
  unchanged. Shared seam for specialize's bounded folding (M1) and the
  safe-eval tier (M5)."
  nil)

(def ^:dynamic *strict-audit*
  "When bound to an atom, the evaluator records (WITHOUT changing behavior) the
  operations that would fail under strict Nix typing: a non-boolean `if`/`assert`
  condition or `!` operand, and `+` coercing a string with a non-string. Default
  nil disables all auditing, so evaluation is byte-for-byte identical."
  nil)

(def ^:dynamic *strict*
  "Strict Nix typing IS pnix's semantics (R2 Phase D landed 2026-07-07, owner
  doctrine: truthy if/assert/! and string+non-string coercion were CLOJURE
  HOST LEAKS into the guest language, never a dialect — two languages, no
  blending). The default is true; the dynamic var remains only as the
  mechanism the strict-audit lane and tests hook into. Flip measurement
  before landing: 278 corpus+runtime sources, strict violations 0."
  true)

(def ^:dynamic *coverage*
  "When bound to an atom, evaluator execution records lightweight semantic
  coverage counters. Default nil keeps evaluation behavior unchanged."
  nil)

(defn- cover!
  [category id]
  (when *coverage*
    (swap! *coverage* update-in [category id] (fnil inc 0)))
  nil)

(defn strict-type
  "The strict-typing name of a pnix value. Public: shared with the derived
  abstract machine's guard data."
  [v]
  (cond
    (boolean? v) :bool
    (integer? v) :int
    (float? v) :float
    (path-value? v) :path
    (instance? PxBytes v) :string
    (string? v) :string
    (ctx-string? v) :string
    (vector? v) :list
    (nil? v) :null
    (and (map? v) (contains? #{:closure :builtin} (:kind v))) :function
    (map? v) :attrset
    :else :unknown))

(defn- audit-strict!
  [event]
  (when *strict-audit*
    (swap! *strict-audit* conj event))
  nil)

(defn- strict-violation
  [reason event]
  (audit-strict! event)
  (when *strict*
    (err/failed :eval reason event)))

(declare ctx-string?)

(defn- strict-string?
  "Strict-typing notion of a string: a plain String or a contextful string
  (contextful strings ARE strings in Nix) — or a raw-byte string (the
  Nix-permitted invalid-UTF-8 intermediate; raw-byte track 2026-07-11)."
  [v]
  (or (string? v) (ctx-string? v) (instance? PxBytes v)))

(defn- audit-string-arg
  [builtin arg-name value]
  (when-not (strict-string? value)
    (strict-violation
     :string-builtin-non-string
     {:construct :builtin
      :builtin builtin
      :issue :non-string-argument
      :arg arg-name
      :value-type (strict-type value)})))

(defn- audit-string-list
  [builtin arg-name values]
  (when (vector? values)
    (some (fn [[idx value]]
            (when-not (strict-string? value)
              (strict-violation
               :string-list-builtin-non-string-element
               {:construct :builtin
                :builtin builtin
                :issue :non-string-list-element
                :arg arg-name
                :index idx
                :value-type (strict-type value)})))
          (map-indexed vector values))))

(defn- audit-list-arg
  [builtin arg-name value]
  (when-not (vector? value)
    (strict-violation
     :builtin-arg-not-list
     {:construct :builtin
      :builtin builtin
      :issue :non-list-argument
      :arg arg-name
      :value-type (strict-type value)})))

(defn- audit-numeric-operands
  [operator left right]
  (when-not (and (number? left) (number? right))
    (strict-violation
     :arithmetic-non-number
     {:construct :binary
      :operator operator
      :issue :non-number-operand
      :left-type (strict-type left)
      :right-type (strict-type right)})))

(defn- checked-integer
  [phase reason-details f]
  (try
    {:status :ok :value (f)}
    (catch ArithmeticException _
      (err/failed phase :integer-overflow reason-details))))

(defn- production-primitive-result
  [operator left right]
  (let [outcome (primitive-kernel/invoke-shadow operator left right)]
    (if (= :ok (:kind outcome))
      {:status :ok :value (:value outcome)}
      (let [error-class (:class outcome)
            public-reason (if (= error-class :division-by-zero)
                            :eval-binary-failed
                            error-class)]
        (err/failed (:phase outcome)
                  public-reason
                  {:class error-class
                   :construct :binary
                   :operator operator
                   :primitive-id (get primitive-kernel/operator->primitive-id operator)})))))

(defn- arithmetic-result
  [phase reason-details operator left right int-float-fn]
  (or (audit-numeric-operands operator left right)
      (if (and (integer? left) (integer? right))
        (if (and (= phase :eval)
                 (= (:construct reason-details) :binary)
                 (contains? primitive-kernel/operator->primitive-id operator))
          (production-primitive-result operator left right)
          (checked-integer phase
                           (assoc reason-details
                                  :operator operator
                                  :left left
                                  :right right)
                           #(int-float-fn left right)))
        {:status :ok
         :value (int-float-fn left right)})))

(defn- unary-integer-result
  [phase reason-details operator value f]
  (or (when-not (number? value)
        (strict-violation
         :arithmetic-non-number
         {:construct :unary
          :operator operator
          :issue :non-number-operand
          :value-type (strict-type value)}))
      (if (integer? value)
        (checked-integer phase
                         (assoc reason-details
                                :operator operator
                                :value value)
                         #(f value))
        {:status :ok
         :value (f value)})))

(def ^:private nix-int-upper-exclusive-double
  ;; 2^63 is exactly representable as f64. It is the positive EXCLUSIVE bound
  ;; because NixInt is signed i64; the negative counterpart is inclusive.
  (double (bigint "9223372036854775808")))

(def ^:private nix-int-lower-inclusive-double
  (- nix-int-upper-exclusive-double))

(defn- integer-exactly-representable-as-double?
  [value]
  (let [as-double (double value)
        ^java.math.BigDecimal integer-decimal (bigdec value)
        double-decimal (java.math.BigDecimal. as-double)]
    (and (Double/isFinite as-double)
         (zero? (.compareTo integer-decimal double-decimal)))))

(defn- rounding-builtin-result
  "Nix 2.34.7 ceil/floor conversion discipline. Integer arguments first cross
  the NixInt -> NixFloat seam and must survive exactly; float results must fit
  the signed i64 interval before conversion."
  [name value]
  (cond
    (integer? value)
    (if (integer-exactly-representable-as-double? value)
      {:status :ok :value (long value)}
      (err/failed :builtin
                :nix-int-to-float-precision-loss
                {:builtin name :value value}))

    (float? value)
    (let [rounded (if (= name :floor)
                    (Math/floor (double value))
                    (Math/ceil (double value)))]
      (if (and (Double/isFinite rounded)
               (<= nix-int-lower-inclusive-double rounded)
               (< rounded nix-int-upper-exclusive-double))
        {:status :ok :value (long rounded)}
        (err/failed :builtin
                  :float-to-nix-int-out-of-range
                  {:builtin name :value value})))

    :else
    (err/failed :builtin
              :rounding-argument-not-number
              {:builtin name :value-type (strict-type value)})))

(defn- div-result
  [phase reason-details left right]
  (or (audit-numeric-operands "/" left right)
      (cond
        (and (integer? left) (integer? right)
             (= left Long/MIN_VALUE)
             (= right -1))
        (err/failed phase
                  :integer-overflow
                  (assoc reason-details
                         :operator "/"
                         :left left
                         :right right))

        :else
        (try
          {:status :ok :value (math/div left right)}
          (catch ArithmeticException t
            (if (= "long overflow" (.getMessage t))
              (err/failed phase
                        :integer-overflow
                        (assoc reason-details
                               :operator "/"
                               :left left
                               :right right))
              (throw t)))))))

(defn- mod-result
  [phase reason-details left right]
  (or (audit-numeric-operands "%" left right)
      (try
        {:status :ok
         :value (if (and (integer? left) (integer? right))
                  (rem left right)
                  (- left (* (math/div left right) right)))}
        (catch ArithmeticException t
          (if (= "long overflow" (.getMessage t))
            (err/failed phase
                      :integer-overflow
                      (assoc reason-details
                             :operator "%"
                             :left left
                             :right right))
            (throw t))))))

(defn- builtin
  [name arity]
  {:kind :builtin
   :name name
   :arity arity
   :args []})

(defn- lazy-builtin-arg?
  [f]
  (or (and (= :elem (:name f))
           (empty? (:args f)))
      ;; D2 (oracle-confirmed): Nix map/filter/mapAttrs/sort never force their
      ;; function argument when the collection is empty (literal or computed),
      ;; so it is collected lazily and forced by finish-builtin only on a
      ;; non-empty (or non-collection) second argument.
      (and (contains? #{:map :filter :mapAttrs :sort} (:name f))
           (empty? (:args f)))
      ;; D2 (oracle-confirmed): foldl' passes the initial accumulator lazily;
      ;; it is forced only per-iteration result / final result, so an operator
      ;; that ignores it never evaluates it.
      (and (= :foldl' (:name f))
           (= 1 (count (:args f))))
      (and (contains? #{:optional :optionalString} (:name f))
           (= 1 (count (:args f))))
      (and (= :replicate (:name f))
           (= 1 (count (:args f))))
      ;; Validate hashString's already-strict algorithm before forcing its
      ;; payload, so an algorithm error wins over a poisoned second argument.
      (and (= :hashString (:name f))
           (= 1 (count (:args f))))))

(def impure-builtins
  "Builtin names whose evaluation is purity-gated when *pure-eval* is true.
  Single source for the safe-eval tier's static purity check."
  #{"readFile" "readDir" "getEnv" "storePath" "pathExists"
    "toFile" "fetchurl" "fetchTarball" "fetchGit"})

(def ^:dynamic *pure-eval*
  "When true, host filesystem / network / env effects are purity-gated.
  Default false so interactive eval matches nix-instantiate. safe-eval binds true."
  false)

(def ^:private pnix-store-root
  "/tmp/pnix-nix-store")

(defn- ensure-pnix-store!
  []
  (let [d (java.io.File. pnix-store-root)]
    (.mkdirs d)
    d))

(defn- host-path-string
  "Coerce a path-or-string value to a host filesystem path string."
  [v]
  (path-or-string v))

(def default-env
  {"builtins" {"builtins" {:kind :thunk
                            :label :builtins-self
                            :state (atom {:phase :pending})
                            :compute (fn [] {:status :ok
                                             :value (get default-env "builtins")})}
               "true" true
               "false" false
               "null" nil
               "add" (builtin :add 2)
               "and" (builtin :and 2)
               "append" (builtin :append 2)
               "abs" (builtin :abs 1)
               "attrNames" (builtin :attrNames 1)
               "atan2" (builtin :atan2 2)
               "baseNameOf" (builtin :baseNameOf 1)
               "ceil" (builtin :ceil 1)
               "catAttrs" (builtin :catAttrs 2)
               "compareVersions" (builtin :compareVersions 2)
               "concatLists" (builtin :concatLists 1)
               "concatMap" (builtin :concatMap 2)
               "cons" (builtin :cons 2)
               "currentSystem" version/current-system
               "langVersion" version/lang-version
               "div" (builtin :div 2)
               "drop" (builtin :drop 2)
               "elem" (builtin :elem 2)
               "elemAt" (builtin :elemAt 2)
               "eq" (builtin :eq 2)
               "exp" (builtin :exp 1)
               "filter" (builtin :filter 2)
               "filterAttrs" (builtin :filterAttrs 2)
               "find" (builtin :find 2)
               "flatten" (builtin :flatten 1)
               "floor" (builtin :floor 1)
               "foldl'" (builtin :foldl' 3)
               "functionArgs" (builtin :functionArgs 1)
               "genList" (builtin :genList 2)
               "ge" (builtin :ge 2)
               "get" (builtin :get 2)
               "getAttr" (builtin :getAttr 2)
               "gt" (builtin :gt 2)
               "hasAttr" (builtin :hasAttr 2)
               "derivation" (builtin :derivation 1)
               "derivationStrict" (builtin :derivationStrict 1)
               "placeholder" (builtin :placeholder 1)
               "break" (builtin :break 1)
               "storePath" (builtin :storePath 1)
               "toPath" (builtin :toPath 1)
               "hasContext" (builtin :hasContext 1)
               "getContext" (builtin :getContext 1)
               "hashString" (builtin :hashString 2)
               "unsafeDiscardStringContext" (builtin :unsafeDiscardStringContext 1)
               "unsafeDiscardOutputDependency" (builtin :unsafeDiscardOutputDependency 1)
               "appendContext" (builtin :appendContext 2)
               "head" (builtin :head 1)
               "isAttrs" (builtin :isAttrs 1)
               "isBool" (builtin :isBool 1)
               "isFloat" (builtin :isFloat 1)
               "isFunction" (builtin :isFunction 1)
               "isInt" (builtin :isInt 1)
               "isList" (builtin :isList 1)
               "isNull" (builtin :isNull 1)
               "isPath" (builtin :isPath 1)
               "isString" (builtin :isString 1)
               "groupBy" (builtin :groupBy 2)
               "keys" (builtin :keys 1)
               "length" (builtin :length 1)
               "le" (builtin :le 2)
               "lessThan" (builtin :lessThan 2)
               "ln" (builtin :ln 1)
               "listToAttrs" (builtin :listToAttrs 1)
               "lt" (builtin :lt 2)
               "match" (builtin :match 2)
               "map" (builtin :map 2)
               "mapAttrs" (builtin :mapAttrs 2)
               "mapAttrs'" (builtin :mapAttrs' 2)
               "genAttrs" (builtin :genAttrs 2)
               "nameValuePair" (builtin :nameValuePair 2)
               "foldlAttrs" (builtin :foldlAttrs 3)
               "addErrorContext" (builtin :addErrorContext 2)
               "unsafeGetAttrPos" (builtin :unsafeGetAttrPos 2)
               "merge" (builtin :merge 2)
               "mod" (builtin :mod 2)
               "mul" (builtin :mul 2)
               "neg" (builtin :neg 1)
               "intersectAttrs" (builtin :intersectAttrs 2)
               "nixVersion" version/nix-version
               "not" (builtin :not 1)
               "or" (builtin :or 2)
               "parseDrvName" (builtin :parseDrvName 1)
               "partition" (builtin :partition 2)
               "pathExists" (builtin :pathExists 1)
               "pnixMounts" (builtin :pnixMounts 0)
               "pow" (builtin :pow 2)
               "replaceStrings" (builtin :replaceStrings 3)
               "removeAttrs" (builtin :removeAttrs 2)
               "reverseList" (builtin :reverseList 1)
               "cos" (builtin :cos 1)
               "sin" (builtin :sin 1)
               "sqrt" (builtin :sqrt 1)
               "sort" (builtin :sort 2)
               "sub" (builtin :sub 2)
               "all" (builtin :all 2)
               "any" (builtin :any 2)
               "abort" (builtin :abort 1)
               "split" (builtin :split 2)
               "splitVersion" (builtin :splitVersion 1)
               "storeDir" version/store-dir
               "concatStringsSep" (builtin :concatStringsSep 2)
               "concatStrings" (builtin :concatStrings 1)
               "hasPrefix" (builtin :hasPrefix 2)
               "hasSuffix" (builtin :hasSuffix 2)
               "attrValues" (builtin :values 1)
               "bitAnd" (builtin :bitAnd 2)
               "bitOr" (builtin :bitOr 2)
               "bitXor" (builtin :bitXor 2)
               "foldr" (builtin :foldr 3)
               "attrByPath" (builtin :attrByPath 3)
               "dirOf" (builtin :dirOf 1)
               "mapAttrsToList" (builtin :mapAttrsToList 2)
               "optional" (builtin :optional 2)
               "optionals" (builtin :optionals 2)
               "toLower" (builtin :toLower 1)
               "toUpper" (builtin :toUpper 1)
               "stringToCharacters" (builtin :stringToCharacters 1)
               "range" (builtin :range 2)
               "last" (builtin :last 1)
               "init" (builtin :init 1)
               "unique" (builtin :unique 1)
               "concatMapStrings" (builtin :concatMapStrings 2)
               "splitString" (builtin :splitString 2)
               "genericClosure" (builtin :genericClosure 1)
               "min" (builtin :min 2)
               "max" (builtin :max 2)
               "imap0" (builtin :imap0 2)
               "imap1" (builtin :imap1 2)
               "optionalString" (builtin :optionalString 2)
               "removePrefix" (builtin :removePrefix 2)
               "removeSuffix" (builtin :removeSuffix 2)
               "concatMapStringsSep" (builtin :concatMapStringsSep 3)
               "id" (builtin :id 1)
               "flip" (builtin :flip 3)
               "toInt" (builtin :toInt 1)
               "replicate" (builtin :replicate 2)
               "findFirst" (builtin :findFirst 3)
               "foldl" (builtin :foldl' 3)
               "getEnv" (builtin :getEnv 1)
               "recursiveUpdate" (builtin :recursiveUpdate 2)
               "hasInfix" (builtin :hasInfix 2)
               "pipe" (builtin :pipe 2)
               "count" (builtin :count 2)
               "zipListsWith" (builtin :zipListsWith 3)
               "boolToString" (builtin :boolToString 1)
               "readDir" (builtin :readDir 1)
               "readFile" (builtin :readFile 1)
               "stringLength" (builtin :stringLength 1)
               "substring" (builtin :substring 3)
               "set" (builtin :set 3)
               "tail" (builtin :tail 1)
               "take" (builtin :take 2)
               "throw" (builtin :throw 1)
               "trace" (builtin :trace 2)
               "tryEval" (builtin :tryEval 1)
               "typeOf" (builtin :typeOf 1)
               "deepSeq" (builtin :deepSeq 2)
               "seq" (builtin :seq 2)
               "fromJSON" (builtin :fromJSON 1)
               "toJSON" (builtin :toJSON 1)
               "toString" (builtin :toString 1)
               "values" (builtin :values 1)
               "zip" (builtin :zip 2)
               "zipAttrsWith" (builtin :zipAttrsWith 2)
               "zipAttrs" (builtin :zipAttrs 1)
               "toXML" (builtin :toXML 1)
               "toFile" (builtin :toFile 2)
               "getAttrFromPath" (builtin :getAttrFromPath 2)
               "hasAttrByPath" (builtin :hasAttrByPath 2)
               "warn" (builtin :warn 2)
               "fetchurl" (builtin :fetchurl 1)
               "fetchTarball" (builtin :fetchTarball 1)
               "fetchGit" (builtin :fetchGit 1)
               "implies" (builtin :implies 2)
               "optionalAttrs" (builtin :optionalAttrs 2)
               "when" (builtin :when 2)
               "const" (builtin :const 2)
               "fix" (builtin :fix 1)
               "sum" (builtin :sum 1)
               "product" (builtin :product 1)
               "updateManyAttrs" (builtin :updateManyAttrs 2)
               "zipLists" (builtin :zipLists 2)
               "getName" (builtin :getName 1)
               "getVersion" (builtin :getVersion 1)
               "getAttrFromPathOr" (builtin :getAttrFromPathOr 3)
               "filterAttrsRecursive" (builtin :filterAttrsRecursive 2)
               "mapAttrsRecursive" (builtin :mapAttrsRecursive 2)
               "intersectLists" (builtin :intersectLists 2)
               "subtractLists" (builtin :subtractLists 2)
               "assertMsg" (builtin :assertMsg 2)}
   ;; Nix default scope: a fixed subset of builtins is also bound unprefixed at
   ;; the top level (the rest require the `builtins.` prefix). `import` is both
   ;; syntax in this parser and a callable value for select-default precedence.
   ;; `true`, `false`, and `null` are handled as syntax.
   "import" (builtin :import 1)
   "scopedImport" (builtin :scopedImport 2)
   "abort" (builtin :abort 1)
   "baseNameOf" (builtin :baseNameOf 1)
   "dirOf" (builtin :dirOf 1)
   "isNull" (builtin :isNull 1)
   "map" (builtin :map 2)
   "removeAttrs" (builtin :removeAttrs 2)
   "throw" (builtin :throw 1)
   "toString" (builtin :toString 1)})

;; Inject `lib` after the circular builtins self-thunk is wired: lib is the
;; builtins attrset plus lib-only helpers, with a nested attrsets submodule.
(def default-env
  (let [builtins (get default-env "builtins")
        lib-only {"implies" (builtin :implies 2)
                  "optionalAttrs" (builtin :optionalAttrs 2)
                  "when" (builtin :when 2)
                  "const" (builtin :const 2)
                  "fix" (builtin :fix 1)
                  "sum" (builtin :sum 1)
                  "product" (builtin :product 1)
                  "updateManyAttrs" (builtin :updateManyAttrs 2)
                  "zipLists" (builtin :zipLists 2)
                  "getName" (builtin :getName 1)
                  "getVersion" (builtin :getVersion 1)
                  "getAttrFromPathOr" (builtin :getAttrFromPathOr 3)
                  "filterAttrsRecursive" (builtin :filterAttrsRecursive 2)
                  "mapAttrsRecursive" (builtin :mapAttrsRecursive 2)
                  "intersectLists" (builtin :intersectLists 2)
                  "subtractLists" (builtin :subtractLists 2)
                  "assertMsg" (builtin :assertMsg 2)
                  "getAttrFromPath" (builtin :getAttrFromPath 2)
                  "hasAttrByPath" (builtin :hasAttrByPath 2)
                  "zipAttrs" (builtin :zipAttrs 1)
                  "attrsets" {"isAttrs" (builtin :isAttrs 1)
                              "mapAttrsToList" (builtin :mapAttrsToList 2)
                              "zipAttrs" (builtin :zipAttrs 1)
                              "hasAttrByPath" (builtin :hasAttrByPath 2)
                              "getAttrFromPath" (builtin :getAttrFromPath 2)
                              "filterAttrsRecursive" (builtin :filterAttrsRecursive 2)
                              "mapAttrsRecursive" (builtin :mapAttrsRecursive 2)
                              "optionalAttrs" (builtin :optionalAttrs 2)
                              "attrByPath" (builtin :attrByPath 3)}}
        lib (merge builtins lib-only)]
    (assoc default-env "lib" lib)))

(defn- pnix-to-string
  [value]
  (when (ctx-string? value)
    ;; A raw (str ...) join would drop the context silently; refuse and let the
    ;; caller surface a held instead. Context-aware code paths must unwrap via
    ;; string-content/string-ctx explicitly.
    (throw (ex-info "string-context frontier: raw string join would drop context"
                    {:reason :string-context-frontier})))
  (str value))

(defn- interpolation-to-string
  [value]
  (let [r (force-value value)]
    (if (not= :ok (:status r))
      r
      (let [value (:value r)]
        (cond
          (string? value)
          {:status :ok :value value}

          ;; A contextful string interpolates as itself; the template joiner
          ;; unions the contexts of all chunks.
          (ctx-string? value)
          {:status :ok :value value}

          (path-value? value)
          (let [path (get value "path")]
            {:status :ok :value (ctx-string path [path])})

          (attrset-value? value)
          (cond
            (contains? value "__toString")
            (let [to-string-result (force-attr value "__toString")]
              (if (= :ok (:status to-string-result))
                (let [call-result (apply-callable (:value to-string-result) value)]
                  (if (= :ok (:status call-result))
                    (recur (:value call-result))
                    call-result))
                to-string-result))

            (contains? value "outPath")
            (let [out-path-result (force-attr value "outPath")]
              (if (= :ok (:status out-path-result))
                (recur (:value out-path-result))
                out-path-result))

            :else
            (err/failed :eval
                      :string-interpolation-coercion-failed
                      {:value value
                       :accepted [:string :path :attrset-__toString
                                  :attrset-outPath]}))

          :else
          (err/failed :eval
                    :string-interpolation-coercion-failed
                    {:value value
                     :accepted [:string :path :attrset-__toString
                                :attrset-outPath]}))))))

(declare nix-coerce-to-string)

(defn nix-float-str
  "Nix float-to-string coercion (oracle-confirmed, D4): C %.6f over the EXACT
  binary value — 1.5 -> \"1.500000\", 1.0e-7 -> \"0.000000\", 1.0e30 -> its
  full 31-digit expansion. Java's own %.6f pads shortest-repr digits instead,
  so this goes through BigDecimal. Nix canonicalizes negative zero to zero."
  [v]
  (let [d (double v)]
    (cond
      (Double/isNaN d) "nan"
      (= d Double/POSITIVE_INFINITY) "inf"
      (= d Double/NEGATIVE_INFINITY) "-inf"
      :else
      (let [rounded (-> (java.math.BigDecimal. d)
                        (.setScale 6 java.math.RoundingMode/HALF_EVEN)
                        (.toPlainString))]
        ;; BigDecimal drops the sign when a nonzero negative rounds to zero.
        ;; Nix preserves that sign, but canonicalizes exact -0.0 itself.
        (if (and (neg? (Math/copySign 1.0 d)) (= rounded "0.000000"))
          "-0.000000"
          rounded)))))

(defn- nix-coerce-to-string
  "Nix `toString` coercion (coerceMore=true): strings are unchanged, true -> \"1\",
  false and null -> \"\", numbers -> their digits, lists -> the space-joined
  coercion of each element, and an attrset via its outPath. Other attrsets and
  functions cannot be coerced and throw, which the caller surfaces as held.
  This is deliberately more permissive than string interpolation."
  [v]
  (let [r (force-value v)]
    (when (not= :ok (:status r))
      (throw (ex-info "cannot force value for string coercion" {:result r})))
    (let [v (:value r)]
      (cond
        (string? v) v
        (ctx-string? v)
        ;; This plain-String coercion cannot carry context; context-aware
        ;; callers (the :toString builtin) handle ctx-strings before calling.
        (throw (ex-info "string-context frontier: coercion would drop context"
                        {:reason :string-context-frontier}))
        (true? v) "1"
        (false? v) ""
        (nil? v) ""
        (path-value? v) (get v "path")
        (integer? v) (str v)
        (float? v) (nix-float-str v)
        (vector? v) (str/join " " (map nix-coerce-to-string v))
        (and (map? v) (contains? v "outPath")) (nix-coerce-to-string (get v "outPath"))
        :else (throw (ex-info "cannot coerce value to a string" {:value v}))))))

;; --- Laziness: thunks for bindings and collection slots ----------------------
;;
;; A thunk defers evaluation until a value is inspected, memoizes the result,
;; and black-holes self-reference (infinite recursion). `let` bindings, function
;; arguments, attrset values, and list elements are call-by-need. Most consumers
;; force only to weak-head normal form; public eval results are deep-realized at
;; the API boundary so existing callers still see plain Clojure values.

(defn- thunk?
  [v]
  (and (map? v) (= :thunk (:kind v))))

(defn- make-thunk
  [label compute]
  {:kind :thunk
   :label label
   :state (atom {:phase :pending})
   :compute compute})

(defn- force-value
  "Force v to an eval result map. A non-thunk becomes {:status :ok :value v};
  a thunk is computed at most once (memoized), and a thunk that forces itself is
  reported as an infinite-recursion held instead of looping forever. If a thunk
  computes to another thunk, chase the indirection to weak-head normal form."
  [v]
  (if (thunk? v)
    (let [state (:state v)]
      (case (:phase @state)
        :done (:result @state)
        :forcing (err/failed :eval
                           :infinite-recursion
                           {:label (:label v)})
        :pending
        (do
          (reset! state {:phase :forcing})
          (let [result ((:compute v))
                result' (if (and (= :ok (:status result))
                                 (thunk? (:value result)))
                          (force-value (:value result))
                          result)]
            (reset! state {:phase :done :result result'})
            result'))))
    {:status :ok :value v}))

(defn attrset-value?
  "True when value is a pnix attrset (a plain map that is not a tagged
  path/ctx-string/closure/builtin/thunk). Public: shared value predicate for
  the derived abstract machine."
  [value]
  (and (map? value)
       (not (path-value? value))
       (not (ctx-string? value))
       (not (contains? #{:builtin :closure :path :thunk} (:kind value)))))

(declare force-deep)

(defn- force-result-value
  [result]
  (if (= :ok (:status result))
    (force-value (:value result))
    result))

(defn- force-deep
  "Deep-realize a pnix value into ordinary Clojure data, propagating the first
  held result. This is for strict consumers such as equality, toJSON/deepSeq, and
  the public eval boundary; lazy list/attrset consumers should use force-value."
  [value]
  (let [forced (force-value value)]
    (if (not= :ok (:status forced))
      forced
      (let [v (:value forced)]
        (cond
          (path-value? v)
          {:status :ok :value v}

          (vector? v)
          (loop [remaining v
                 out []]
            (if (seq remaining)
              (let [item-result (force-deep (first remaining))]
                (if (= :ok (:status item-result))
                  (recur (rest remaining) (conj out (:value item-result)))
                  item-result))
              {:status :ok :value out}))

          (attrset-value? v)
          (loop [remaining (sort (keys v))
                 out {}]
            (if (seq remaining)
              (let [k (first remaining)
                    item-result (force-deep (get v k))]
                (if (= :ok (:status item-result))
                  (recur (rest remaining) (assoc out k (:value item-result)))
                  item-result))
              {:status :ok :value out}))

          :else
          {:status :ok :value v})))))

(defn- force-list-values
  [values]
  (loop [remaining values
         out []]
    (if (seq remaining)
      (let [r (force-value (first remaining))]
        (if (= :ok (:status r))
          (recur (rest remaining) (conj out (:value r)))
          r))
      {:status :ok :value out})))

(defn- force-attr
  [attrs key]
  (force-value (get attrs key)))

(defn- sorted-attr-keys
  [attrs]
  (vec (sort (keys attrs))))

(defn source-position
  "The __curPos value for a source span. Public: the derived abstract machine
  mirrors the evaluator's :var special case (which wins even over a lexical
  shadow — bug-for-bug parity) through this one definition."
  [span]
  (when span
    {"span" (vec span)
     "start" (first span)
     "end" (second span)}))

(defn- attr-positions
  [attrs]
  (::attr-positions (meta attrs)))

(defn- with-attr-position
  [attrs key span]
  (if-let [pos (source-position span)]
    (with-meta attrs
      (assoc (meta attrs)
             ::attr-positions
             (assoc (attr-positions attrs) key pos)))
    attrs))

(defn- flatten-list-result
  [value]
  (let [r (force-value value)]
    (if (not= :ok (:status r))
      r
      (let [value (:value r)]
        (if (vector? value)
          (loop [remaining value
                 out []]
            (if (seq remaining)
              (let [item-result (flatten-list-result (first remaining))]
                (if (= :ok (:status item-result))
                  (recur (rest remaining) (into out (:value item-result)))
                  item-result))
              {:status :ok :value out}))
          {:status :ok :value [value]})))))

(defn- function-args-value
  [value]
  (cond
    (and (= :closure (:kind value))
         (:param-pattern value))
    (into {}
          (map (fn [{:keys [name default]}]
                 [name (boolean default)]))
          (get-in value [:param-pattern :params]))

    (or (= :closure (:kind value))
        (= :builtin (:kind value))
        (= :host-fn (:kind value)))
    {}

    :else
    (throw (ex-info "functionArgs target is not a function"
                    {:target-kind (:kind value)}))))

(defn- callable-value?
  [value]
  (contains? #{:closure :builtin :host-fn :lazy-host-fn} (:kind value)))

(defn- eval-items
  [env items]
  {:status :ok
   :value (mapv (fn [idx item]
                  (make-thunk [:list idx] (fn [] (eval-ast* env item))))
                (range)
                items)})

(defn attr-key-value-result
  "D20 (oracle-gated): a DYNAMIC attr key value must be a STRING — real Nix
  errors ('value is an integer while a string was expected') instead of
  coercing, and an `or` default does NOT catch it. A context-carrying string
  stays behind the deny-by-default context frontier. Public: shared by the
  evaluator's eval-attr-key and the machine's dynamic-key frames."
  [v]
  (cond
    (string? v)
    {:status :ok :value v}

    (ctx-string? v)
    (err/failed :eval
              :string-context-frontier
              {:construct :dynamic-attr-key
               :policy :deny-by-default-until-context-aware})

    :else
    (err/failed :eval
              :dynamic-attr-key-not-string
              {:value-type (strict-type v)})))

(defn- eval-attr-key
  [env key]
  (cond
    (string? key)
    {:status :ok
     :value key}

    (= :dynamic-attr-key (:kind key))
    (let [result (force-result-value (eval-ast* env (:expr key)))]
      (if (= :ok (:status result))
        (attr-key-value-result (:value result))
        result))

    :else
    (err/failed :eval :unsupported-attr-key {:key key})))

(defn merge-attr-path
  "Insert value v at key path ks into attrset map m, creating nested attrsets
  and merging with existing ones. Returns [:ok merged-map] or [:err key] where
  key is the attribute that conflicts (a leaf already set, or an intermediate
  segment that is not an attrset). Public: the machine's dynamic-path frames
  share this one definition with eval-attrs."
  [m ks spans v]
  (let [k (first ks)
        span (first spans)]
    (if (empty? (rest ks))
      (if (contains? m k)
        [:err k]
        [:ok (with-attr-position (assoc m k v) k span)])
      (let [existing (get m k ::absent)]
        (cond
          (= existing ::absent)
          (let [[st sub] (merge-attr-path {} (rest ks) (rest spans) v)]
            (if (= st :ok)
              [:ok (with-attr-position (assoc m k sub) k span)]
              [st sub]))

          (attrset-value? existing)
          (let [[st sub] (merge-attr-path existing (rest ks) (rest spans) v)]
            (if (= st :ok)
              [:ok (with-attr-position (assoc m k sub) k span)]
              [st sub]))

          :else
          [:err k])))))

(defn- eval-path-keys
  "Evaluate each segment of an attr path to its string key. Returns
  {:status :ok :value [k ...]} or the first failing key result."
  [env path]
  (loop [remaining path keys []]
    (if-let [k (first remaining)]
      (let [kr (eval-attr-key env k)]
        (if (= :ok (:status kr))
          (recur (rest remaining) (conj keys (:value kr)))
          kr))
      {:status :ok :value keys})))

(defn- eval-attrs
  [env attrs recursive?]
  ;; In a `rec` set every statically-named binding is visible to all the others,
  ;; including forward references and mutual recursion (Nix `rec` is the same
  ;; recursive scope as `let`). Pre-bind each such key as a memoized thunk that
  ;; evaluates in the final env (knot-tied through env-ref), so sibling
  ;; references and recursive closures resolve against the complete set rather
  ;; than only the bindings seen so far. Path bindings and dynamic keys are not
  ;; statically named, so they are threaded incrementally via env' (and still see
  ;; the pre-bound static siblings). A non-rec set evaluates each value in the
  ;; enclosing env with no sibling visibility.
  (let [env-ref (atom nil)
        base-env (if recursive?
                   (reduce (fn [acc {:keys [key value from-enclosing]}]
                             (if (string? key)
                               (assoc acc key
                                      (make-thunk key
                                                  ;; Plain `inherit x` copies the
                                                  ;; enclosing x, so it resolves
                                                  ;; against `env`, not the rec set
                                                  ;; itself (which would self-cycle).
                                                  (fn [] (eval-ast* (if from-enclosing
                                                                      env
                                                                      @env-ref)
                                                                    value))))
                               acc))
                           env attrs)
                   env)]
    (reset! env-ref base-env)
    (loop [remaining attrs
           values {}
           env' base-env]
      (if-let [{:keys [key key-span path path-spans value]} (first remaining)]
        (if path
          ;; Dotted path binding `a.b.c = v`: merge into nested attrsets.
          (let [keys-result (eval-path-keys env' path)]
            (if (not= :ok (:status keys-result))
              keys-result
              (let [slot (make-thunk [:attr-path (:value keys-result)]
                                     (fn [] (eval-ast* env' value)))
                    [st merged] (merge-attr-path values
                                                 (:value keys-result)
                                                 path-spans
                                                 slot)]
                (if (= st :err)
                  (err/failed :eval :duplicate-attr
                            {:attr merged :path (:value keys-result)})
                  (let [top (first (:value keys-result))
                        env'' (if recursive?
                                (assoc env' top (get merged top))
                                env')]
                    (recur (rest remaining)
                           merged
                           (if recursive?
                             (do
                               (reset! env-ref env'')
                               env'')
                             env'')))))))
          ;; Single key binding (also covers inherit).
          (let [{:keys [from-enclosing]} (first remaining)
                key-result (eval-attr-key env' key)]
            (if (not= :ok (:status key-result))
              key-result
              ;; Plain `inherit x` resolves against the enclosing env; every
              ;; other binding resolves against env' (rec siblings included).
              (let [k (:value key-result)]
                ;; D20 (oracle-gated): an eval-time key collision is an ERROR
                ;; — real Nix: \"dynamic attribute 'a' already defined\". Only
                ;; dynamic keys can collide here (static-static duplicates are
                ;; parse errors since D10), and the pre-D20 assoc silently
                ;; overwrote (wrong VALUE, not just leniency).
                (if (contains? values k)
                  (err/failed :eval :duplicate-attr {:attr k})
                  (let [slot (if (and recursive? (string? key))
                               (get base-env k)
                               (make-thunk [:attr k]
                                           (fn [] (eval-ast* (if from-enclosing
                                                              env
                                                              env')
                                                            value))))
                        env'' (if recursive?
                                (assoc env' k slot)
                                env')]
                    (recur (rest remaining)
                           (with-attr-position (assoc values k slot) k key-span)
                           (if recursive?
                             (do
                               (reset! env-ref env'')
                               env'')
                             env''))))))))
        {:status :ok
         :value values}))))

(defn- eval-let
  [env bindings body]
  ;; `let` is recursive: every binding is a memoized thunk that evaluates in the
  ;; final environment (all siblings visible), so mutual recursion, forward
  ;; references, and recursive *values* work, and an unused binding is never
  ;; evaluated. The env-ref knot-ties the thunks to the env that contains them;
  ;; closures captured here see the complete env, so no per-closure env-ref hack
  ;; is needed (unlike the rec-attrset path, which still uses one).
  ;;
  ;; The body is a TAIL position, so a consecutive `let .. in let .. in ..`
  ;; CHAIN evaluates in a loop -- constant stack, matching real Nix, which
  ;; handles 10k+ deep let chains (oracle-probed; the recursive version
  ;; overflowed around 9-10k even on the deep-eval stack). Each folded level
  ;; replays eval-ast*'s per-node hooks (fuel tick + coverage) so safe-eval
  ;; and coverage semantics are byte-identical to the recursive route.
  (loop [env env
         bindings bindings
         body body]
    (let [enclosing env
          env-ref (atom nil)
          final-env (reduce (fn [acc {:keys [name value from-enclosing]}]
                              (assoc acc name
                                     (make-thunk name
                                                 ;; Plain `inherit x` resolves
                                                 ;; against the enclosing env so it
                                                 ;; copies the outer binding rather
                                                 ;; than referencing itself; every
                                                 ;; other binding (including
                                                 ;; `inherit (e) x`) is recursive.
                                                 (if from-enclosing
                                                   (fn [] (eval-ast* enclosing value))
                                                   (fn [] (eval-ast* @env-ref value))))))
                            env
                            bindings)]
      (reset! env-ref final-env)
      (if (= :let (:op body))
        (do (when-let [f *fuel*]
              (when (neg? (long (vswap! f unchecked-dec)))
                (throw (ex-info "pnix fuel exhausted" {:pnix-fuel-exhausted true}))))
            (cover! :op :let)
            (recur final-env (:bindings body) (:body body)))
        (eval-ast* final-env body)))))

(defn- pnix-less-than-result
  "Nix `<` ordering: numbers numerically, strings lexicographically, and lists
  lexicographically (element by element; a proper prefix is smaller). Returns an
  eval result so forcing a lazy element can propagate held errors."
  [a b]
  (let [ar (force-value a)
        br (when (= :ok (:status ar)) (force-value b))]
    (cond
      (not= :ok (:status ar))
      ar

      (not= :ok (:status br))
      br

      :else
      (let [a (:value ar)
            b (:value br)]
        (cond
          (and (number? a) (number? b))
          {:status :ok :value (< a b)}

          (or (raw-bytes? a) (raw-bytes? b))
          (let [xa (byte-vec-of a) xb (byte-vec-of b)]
            (if (and xa xb)
              {:status :ok :value (neg? (compare xa xb))}
              (err/failed :eval :raw-bytes-compare-non-string {:construct :<})))

          (and (string? a) (string? b))
          {:status :ok :value (neg? (compare a b))}

          (and (vector? a) (vector? b))
          (loop [xs a
                 ys b]
            (cond
              (empty? xs)
              {:status :ok :value (not (empty? ys))}

              (empty? ys)
              {:status :ok :value false}

              :else
              (let [eq-result (nix-equal-result (first xs) (first ys) true)]
                (cond
                  (not= :ok (:status eq-result))
                  eq-result

                  (:value eq-result)
                  (recur (rest xs) (rest ys))

                  :else
                  ;; Advance only when equality is true. Unequal unordered
                  ;; values (notably distinct NaNs) make list `<` false; the
                  ;; tail must not decide their order.
                  (pnix-less-than-result (first xs) (first ys))))))

          :else
          (err/failed :eval :eval-binary-failed
                    {:cause :incomparable-values
                     :left a
                     :right b}))))))

(defn- nix-equal-result
  "Nix value equality: numbers compare by value across int/float (so 1 == 1.0),
  lists and attrsets compare structurally and recursively. A recursively
  compared slot that resolves to the SAME value object is equal before scalar
  comparison (Nix's shared-value shortcut), while top-level function/NaN
  equality remains false. Everything else uses ordinary equality.
  pnix attrsets are string-keyed maps, so the keyword :kind only appears on
  closure/builtin/thunk maps, which is how callables are detected here."
  ([a b]
   (nix-equal-result a b false))
  ([a b recursive-slot?]
   (let [ar (force-value a)]
     (if (not= :ok (:status ar))
       ar
       (let [br (force-value b)]
         (if (not= :ok (:status br))
           br
           (let [a (:value ar)
                 b (:value br)]
             (cond
              (and recursive-slot? (identical? a b))
              {:status :ok :value true}

              (or (raw-bytes? a) (raw-bytes? b))
              ;; RAW-BYTE equality is byte-wise; mixed Str/Bytes compares via
              ;; bytes (oracle: substring 1 1 "가" == substring 1 1 "각").
              {:status :ok :value (= (byte-vec-of a) (byte-vec-of b))}

              (and (number? a) (number? b))
              {:status :ok :value (== a b)}

              (or (path-value? a) (path-value? b))
              {:status :ok
               :value (and (path-value? a)
                           (path-value? b)
                           (= (get a "path") (get b "path")))}

              ;; Nix string equality compares character content only; the
              ;; context rides along but does not participate in ==.
              (or (ctx-string? a) (ctx-string? b))
              {:status :ok
               :value (and (string-like? a)
                           (string-like? b)
                           (= (string-content a) (string-content b)))}

              (and (vector? a) (vector? b))
              (if (not= (count a) (count b))
                {:status :ok :value false}
                (loop [xs a ys b]
                  (if (seq xs)
                    (let [r (nix-equal-result (first xs) (first ys) true)]
                      (if (not= :ok (:status r))
                        r
                        (if (:value r)
                          (recur (rest xs) (rest ys))
                          {:status :ok :value false})))
                    {:status :ok :value true})))

              (and (map? a) (map? b))
              (if (or (:kind a) (:kind b))
                {:status :ok :value false}
                (if (or (not= (count a) (count b))
                        (not= (set (keys a)) (set (keys b))))
                  {:status :ok :value false}
                  (loop [ks (keys a)]
                    (if (seq ks)
                      (let [k (first ks)
                            r (nix-equal-result (get a k) (get b k) true)]
                        (if (not= :ok (:status r))
                          r
                          (if (:value r)
                            (recur (rest ks))
                            {:status :ok :value false})))
                      {:status :ok :value true}))))

              :else
              {:status :ok :value (= a b)}))))))))

;; ── shared VALUE semantics (evaluator ⇄ derived abstract machine) ──
;; The M7 abstract machine (pnix-clj.machine) is derived from eval-ast* by the
;; functional correspondence, which transforms CONTROL only — the value algebra
;; must be one shared definition, never a copy. These public fns are that
;; algebra for the machine's fragment; eval-binary / :not / :neg / :if call the
;; same fns, so the two lanes cannot drift on value semantics.

(defn logical-operand-result
  "D18 boolean-operand check for && || -> on an already-evaluated operand
  result: real Nix type-errors on a non-bool operand (uncatchably), checked
  when and only when that operand is evaluated. Returns the result unchanged
  when it is a boolean."
  [operator side result span]
  (or (when-not (boolean? (:value result))
        (strict-violation
         (case operator
           "&&" :non-bool-and-operand
           "||" :non-bool-or-operand
           "->" :non-bool-implies-operand)
         {:construct :binary
          :operator operator
          :issue :non-bool-operand
          :side side
          :value-type (strict-type (:value result))
          :span span}))
      result))

(defn binary-value-result
  "Value-level semantics of a non-short-circuit binary operator applied to two
  WHNF operand values (both already evaluated, in Nix's left-then-right
  order)."
  [operator left right]
  (try
    (case operator
      "+" (if (or (raw-bytes? left) (raw-bytes? right))
            ;; RAW-BYTE concat: bytes join, revalidate (may return to String).
            ;; Contextful operands cannot mix with raw bytes — held.
            (if (or (ctx-string? left) (ctx-string? right))
              (err/failed :eval :raw-bytes-with-context {:construct :+})
              (let [xa (byte-vec-of left)
                    xb (byte-vec-of right)]
                (if (and xa xb)
                  {:status :ok :value (bytes-val (into xa xb))}
                  (err/failed :eval :raw-bytes-concat-non-string {:construct :+}))))
          (if (or (string-like? left) (string-like? right))
            ;; String concatenation carries context: contents join,
            ;; contexts union (plain String when both are empty).
            (if (and (string-like? left) (string-like? right))
              {:status :ok
               :value (ctx-string (str (string-content left)
                                       (string-content right))
                                  (concat (string-ctx left)
                                          (string-ctx right)))}
              (or (strict-violation
                   :string-coercion
                   {:construct :+
                    :issue :string-coercion
                    :left-type (strict-type left)
                    :right-type (strict-type right)})
                  {:status :ok
                   :value (ctx-string (str (string-content left)
                                           (string-content right))
                                      (concat (string-ctx left)
                                              (string-ctx right)))}))
            (arithmetic-result :eval
                               {:construct :binary}
                               "+"
                               left
                               right
                               +)))
      "-" (arithmetic-result :eval
                             {:construct :binary}
                             "-"
                             left
                             right
                             -)
      "*" (arithmetic-result :eval
                             {:construct :binary}
                             "*"
                             left
                             right
                             *)
      "/" (if (and (integer? left) (integer? right))
            (production-primitive-result "/" left right)
            (div-result :eval
                        {:construct :binary}
                        left
                        right))
      "%" (mod-result :eval
                      {:construct :binary}
                      left
                      right)
      "//" {:status :ok
            :value (merge left right)}
      ;; List concatenation: BOTH operands must be lists. Clojure's
      ;; (concat xs nil) treats nil as empty — that is NOT Nix (oracle:
      ;; "expected a list but found null"). Same for string/int right.
      "++" (cond
             (not (vector? left))
             (err/failed :eval :concat-operand-not-list
                         {:operator "++"
                          :side :left
                          :value-type (strict-type left)})
             (not (vector? right))
             (err/failed :eval :concat-operand-not-list
                         {:operator "++"
                          :side :right
                          :value-type (strict-type right)})
             :else
             {:status :ok
              :value (vec (concat left right))})
      "==" (nix-equal-result left right)
      "!=" (let [r (nix-equal-result left right)]
             (if (= :ok (:status r))
               {:status :ok :value (not (:value r))}
               r))
      ;; `<` is the primitive Nix comparison; the rest derive from it
      ;; so list/string ordering stays consistent.
      "<" (pnix-less-than-result left right)
      ">" (pnix-less-than-result right left)
      "<=" (let [r (pnix-less-than-result right left)]
             (if (= :ok (:status r))
               {:status :ok :value (not (:value r))}
               r))
      ">=" (let [r (pnix-less-than-result left right)]
             (if (= :ok (:status r))
               {:status :ok :value (not (:value r))}
               r))
      (throw (ex-info "unsupported binary operator"
                      {:operator operator})))
    (catch Throwable t
      (err/failed-throwable :eval
                          :eval-binary-failed
                          t
                          {:operator operator}))))

(defn not-value-result
  "Value-level `!` on a WHNF operand (strict: bool only, like real Nix)."
  [v span]
  (or (when-not (boolean? v)
        (strict-violation
         :non-bool-not-operand
         {:construct :not
          :issue :non-bool-operand
          :value-type (strict-type v)
          :span span}))
      {:status :ok
       :value (not v)}))

(defn neg-value-result
  "Value-level unary minus on a WHNF operand."
  [v]
  ;; Unary -0.0 canonicalizes to positive zero in Nix. A negative zero made
  ;; by arithmetic bypasses this function and keeps its IEEE sign.
  (if (and (float? v) (zero? v))
    {:status :ok :value 0.0}
    (unary-integer-result :eval
                          {:construct :unary}
                          "-"
                          v
                          -)))

(defn if-condition-violation
  "Strict bool check for an `if` condition value: nil when it is a boolean,
  otherwise the held strict violation (the evaluator's and the machine's `if`
  share this one definition)."
  [v span]
  (when-not (boolean? v)
    (strict-violation
     :non-bool-if-condition
     {:construct :if
      :issue :non-bool-condition
      :value-type (strict-type v)
      :span span})))

(defn assert-condition-violation
  "Strict bool check for an `assert` condition value (shared like
  if-condition-violation)."
  [v span]
  (when-not (boolean? v)
    (strict-violation
     :non-bool-assert-condition
     {:construct :assert
      :issue :non-bool-condition
      :value-type (strict-type v)
      :span span})))

(defn interpolation-value-result
  "Value-level semantics of a `${...}` interpolation chunk: coerce one
  evaluated part to its string piece (plain, contextful, path, __toString /
  outPath attrset) or the held coercion error — shared by the evaluator's
  template joiner and the machine's :template frames."
  [v]
  (interpolation-to-string v))

(def default-env-names
  "Names the default environment binds (builtins + bare default-scope names).
  The abstract machine's closed-fragment check refuses exactly these as free
  variables — the machine deliberately has no builtins environment yet."
  (set (keys default-env)))

(defn- eval-binary
  [env {:keys [operator left right span]}]
  (cover! :binary-operator operator)
  (let [left-result (force-result-value (eval-ast* env left))
        ;; D18 (oracle-gated): && || -> require BOOLEAN operands in real Nix
        ;; ("value is an integer while a Boolean was expected", uncatchable).
        ;; Each operand is type-checked when (and only when) it is evaluated:
        ;; the left always, the right only past the short-circuit — so
        ;; false && (throw "X") stays false while true && 1 is a type error.
        ;; The pre-D18 arms leaked Clojure truthiness (host leak, R2 doctrine).
        logical-operand
        (fn [side result]
          (logical-operand-result operator side result span))]
    (if (not= :ok (:status left-result))
      left-result
      (case operator
        "&&"
        (let [checked (logical-operand :left left-result)]
          (if (not= :ok (:status checked))
            checked
            (if (:value checked)
              (do
                (cover! :branch :binary/and-right)
                (let [right-result (force-result-value (eval-ast* env right))]
                  (if (not= :ok (:status right-result))
                    right-result
                    (logical-operand :right right-result))))
              (do
                (cover! :branch :binary/and-short-circuit)
                {:status :ok
                 :value false}))))

        "||"
        (let [checked (logical-operand :left left-result)]
          (if (not= :ok (:status checked))
            checked
            (if (:value checked)
              (do
                (cover! :branch :binary/or-short-circuit)
                {:status :ok
                 :value true})
              (do
                (cover! :branch :binary/or-right)
                (let [right-result (force-result-value (eval-ast* env right))]
                  (if (not= :ok (:status right-result))
                    right-result
                    (logical-operand :right right-result)))))))

        "->"
        ;; Logical implication: a -> b is !a || b, so a false antecedent is
        ;; vacuously true and the consequent is only evaluated when a is true.
        (let [checked (logical-operand :left left-result)]
          (if (not= :ok (:status checked))
            checked
            (if (:value checked)
              (do
                (cover! :branch :binary/implies-right)
                (let [right-result (force-result-value (eval-ast* env right))]
                  (if (not= :ok (:status right-result))
                    right-result
                    (logical-operand :right right-result))))
              (do
                (cover! :branch :binary/implies-short-circuit)
                {:status :ok
                 :value true}))))

        (let [right-result (force-result-value (eval-ast* env right))]
          (if (not= :ok (:status right-result))
            right-result
            (binary-value-result operator
                                 (:value left-result)
                                 (:value right-result))))))))

(defn- eval-select
  [env {:keys [target attr default]}]
  (let [target-result (force-result-value (eval-ast* env target))
        attr-result (eval-attr-key env attr)
        miss-reasons #{:missing-attr :select-target-not-attrset}]
    (cond
      ;; `a.b or d`: the default catches a missing attr or non-attrset target,
      ;; including a missing intermediate attr on the path, but NOT other
      ;; evaluation errors (an unbound variable, say, still propagates).
      (and default
           (= :failed (:status target-result))
           (contains? miss-reasons (:reason target-result)))
      (eval-ast* env default)

      (not= :ok (:status target-result))
      target-result

      (not= :ok (:status attr-result))
      attr-result

      (and (map? (:value target-result))
           (contains? (:value target-result) (:value attr-result)))
      (let [selected-result (force-value (get (:value target-result)
                                              (:value attr-result)))]
        (if (= :ok (:status selected-result))
          (nullary-builtin-result (:value selected-result))
          selected-result))

      default
      (eval-ast* env default)

      (not (map? (:value target-result)))
      (err/failed :eval
                :select-target-not-attrset
                {:attr (:value attr-result)
                 :target-value (:value target-result)})

      :else
      (err/failed :eval
                :missing-attr
                {:attr (:value attr-result)
                 :available-attrs (vec (sort (keys (:value target-result))))}))))

(defn- eval-has-attr
  [env {:keys [target attr]}]
  (let [target-result (force-result-value (eval-ast* env target))
        attr-result (eval-attr-key env attr)]
    (cond
      (not= :ok (:status target-result))
      target-result

      (not= :ok (:status attr-result))
      attr-result

      ;; Nix `?` on a non-attrset is FALSE, not an error (oracle-confirmed,
      ;; D6: `1 ? a` => false) — unlike builtins.hasAttr, which stays strict.
      (not (attrset-value? (:value target-result)))
      {:status :ok :value false}

      :else
      {:status :ok
       :value (contains? (:value target-result) (:value attr-result))})))

(defn- apply-callable2
  [f a b]
  (let [first-result (apply-callable f a)]
    (if (not= :ok (:status first-result))
      first-result
      (apply-callable (:value first-result) b))))

(defn- recursive-update-result
  "Deep-merge two attrsets: where both sides hold an attrset for a key, recurse;
  otherwise the right side wins. Keys present only on the left are kept."
  [lhs rhs]
  (let [lhs-result (force-value lhs)
        rhs-result (force-value rhs)]
    (if (not= :ok (:status lhs-result))
      lhs-result
      (if (not= :ok (:status rhs-result))
        rhs-result
        (let [lhs (:value lhs-result)
              rhs (:value rhs-result)]
          (if (and (attrset-value? lhs) (attrset-value? rhs))
            (loop [remaining (keys rhs)
                   out lhs]
              (if (seq remaining)
                (let [k (first remaining)]
                  (if (and (contains? lhs k)
                           (contains? rhs k))
                    (let [merged (recursive-update-result (get lhs k) (get rhs k))]
                      (if (= :ok (:status merged))
                        (recur (rest remaining) (assoc out k (:value merged)))
                        merged))
                    (recur (rest remaining) (assoc out k (get rhs k)))))
                {:status :ok :value out}))
            {:status :ok :value rhs}))))))

;; --- Derivations (pure simulation, no builder/store) --------------------------
;;
;; Tvix-style separation: the evaluator models derivation VALUES (deterministic
;; pseudo store paths carrying string context) without any builder or on-disk
;; store. Paths look like /nix/store/<32-hex>-<name>(.drv) where the hex comes
;; from a sha256 of the deep-forced input attrset — deterministic, but NOT
;; byte-compatible with real Nix store hashing (documented simulation scope).
;; Context elements use Nix's encoding: a drvPath depends on itself as
;; "<drvPath>", an output path as "!<output>!<drvPath>".

(defn- derivation-uncoercible
  "Walk a deep-forced derivation input and return the first value a derivation
  attr cannot contain (a function). nil when clean."
  [v]
  (cond
    (and (map? v) (contains? #{:closure :builtin} (:kind v))) v
    (vector? v) (some derivation-uncoercible v)
    (attrset-value? v) (some derivation-uncoercible (vals v))
    :else nil))

(defn- derivation-paths
  "Deterministic pseudo drvPath and per-output paths for a deep-forced
  derivation input. Non-\"out\" outputs get the Nix-style \"-<output>\" name
  suffix (oracle: /nix/store/<h>-t vs /nix/store/<h>-t-dev)."
  [forced-attrs name outputs]
  (let [h (fn [salt] (subs (hash/data-hash [salt forced-attrs]) 0 32))]
    {:drv-path (str "/nix/store/" (h :drv) "-" name ".drv")
     :out-paths (into {}
                      (map (fn [o]
                             [o (str "/nix/store/" (h [:out o]) "-" name
                                     (when (not= o "out") (str "-" o)))]))
                      outputs)}))

(defn- derivation-core
  "Validate + realize a derivation input attrset. Returns
  {:status :ok :forced .. :name .. :drv-path .. :out-path ..} or a held."
  [builtin-name attrs]
  (if-not (attrset-value? attrs)
    (err/failed :builtin :derivation-argument-not-attrset
              {:builtin builtin-name :value-type (strict-type attrs)})
    (let [forced-result (force-deep attrs)]
      (if (not= :ok (:status forced-result))
        forced-result
        (let [forced (:value forced-result)
              name-v (get forced "name")
              missing (first (remove #(contains? forced %)
                                     ["name" "system" "builder"]))]
          (cond
            missing
            (err/failed :builtin :derivation-missing-required-attr
                      {:builtin builtin-name :attr missing})

            (not (string? name-v))
            (err/failed :builtin :derivation-name-not-plain-string
                      {:builtin builtin-name :value-type (strict-type name-v)})

            :else
            (if-let [bad (derivation-uncoercible forced)]
              (err/failed :builtin :derivation-attr-not-coercible
                        {:builtin builtin-name :value-type (strict-type bad)})
              (let [outputs (get forced "outputs" ["out"])]
                (if-not (and (vector? outputs)
                             (seq outputs)
                             (every? string? outputs)
                             (apply distinct? outputs))
                  (err/failed :builtin :derivation-invalid-outputs
                            {:builtin builtin-name :outputs outputs})
                  (let [{:keys [drv-path out-paths]}
                        (derivation-paths forced name-v outputs)]
                    {:status :ok
                     :forced forced
                     :name name-v
                     :outputs outputs
                     :drv-path drv-path
                     :out-paths out-paths}))))))))))

(def ^:private context-aware-builtins
  "Builtins allowed to receive a contextful string. Everything else is denied by
  default (held :string-context-frontier) so context is never silently dropped
  or mangled; grow this set builtin-by-builtin as each implementation learns to
  propagate context, with tests."
  #{:hasContext :getContext :hashString :unsafeDiscardStringContext
    :unsafeDiscardOutputDependency :appendContext :toPath
    :toString :typeOf :isString :isAttrs :isList :isInt :isFloat :isBool
    :isNull :isFunction :seq :deepSeq :trace :id :eq
    :derivation :derivationStrict :placeholder :storePath
    :stringLength :substring :concatStringsSep
    :hasPrefix :hasSuffix :hasInfix :toUpper :toLower :replaceStrings
    :match :split :toJSON :fromJSON :stringToCharacters :splitString
    :removePrefix :removeSuffix :toInt :concatStrings :concatMapStrings
    ;; structural list/element operations pass contextful strings through
    ;; untouched, so they are safe by construction
    :head :tail :elemAt :last :init :length :elem})

(defn- utf8-length
  "★B4 DECIDED (owner 2026-07-09): the BYTE string model — Nix counts UTF-8
  bytes (stringLength \"å\" == 2), by design (manual + Dolstra #770). JVM
  strings are UTF-16, so measure via the UTF-8 encoding."
  ^long [^String s]
  (alength (.getBytes s "UTF-8")))

(defn- utf8-substring
  "Byte-offset substring per B4: clamp start-beyond-end to \"\" and
  over-length to the end (Nix behavior); a cut that lands OFF a UTF-8 char
  boundary returns a PxBytes raw-byte value (the Nix-permitted
  intermediate), revalidated back to String when a later concat is valid."
  [^String s ^long start ^long len]
  ;; RAW-BYTE track (2026-07-11): slice the UTF-8 bytes at ANY offset; a cut
  ;; off a char boundary yields a PxBytes value (the Nix-permitted
  ;; intermediate), revalidated on construction — oracle:
  ;; substring 0 1 "가" + substring 1 2 "가" == "가".
  (let [bs (str->byte-vec s)
        n (count bs)]
    (if (>= start n)
      ""
      (let [b (min n (+ start len))]
        (bytes-val (subvec bs start b))))))

(defn- ctx-string-in-args?
  "Shallow scan (top level + one level into vectors) for contextful strings in
  builtin args. Thunk-wrapped elements are opaque here; pnix-to-string /
  nix-coerce-to-string throw as the deep backstop."
  [args]
  (boolean (some (fn [a]
                   (or (ctx-string? a)
                       (and (vector? a) (some ctx-string? a))))
                 args)))

(defn- finish-context-builtin
  "String-context and derivation builtins, dispatched before finish-builtin's
  main case (which is near the JVM method-size limit). Returns nil for any
  other builtin."
  [name args]
  (case name
    :break
    {:status :ok :value (first args)}

    :toPath
    {:status :ok :value (nix-to-path (first args))}

    :hasContext
    (let [v (first args)]
      (if (string-like? v)
        {:status :ok :value (ctx-string? v)}
        (err/failed :builtin :has-context-argument-not-string
                  {:builtin name :value-type (strict-type v)})))

    :getContext
    ;; Decode the Nix-encoded context elements back into per-path info attrsets
    ;; and merge kinds on the same path (oracle: nix-instantiate 2.34.7 —
    ;; plain "<p>" -> { path = true; }, "!o!<drv>" -> { outputs = [o..]; },
    ;; "=<drv>" -> { allOutputs = true; }; mixed kinds merge on one key).
    (let [v (first args)
          sorted-by-key (fn [m] (into {} (sort-by key m)))]
      (if (string-like? v)
        {:status :ok
         :value (sorted-by-key
                 (into {}
                       (map (fn [[k info]] [k (sorted-by-key info)]))
                 (reduce
                 (fn [acc e]
                   (cond
                     (str/starts-with? e "=")
                     (update acc (subs e 1) assoc "allOutputs" true)

                     (str/starts-with? e "!")
                     (if-let [[_ output p] (re-matches #"!([^!]*)!(.*)" e)]
                       (update-in acc [p "outputs"]
                                  (fn [outs]
                                    (vec (sort (distinct (conj (or outs []) output))))))
                       (update acc e assoc "path" true))

                     :else
                     (update acc e assoc "path" true)))
                 {}
                 (string-ctx v))))}
        (err/failed :builtin :get-context-argument-not-string
                  {:builtin name :value-type (strict-type v)})))

    :hashString
    ;; Oracle (Nix 2.34.7): hashString consumes UTF-8 bytes and returns
    ;; lowercase hex. A contextful DATA string is accepted but the digest is
    ;; context-free; a contextful ALGORITHM string is rejected because the
    ;; algorithm selector may not refer to a store path.
    (let [[algorithm data-slot] args]
      (cond
        (raw-bytes? algorithm)
        (err/failed :builtin :hash-string-raw-bytes-unsupported
                  {:builtin name :arg :algorithm})

        (not (string-like? algorithm))
        (err/failed :builtin :hash-string-algorithm-not-string
                  {:builtin name :value-type (strict-type algorithm)})

        (ctx-string? algorithm)
        (err/failed :builtin :hash-string-algorithm-has-context
                  {:builtin name})

        (not (contains? hash/nix-hash-algorithms
                        (string-content algorithm)))
        (err/failed :builtin :hash-string-unsupported-algorithm
                  {:builtin name
                   :algorithm (string-content algorithm)
                   :supported-algorithms
                   (vec (keys hash/nix-hash-algorithms))})

        :else
        (let [data-result (force-value data-slot)]
          (if (not= :ok (:status data-result))
            data-result
            (let [data (:value data-result)]
              (cond
                (raw-bytes? data)
                {:status :ok
                 :value (hash/hash-bytes
                         (string-content algorithm)
                         (byte-array
                          (map unchecked-byte (.bs ^PxBytes data))))}

                (not (string-like? data))
                (err/failed :builtin :hash-string-data-not-string
                          {:builtin name :value-type (strict-type data)})

                :else
                {:status :ok
                 :value (hash/hash-string (string-content algorithm)
                                          (string-content data))}))))))

    :unsafeDiscardStringContext
    (let [v (first args)]
      (if (string-like? v)
        {:status :ok :value (string-content v)}
        (err/failed :builtin :discard-string-context-argument-not-string
                  {:builtin name :value-type (strict-type v)})))

    :unsafeDiscardOutputDependency
    (let [v (first args)]
      (if (string-like? v)
        {:status :ok
         :value (ctx-string (string-content v)
                            (remove #(or (str/starts-with? % "!")
                                         (str/starts-with? % "="))
                                    (string-ctx v)))}
        (err/failed :builtin :discard-output-dependency-argument-not-string
                  {:builtin name :value-type (strict-type v)})))

    :appendContext
    ;; appendContext s ctx-attrset: interpret each key's info attrset into
    ;; Nix-encoded context elements — path=true -> "<p>", allOutputs=true ->
    ;; "=<p>", outputs=[o..] -> "!o!<p>". An EMPTY info attrset contributes
    ;; nothing (oracle: nix-instantiate 2.34.7 — hasContext (appendContext s
    ;; { p = {}; }) is false).
    (let [[s ctx-attrs] args]
      (cond
        (not (string-like? s))
        (err/failed :builtin :append-context-argument-not-string
                  {:builtin name :value-type (strict-type s)})

        (not (attrset-value? ctx-attrs))
        (err/failed :builtin :append-context-context-not-attrset
                  {:builtin name :value-type (strict-type ctx-attrs)})

        :else
        (let [forced (force-deep ctx-attrs)]
          (if (not= :ok (:status forced))
            forced
            (let [infos (:value forced)
                  bad (some (fn [[k info]]
                              (when-not (attrset-value? info) k))
                            infos)]
              (if bad
                (err/failed :builtin :append-context-info-not-attrset
                          {:builtin name :key bad})
                {:status :ok
                 :value (ctx-string
                         (string-content s)
                         (concat (string-ctx s)
                                 (for [[k info] infos
                                       e (concat
                                          (when (get info "path") [k])
                                          (when (get info "allOutputs")
                                            [(str "=" k)])
                                          (for [o (get info "outputs" [])]
                                            (str "!" o "!" k)))]
                                   e)))}))))))

    :derivationStrict
    ;; Low-level derivation primitive: drvPath plus one attr per output
    ;; (oracle: attrNames = ["dev" "drvPath" "out"] for outputs=["out" "dev"]),
    ;; each carrying its own string context — drvPath allOutputs ("=<drvPath>"),
    ;; an output path its own output ("!<o>!<drvPath>").
    (let [core (derivation-core name (first args))]
      (if (not= :ok (:status core))
        core
        (let [{:keys [drv-path out-paths outputs]} core]
          {:status :ok
           :value (reduce (fn [acc o]
                            (assoc acc o
                                   (ctx-string (get out-paths o)
                                               [(str "!" o "!" drv-path)])))
                          {"drvPath" (ctx-string drv-path [(str "=" drv-path)])}
                          outputs)})))

    :derivation
    ;; High-level wrapper (in Nix a nix-lang wrapper over derivationStrict):
    ;; the input attrs merged with type/name/drvPath/outPath/outputName, plus
    ;; one attr per output. outPath/outputName follow the FIRST output
    ;; (oracle: outputs=["dev" "out"] -> outputName="dev"). Each d.<o> is a
    ;; NON-cyclic reduced derivation attrset (type/name/drvPath/outPath/
    ;; outputName only) — real Nix's d.out == d self-reference is not
    ;; representable in the plain-map value model; documented simulation scope.
    (let [core (derivation-core name (first args))]
      (if (not= :ok (:status core))
        core
        (let [{:keys [forced drv-path out-paths outputs]} core
              drv-name (:name core)
              drv-ctx (ctx-string drv-path [(str "=" drv-path)])
              out-attr (fn [o]
                         (ctx-string (get out-paths o)
                                     [(str "!" o "!" drv-path)]))
              sub-drv (fn [o]
                        {"type" "derivation"
                         "name" drv-name
                         "drvPath" drv-ctx
                         "outputName" o
                         "outPath" (out-attr o)})
              first-o (first outputs)]
          {:status :ok
           :value (assoc (reduce (fn [acc o] (assoc acc o (sub-drv o)))
                                 forced
                                 outputs)
                         "type" "derivation"
                         "drvPath" drv-ctx
                         "outPath" (out-attr first-o)
                         "outputName" first-o)})))

    :placeholder
    ;; Deterministic context-free placeholder for an output name, replaced at
    ;; build time in real Nix. Pseudo hash, not byte-compatible with Nix.
    (let [output (first args)]
      (if (string? output)
        {:status :ok
         :value (str "/" (subs (hash/sha256 (str "pnix-output:" output)) 0 32))}
        (err/failed :builtin :placeholder-argument-not-string
                  {:builtin name :value-type (strict-type output)})))

    :storePath
    ;; Requires a real store; rejected in this pure evaluator like Nix's
    ;; pure-eval mode.
    (err/failed :builtin :store-path-purity-gated
              {:builtin name
               :effect :store-read
               :policy :pure-evaluator-no-store})

    ;; --- context-propagating branches: these fire only when a contextful
    ;; string is involved and return nil otherwise, so plain strings keep the
    ;; untouched legacy path in finish-builtin's main case.

    :stringLength
    (when (ctx-string? (first args))
      {:status :ok :value (utf8-length (string-content (first args)))})

    :substring
    ;; Nix keeps the ENTIRE original context on a substring (context is not
    ;; sliced). Clamp exactly like the plain-string case.
    (let [[start length s] args]
      (when (ctx-string? s)
        (or (when (neg? start)
              (strict-violation
               :substring-negative-start
               {:construct :builtin
                :builtin name
                :issue :negative-start
                :start start}))
            (let [content (string-content s)
                  slen (utf8-length content)
                  len* (if (neg? length) slen length)
                  cut (utf8-substring content start len*)]
              (if (raw-bytes? cut)
                ;; raw-byte cut of a CONTEXTFUL string: the byte value cannot
                ;; carry context — held until a use-case pins semantics.
                (err/failed :builtin :substring-raw-bytes-with-context
                          {:builtin name :start start :length length})
                {:status :ok
                 :value (ctx-string cut (string-ctx s))})))))

    (:hasPrefix :hasSuffix :hasInfix)
    ;; Content-based predicates: context does not affect the answer.
    (let [[a b] args]
      (when (or (ctx-string? a) (ctx-string? b))
        (if (and (string-like? a) (string-like? b))
          {:status :ok
           :value (let [n (string-content a)
                        s (string-content b)]
                    (case name
                      :hasPrefix (str/starts-with? s n)
                      :hasSuffix (str/ends-with? s n)
                      :hasInfix (str/includes? s n)))}
          (err/failed :builtin :string-builtin-non-string
                    {:builtin name}))))

    (:toUpper :toLower)
    ;; Case conversion keeps the context.
    (let [v (first args)]
      (when (ctx-string? v)
        {:status :ok
         :value (ctx-string ((case name
                               :toUpper str/upper-case
                               :toLower str/lower-case)
                             (string-content v))
                            (string-ctx v))}))

    :match
    ;; Oracle (nix-instantiate 2.34.7): a contextful REGEX is an error, and
    ;; captured groups from a contextful subject are context-FREE strings (Nix
    ;; collects but drops the subject's context on match results).
    (let [[pattern s] args]
      (cond
        (ctx-string? pattern)
        (err/failed :builtin :regex-argument-has-context {:builtin name})

        (ctx-string? s)
        (let [m (re-matches (nix-regex-pattern pattern) (string-content s))]
          {:status :ok
           :value (cond
                    (nil? m) nil
                    (vector? m) (vec (rest m))
                    :else [])})))

    :split
    ;; Same oracle result as :match — a contextful regex errors; pieces and
    ;; capture groups from a contextful subject come back context-free.
    (let [[pattern s] args]
      (cond
        (ctx-string? pattern)
        (err/failed :builtin :regex-argument-has-context {:builtin name})

        (ctx-string? s)
        (try
          (let [content (string-content s)
                m (re-matcher (nix-regex-pattern pattern) content)]
            (loop [result [] last-end 0]
              (if (.find m)
                (let [start (.start m)
                      end (.end m)
                      groups (mapv #(.group m %)
                                   (range 1 (inc (.groupCount m))))]
                  (recur (conj result (subs content last-end start) groups) end))
                {:status :ok
                 :value (conj result (subs content last-end))})))
          (catch Throwable t
            (err/failed-throwable :builtin :split-builtin-failed t
                                {:builtin name :pattern pattern})))))

    :toJSON
    ;; Oracle: toJSON KEEPS context — the JSON string carries the union of all
    ;; embedded strings' contexts. Strip ctx-strings to their contents for
    ;; serialization and collect their contexts onto the result. This branch
    ;; runs unconditionally (nil only when nothing forces), because contextful
    ;; strings can hide anywhere inside the structure.
    (let [r (force-deep (first args))]
      (if (not= :ok (:status r))
        r
        (let [collected (volatile! [])
              bail (fn [held] (throw (ex-info "held" {::held held})))
              strip (fn strip [v]
                      (cond
                        (ctx-string? v)
                        (do (vswap! collected into (string-ctx v))
                            (string-content v))
                        (vector? v) (mapv strip v)
                        ;; __toString wins over outPath (oracle:
                        ;; toJSON { __toString = ..; outPath = "/x"; } uses
                        ;; __toString); it is called with self and must yield
                        ;; a string, whose context is kept.
                        (and (attrset-value? v) (contains? v "__toString"))
                        (let [call (apply-callable (get v "__toString") v)]
                          (if (not= :ok (:status call))
                            (bail call)
                            (let [forced (force-value (:value call))]
                              (if (not= :ok (:status forced))
                                (bail forced)
                                (let [sv (:value forced)]
                                  (if (string-like? sv)
                                    (strip sv)
                                    (bail (err/failed :builtin
                                                    :to-json-tostring-not-string
                                                    {:builtin :toJSON
                                                     :value-type (strict-type sv)}))))))))
                        ;; An attrset with outPath serializes as that path
                        ;; (oracle: toJSON { outPath = "/x"; other = 1; } is
                        ;; "\"/x\""), so derivations become their store path
                        ;; with context kept.
                        (and (attrset-value? v) (contains? v "outPath"))
                        (strip (get v "outPath"))
                        (attrset-value? v)
                        (into {} (map (fn [[k x]] [k (strip x)])) v)
                        :else v))]
          (try
            (let [stripped (strip (:value r))]
              {:status :ok
               :value (ctx-string (json/write-json stripped) @collected)})
            (catch clojure.lang.ExceptionInfo e
              (or (::held (ex-data e)) (throw e)))))))

    :toString
    ;; Full owner of toString: Nix coerceMore coercion, context-aware — strings
    ;; keep their context, list elements' contexts are collected (oracle:
    ;; toString [ ctx "b" ] carries the context), attrsets coerce via
    ;; __toString (called with self) then outPath.
    (let [collected (volatile! [])
          bail (fn [held] (throw (ex-info "held" {::held held})))
          coerce (fn coerce [x]
                   (let [fr (force-value x)]
                     (when (not= :ok (:status fr))
                       (bail fr))
                     (let [v (:value fr)]
                       (cond
                         (string? v) v
                         (ctx-string? v) (do (vswap! collected into (string-ctx v))
                                             (string-content v))
                         (true? v) "1"
                         (false? v) ""
                         (nil? v) ""
                         (path-value? v) (get v "path")
                         (integer? v) (str v)
                         (float? v) (nix-float-str v)
                         (vector? v) (str/join " " (map coerce v))
                         (and (attrset-value? v) (contains? v "__toString"))
                         ;; the attr value may be a lazy thunk; force it to the
                         ;; callable before applying (called with self).
                         (let [f (force-value (get v "__toString"))]
                           (if (not= :ok (:status f))
                             (bail f)
                             (let [call (apply-callable (:value f) v)]
                               (if (= :ok (:status call))
                                 (coerce (:value call))
                                 (bail call)))))
                         (and (map? v) (contains? v "outPath"))
                         (coerce (get v "outPath"))
                         :else (throw (ex-info "cannot coerce value to a string"
                                               {:value v}))))))]
      (try
        (let [s (coerce (first args))]
          {:status :ok :value (ctx-string s @collected)})
        (catch clojure.lang.ExceptionInfo e
          (cond
            (:pnix-fuel-exhausted (ex-data e)) (throw e)
            (::held (ex-data e)) (::held (ex-data e))
            :else (err/failed-throwable :builtin :to-string-builtin-failed e
                                      {:builtin name})))
        (catch Throwable t
          (err/failed-throwable :builtin :to-string-builtin-failed t
                              {:builtin name}))))

    :fromJSON
    ;; Oracle: a contextful string is REJECTED ("is not allowed to refer to a
    ;; store path"); fromJSON requires a context-free string.
    (when (ctx-string? (first args))
      (err/failed :builtin :from-json-argument-has-context {:builtin name}))

    :stringToCharacters
    ;; lib.stringToCharacters is substring-based, and substring keeps the whole
    ;; context — so each character carries the source's full context.
    (let [v (first args)]
      (when (ctx-string? v)
        {:status :ok
         :value (mapv #(ctx-string (str %) (string-ctx v))
                      (string-content v))}))

    (:removePrefix :removeSuffix)
    ;; substring-based lib semantics: the result keeps s's whole context; a
    ;; contextful prefix/suffix argument only affects the comparison.
    (let [[a b] args]
      (when (or (ctx-string? a) (ctx-string? b))
        (if-not (and (string-like? a) (string-like? b))
          (err/failed :builtin :string-builtin-non-string {:builtin name})
          (let [pre (string-content a)
                s (string-content b)
                out (case name
                      :removePrefix (if (str/starts-with? s pre)
                                      (subs s (count pre))
                                      s)
                      :removeSuffix (if (str/ends-with? s pre)
                                      (subs s 0 (- (count s) (count pre)))
                                      s))]
            {:status :ok :value (ctx-string out (string-ctx b))}))))

    :toInt
    ;; Oracle: lib.toInt accepts a contextful string ("42" with context parses
    ;; to 42); the integer result carries nothing.
    (let [v (first args)]
      (when (ctx-string? v)
        (try
          {:status :ok :value (Long/parseLong (str/trim (string-content v)))}
          (catch Throwable _
            (err/failed :builtin :to-int-failed {:builtin name :value v})))))

    :splitString
    ;; lib.splitString is builtins.split-based, and split results are
    ;; context-free (oracle) — pieces come back as plain strings.
    (let [[sep s] args]
      (when (or (ctx-string? sep) (ctx-string? s))
        (let [sep-c (string-content sep)
              s-c (string-content s)]
          (if-not (and (string-like? sep) (string-like? s))
            (err/failed :builtin :string-builtin-non-string {:builtin name})
            {:status :ok
             :value (if (empty? sep-c)
                      (mapv str s-c)
                      (vec (str/split s-c
                                      (re-pattern
                                       (java.util.regex.Pattern/quote sep-c))
                                      -1)))}))))

    nil))

(defn nullary-builtin-result
  "eval-select's nullary-builtin finish as a value-level operation: a selected
  value that is a zero-arity builtin with no collected args finishes to its
  value; anything else passes through unchanged. Public: the machine's
  [:select-finish] frame shares this one definition."
  [v]
  (if (and (= :builtin (:kind v))
           (zero? (:arity v))
           (empty? (:args v)))
    (finish-builtin v)
    {:status :ok :value v}))

(defn- xml-escape
  [^String s]
  (-> (str s)
      (str/replace "&" "&amp;")
      (str/replace "<" "&lt;")
      (str/replace ">" "&gt;")
      (str/replace "\"" "&quot;")
      (str/replace "'" "&apos;")))

(defn- to-xml-value
  "Serialize a deep-forced pnix value into Nix-compatible toXML body (no header)."
  [v indent]
  (let [pad (apply str (repeat indent "  "))
        pad1 (str pad "  ")]
    (cond
      (nil? v)
      (str pad "<null />\n")

      (or (true? v) (false? v))
      (str pad "<bool value=\"" (if v "true" "false") "\" />\n")

      (integer? v)
      (str pad "<int value=\"" v "\" />\n")

      (float? v)
      (str pad "<float value=\"" v "\" />\n")

      (string? v)
      (str pad "<string value=\"" (xml-escape v) "\" />\n")

      (path-value? v)
      (str pad "<path value=\"" (xml-escape (get v "path")) "\" />\n")

      (ctx-string? v)
      (str pad "<string value=\"" (xml-escape (string-content v)) "\" />\n")

      (vector? v)
      (str pad "<list>\n"
           (apply str (map #(to-xml-value % (inc indent)) v))
           pad "</list>\n")

      (and (map? v) (contains? #{:closure :builtin :lazy-host-fn} (:kind v)))
      (let [pat (cond
                  (= :builtin (:kind v))
                  (str pad1 "<varpat name=\"" (xml-escape (name (:name v))) "\" />\n")
                  (string? (:param v))
                  (str pad1 "<varpat name=\"" (xml-escape (:param v)) "\" />\n")
                  :else
                  (str pad1 "<attrspat />\n"))]
        (str pad "<function>\n" pat pad "</function>\n"))

      (attrset-value? v)
      (str pad "<attrs>\n"
           (apply str
                  (for [k (sorted-attr-keys v)]
                    (str pad1 "<attr name=\"" (xml-escape (str k)) "\">\n"
                         (to-xml-value (get v k) (+ indent 2))
                         pad1 "</attr>\n")))
           pad "</attrs>\n")

      :else
      (str pad "<unknown />\n"))))

(defn- store-path-for
  "Content-addressed path under /tmp/pnix-nix-store for name+payload."
  [name contents-or-key]
  (let [digest (subs (hash/sha256 (str name "\0" contents-or-key)) 0 32)
        safe-name (-> (str name)
                      (str/replace #"[^A-Za-z0-9._+-]+" "-")
                      (str/replace #"^-+" ""))]
    (str pnix-store-root "/" digest "-" (if (seq safe-name) safe-name "store"))))

(defn- write-store-file!
  [store-path contents]
  (ensure-pnix-store!)
  (let [f (java.io.File. store-path)]
    (when-not (.exists f)
      (spit f (str contents)))
    store-path))

(defn- download-url!
  "Download URL bytes to a store path; returns {:status :ok :value path} or failed."
  [url name-hint]
  (try
    (let [uri (java.net.URI. (str url))
          conn (.openConnection (.toURL uri))
          _ (.setConnectTimeout conn 15000)
          _ (.setReadTimeout conn 60000)
          _ (.setRequestProperty conn "User-Agent" "pnix-clj/fetch")
          in (.getInputStream conn)
          bytes (try
                  (with-open [is in
                              baos (java.io.ByteArrayOutputStream.)]
                    (let [buf (byte-array 8192)]
                      (loop []
                        (let [n (.read is buf)]
                          (when (pos? n)
                            (.write baos buf 0 n)
                            (recur)))))
                    (.toByteArray baos))
                  (finally
                    (when (instance? java.net.HttpURLConnection conn)
                      (.disconnect ^java.net.HttpURLConnection conn))))]
      (let [hint (or name-hint
                     (let [p (.getPath uri)]
                       (if (seq p)
                         (last (str/split p #"/"))
                         "download")))
            store-path (store-path-for hint (String. ^bytes bytes "ISO-8859-1"))]
        (ensure-pnix-store!)
        (let [f (java.io.File. store-path)]
          (when-not (.exists f)
            (with-open [os (java.io.FileOutputStream. f)]
              (.write os ^bytes bytes)))
          {:status :ok :value store-path})))
    (catch Exception e
      (err/failed-throwable :builtin :fetch-failed e
                          {:url (str url)}))))

(defn- forced-attr
  "Read one attrset member in weak-head normal form. Attrset members are
  thunks, so an unforced read would hand a thunk to code that expects a value
  — and a thunk stringified into error evidence carries JVM object identity,
  which is not deterministic. Returns nil when the attr is absent or fails to
  force; the caller reports the missing/typed argument in its own vocabulary."
  [attrs k]
  (when-let [v (get attrs k)]
    (let [result (force-value v)]
      (when (= :ok (:status result))
        (:value result)))))

(defn- fetch-url-arg
  "Normalize fetchurl/fetchTarball arg (string URL or attrs with url)."
  [arg]
  (cond
    (string-like? arg) {:url (string-content arg) :name nil}
    (attrset-value? arg)
    ;; `urls` is a list, whose elements are thunks in their own right, so the
    ;; chosen element is forced again before it is read as a string.
    (let [forced-string (fn [v]
                          (let [result (force-value v)
                                v (when (= :ok (:status result)) (:value result))]
                            (when (string-like? v) (string-content v))))
          url (or (forced-attr arg "url")
                  (when-let [urls (forced-attr arg "urls")]
                    (when (vector? urls) (first urls))))]
      {:url (forced-string url)
       :name (forced-string (get arg "name"))
       :sha256 (forced-string (get arg "sha256"))
       :attrs arg})
    :else nil))

(defn- walk-attr-path
  "Walk attrs along path (list of strings). Returns {:status :ok :value v} or
  {:missing true :path walked} when a segment is absent."
  [path attrs]
  (loop [p path
         cur attrs
         walked []]
    (if (seq p)
      (let [cur-result (force-value cur)]
        (if (not= :ok (:status cur-result))
          cur-result
          (let [cur (:value cur-result)
                k (first p)]
            (if (and (map? cur) (contains? cur k))
              (recur (rest p) (get cur k) (conj walked k))
              {:status :missing :path (conj walked k)}))))
      (force-value cur))))

(defn- filter-attrs-recursive-result
  [pred attrs]
  (if-not (attrset-value? attrs)
    (err/failed :builtin :filter-attrs-recursive-not-attrset
              {:builtin :filterAttrsRecursive :value attrs})
    (loop [remaining (sorted-attr-keys attrs)
           out {}]
      (if-let [key (first remaining)]
        (let [val (get attrs key)
              pr (apply-callable2 pred key val)]
          (if (not= :ok (:status pr))
            pr
            (if (:value pr)
              (let [val-r (force-value val)]
                (if (not= :ok (:status val-r))
                  val-r
                  (if (attrset-value? (:value val-r))
                    (let [rec (filter-attrs-recursive-result pred (:value val-r))]
                      (if (= :ok (:status rec))
                        (recur (rest remaining) (assoc out key (:value rec)))
                        rec))
                    (recur (rest remaining) (assoc out key val)))))
              (recur (rest remaining) out))))
        {:status :ok :value out}))))

(defn- map-attrs-recursive-result
  [f attrs path]
  (if-not (attrset-value? attrs)
    (err/failed :builtin :map-attrs-recursive-not-attrset
              {:builtin :mapAttrsRecursive :value attrs})
    (loop [remaining (sorted-attr-keys attrs)
           out {}]
      (if-let [key (first remaining)]
        (let [val-r (force-value (get attrs key))]
          (if (not= :ok (:status val-r))
            val-r
            (let [val (:value val-r)
                  path' (conj path key)]
              (if (attrset-value? val)
                (let [rec (map-attrs-recursive-result f val path')]
                  (if (= :ok (:status rec))
                    (recur (rest remaining) (assoc out key (:value rec)))
                    rec))
                (let [mapped (apply-callable2 f path' val)]
                  (if (= :ok (:status mapped))
                    (recur (rest remaining) (assoc out key (:value mapped)))
                    mapped))))))
        {:status :ok :value out}))))

(defn- finish-extra-builtin
  "Extra builtins (toXML/toFile/fetch*/lib helpers) kept out of finish-builtin's
  main case to stay under the JVM method-size limit. Returns nil when unhandled."
  [name args]
  (case name
    :pathExists
    (if *pure-eval*
      (err/failed :builtin
                :path-exists-purity-gated
                {:builtin name
                 :effect :path-exists
                 :path (first args)
                 :policy :pure-evaluator-no-host-filesystem})
      (let [p (host-path-string (first args))
            f (java.io.File. p)]
        {:status :ok :value (.exists f)}))

    :readFile
    (if *pure-eval*
      (err/failed :builtin
                :read-file-purity-gated
                {:builtin name
                 :effect :file-read
                 :path (first args)
                 :policy :pure-evaluator-no-host-filesystem})
      (let [p (host-path-string (first args))
            f (java.io.File. p)]
        (cond
          (not (.exists f))
          (err/failed :builtin :read-file-not-found
                    {:builtin name :path p})
          (not (.isFile f))
          (err/failed :builtin :read-file-not-file
                    {:builtin name :path p})
          :else
          (try
            {:status :ok :value (slurp f)}
            (catch Exception e
              (err/failed-throwable :builtin :read-file-failed e
                                  {:builtin name :path p}))))))

    :readDir
    (if *pure-eval*
      (err/failed :builtin
                :read-dir-purity-gated
                {:builtin name
                 :effect :directory-read
                 :path (first args)
                 :policy :pure-evaluator-no-host-filesystem})
      (let [p (host-path-string (first args))
            f (java.io.File. p)]
        (cond
          (not (.exists f))
          (err/failed :builtin :read-dir-not-found
                    {:builtin name :path p})
          (not (.isDirectory f))
          (err/failed :builtin :read-dir-not-directory
                    {:builtin name :path p})
          :else
          (try
            {:status :ok
             :value (into {}
                          (for [child (.listFiles f)
                                :let [nm (.getName child)
                                      kind (cond
                                             (.isDirectory child) "directory"
                                             (.isFile child) "regular"
                                             :else "unknown")]]
                            [nm kind]))}
            (catch Exception e
              (err/failed-throwable :builtin :read-dir-failed e
                                  {:builtin name :path p}))))))

    :getEnv
    (if *pure-eval*
      (err/failed :builtin
                :get-env-purity-gated
                {:builtin name
                 :effect :env-read
                 :name (first args)
                 :policy :pure-evaluator-no-host-environment})
      {:status :ok
       :value (or (System/getenv (str (first args))) "")})

    :toXML
    (let [r (force-deep (first args))]
      (if (not= :ok (:status r))
        r
        {:status :ok
         :value (str "<?xml version='1.0' encoding='utf-8'?>\n"
                     "<expr>\n"
                     (to-xml-value (:value r) 1)
                     "</expr>\n")}))

    :toFile
    (if *pure-eval*
      (err/failed :builtin :to-file-purity-gated
                {:builtin name :effect :file-write
                 :policy :pure-evaluator-no-host-filesystem})
      (let [[fname contents] args
            fname (str fname)
            contents (str contents)
            store-path (store-path-for fname contents)]
        (try
          {:status :ok :value (write-store-file! store-path contents)}
          (catch Exception e
            (err/failed-throwable :builtin :to-file-failed e
                                {:builtin name :name fname})))))

    :getAttrFromPath
    (let [[path attrs] args
          path-result (force-list-values path)]
      (if (not= :ok (:status path-result))
        path-result
        (let [walked (walk-attr-path (:value path-result) attrs)]
          (if (= :missing (:status walked))
            (err/failed :eval :attribute-missing
                      {:attr (str/join "." (:path walked))
                       :path (:path walked)})
            walked))))

    :hasAttrByPath
    (let [[path attrs] args
          path-result (force-list-values path)]
      (if (not= :ok (:status path-result))
        path-result
        (let [walked (walk-attr-path (:value path-result) attrs)]
          (if (= :missing (:status walked))
            {:status :ok :value false}
            (if (= :ok (:status walked))
              {:status :ok :value true}
              walked)))))

    :warn
    (do
      (binding [*out* *err*]
        (println (str "evaluation warning: " (first args))))
      {:status :ok :value (second args)})

    :fetchurl
    (if *pure-eval*
      (err/failed :builtin :fetchurl-purity-gated
                {:builtin name :effect :network-fetch
                 :policy :pure-evaluator-no-network})
      (let [spec (fetch-url-arg (first args))]
        (if (or (nil? spec) (nil? (:url spec)))
          (err/failed :builtin :fetchurl-bad-arg
                    {:builtin name :arg (first args)})
          (download-url! (:url spec) (:name spec)))))

    :fetchTarball
    (if *pure-eval*
      (err/failed :builtin :fetch-tarball-purity-gated
                {:builtin name :effect :network-fetch
                 :policy :pure-evaluator-no-network})
      (let [spec (fetch-url-arg (first args))]
        (if (or (nil? spec) (nil? (:url spec)))
          (err/failed :builtin :fetch-tarball-bad-arg
                    {:builtin name :arg (first args)})
          ;; Minimal: download the archive as a store path (no unpack).
          (download-url! (:url spec) (or (:name spec) "tarball")))))

    :fetchGit
    (if *pure-eval*
      (err/failed :builtin :fetch-git-purity-gated
                {:builtin name :effect :network-fetch
                 :policy :pure-evaluator-no-network})
      (let [arg (first args)
            attrs (if (string-like? arg)
                    {"url" (string-content arg)}
                    (if (attrset-value? arg) arg nil))]
        (if (nil? attrs)
          (err/failed :builtin :fetch-git-bad-arg
                    {:builtin name :arg arg})
          (let [url (forced-attr attrs "url")
                url (when (string-like? url) (string-content url))
                rev (or (let [r (or (forced-attr attrs "rev")
                                    (forced-attr attrs "ref"))]
                          (when (string-like? r) (string-content r)))
                        "HEAD")]
            (if (nil? url)
              (err/failed :builtin :fetch-git-bad-arg
                          {:builtin name :attr "url"})
              (let [key (str url "\0" rev)
                    store-path (store-path-for "git-src" key)]
            (ensure-pnix-store!)
            (let [f (java.io.File. store-path)]
              (if (.exists f)
                {:status :ok
                 :value {"outPath" store-path
                         "rev" rev
                         "url" url
                         "shortRev" (subs rev 0 (min 7 (count rev)))}}
                ;; Best-effort shallow clone; on failure hold with a stable reason.
                (try
                  (let [tmp (java.io.File. (str store-path ".tmp"))
                        _ (when (.exists tmp)
                            (doseq [c (reverse (file-seq tmp))]
                              (.delete c)))
                        cmd (if (and rev (not= rev "HEAD") (not (str/blank? rev)))
                              ["git" "clone" "--depth" "1" (str "--branch=" rev)
                               url (.getPath tmp)]
                              ["git" "clone" "--depth" "1" url (.getPath tmp)])
                        pb (doto (ProcessBuilder. ^java.util.List cmd)
                             (.redirectErrorStream true))
                        proc (.start pb)
                        _ (.waitFor proc)
                        ok (zero? (.exitValue proc))]
                    (if (and ok (.exists tmp))
                      (do
                        (.renameTo tmp f)
                        {:status :ok
                         :value {"outPath" store-path
                                 "rev" rev
                                 "url" url
                                 "shortRev" (subs rev 0 (min 7 (count rev)))}})
                      (do
                        (spit (java.io.File. (str store-path ".meta"))
                              (pr-str {:url url :rev rev}))
                        (err/failed :builtin :fetch-git-failed
                                  {:builtin name :url url :rev rev
                                   :policy :network-or-git-unavailable}))))
                  (catch Exception e
                    (err/failed-throwable :builtin :fetch-git-failed e
                                        {:builtin name :url url :rev rev})))))))))))

    :implies
    {:status :ok :value (boolean (or (not (first args)) (second args)))}

    :optionalAttrs
    {:status :ok :value (if (first args) (second args) {})}

    :when
    {:status :ok :value (when (first args) (second args))}

    :const
    {:status :ok :value (first args)}

    :fix
    (let [f (first args)
          x-atom (atom nil)
          x (make-thunk :fix (fn [] (apply-callable f @x-atom)))]
      (reset! x-atom x)
      (force-value x))

    :sum
    (let [xs (first args)]
      (if-not (vector? xs)
        (err/failed :builtin :sum-not-list {:builtin name :arg xs})
        (loop [remaining xs acc 0]
          (if (seq remaining)
            (let [r (force-value (first remaining))]
              (if (not= :ok (:status r))
                r
                (recur (rest remaining) (+ acc (:value r)))))
            {:status :ok :value acc}))))

    :product
    (let [xs (first args)]
      (if-not (vector? xs)
        (err/failed :builtin :product-not-list {:builtin name :arg xs})
        (loop [remaining xs acc 1]
          (if (seq remaining)
            (let [r (force-value (first remaining))]
              (if (not= :ok (:status r))
                r
                (recur (rest remaining) (* acc (:value r)))))
            {:status :ok :value acc}))))

    :updateManyAttrs
    ;; updateManyAttrs updates base: fold shallow merge of each attrset in list onto base.
    (let [[updates base] args
          updates-r (force-list-values updates)]
      (if (not= :ok (:status updates-r))
        updates-r
        (loop [remaining (:value updates-r)
               acc base]
          (if (seq remaining)
            (let [item-r (force-value (first remaining))]
              (if (not= :ok (:status item-r))
                item-r
                (recur (rest remaining) (merge acc (:value item-r)))))
            {:status :ok :value acc}))))

    :zipLists
    (let [[xs ys] args]
      (if-not (and (vector? xs) (vector? ys))
        (err/failed :builtin :zip-lists-not-list {:builtin name})
        {:status :ok
         :value (mapv (fn [a b] {"fst" a "snd" b}) xs ys)}))

    :getName
    (let [x (first args)]
      (cond
        (string-like? x)
        {:status :ok :value (get (version/parse-drv-name (string-content x)) "name")}
        (attrset-value? x)
        (if-let [pname (get x "pname")]
          {:status :ok :value pname}
          (if-let [n (get x "name")]
            (let [nr (force-value n)]
              (if (not= :ok (:status nr))
                nr
                {:status :ok
                 :value (get (version/parse-drv-name (str (:value nr))) "name")}))
            (err/failed :builtin :get-name-missing {:builtin name})))
        :else
        (err/failed :builtin :get-name-bad-arg {:builtin name :arg x})))

    :getVersion
    (let [x (first args)]
      (cond
        (string-like? x)
        {:status :ok :value (get (version/parse-drv-name (string-content x)) "version")}
        (attrset-value? x)
        (if-let [ver (get x "version")]
          (force-value ver)
          (if-let [n (get x "name")]
            (let [nr (force-value n)]
              (if (not= :ok (:status nr))
                nr
                {:status :ok
                 :value (get (version/parse-drv-name (str (:value nr))) "version")}))
            (err/failed :builtin :get-version-missing {:builtin name})))
        :else
        (err/failed :builtin :get-version-bad-arg {:builtin name :arg x})))

    :getAttrFromPathOr
    ;; README order: attrs path default
    (let [[attrs path default] args
          path-result (force-list-values path)]
      (if (not= :ok (:status path-result))
        path-result
        (let [walked (walk-attr-path (:value path-result) attrs)]
          (if (= :missing (:status walked))
            {:status :ok :value default}
            walked))))

    :filterAttrsRecursive
    (let [[pred attrs] args]
      (filter-attrs-recursive-result pred attrs))

    :mapAttrsRecursive
    (let [[f attrs] args]
      (map-attrs-recursive-result f attrs []))

    :intersectLists
    ;; lib.intersectLists e list = filter (x: elem x e) list
    (let [[e xs] args]
      (if-not (and (vector? e) (vector? xs))
        (err/failed :builtin :intersect-lists-not-list {:builtin name})
        (loop [remaining xs
               out []]
          (if (seq remaining)
            (let [item (first remaining)
                  found? (loop [es e]
                           (if (seq es)
                             (let [eq (nix-equal-result item (first es))]
                               (if (not= :ok (:status eq))
                                 eq
                                 (if (:value eq) true (recur (rest es)))))
                             false))]
              (if (map? found?)
                found?
                (recur (rest remaining) (cond-> out found? (conj item)))))
            {:status :ok :value out}))))

    :subtractLists
    ;; subtractLists xs e: items of xs not present in e (xs - e).
    ;; README ergonomics: lib.subtractLists [1 2 3] [2] => [1 3].
    (let [[xs e] args]
      (if-not (and (vector? e) (vector? xs))
        (err/failed :builtin :subtract-lists-not-list {:builtin name})
        (loop [remaining xs
               out []]
          (if (seq remaining)
            (let [item (first remaining)
                  found? (loop [es e]
                           (if (seq es)
                             (let [eq (nix-equal-result item (first es))]
                               (if (not= :ok (:status eq))
                                 eq
                                 (if (:value eq) true (recur (rest es)))))
                             false))]
              (if (map? found?)
                found?
                (recur (rest remaining) (cond-> out (not found?) (conj item)))))
            {:status :ok :value out}))))

    :assertMsg
    (if (first args)
      {:status :ok :value true}
      (message-error-result :throw (second args)))

    :zipAttrs
    ;; zipAttrs = zipAttrsWith (_: values: values)
    (let [rows (first args)
          rows-result (force-list-values rows)]
      (if (not= :ok (:status rows-result))
        rows-result
        (let [rows (:value rows-result)
              attr-names (->> rows (mapcat keys) distinct sort vec)]
          (loop [remaining attr-names
                 out {}]
            (if-let [attr-name (first remaining)]
              (let [values (reduce (fn [acc row]
                                     (cond-> acc
                                       (contains? row attr-name)
                                       (conj (get row attr-name))))
                                   []
                                   rows)]
                (recur (rest remaining) (assoc out attr-name values)))
              {:status :ok :value out})))))

    nil))

(defn- finish-builtin
  [{:keys [name args]}]
  (cover! :builtin name)
  (try
    (if (and (ctx-string-in-args? args)
             (not (contains? context-aware-builtins name)))
      (err/failed :builtin
                :string-context-frontier
                {:builtin name
                 :policy :deny-by-default-until-context-aware})
    (or
     (finish-context-builtin name args)
     (finish-extra-builtin name args)
     (case name
      :attrNames
      {:status :ok
       :value (sorted-attr-keys (first args))}

      :add
      (arithmetic-result :builtin
                         {:builtin name}
                         "+"
                         (first args)
                         (second args)
                         +)

      :sub
      (arithmetic-result :builtin
                         {:builtin name}
                         "-"
                         (first args)
                         (second args)
                         -)

      :mul
      (arithmetic-result :builtin
                         {:builtin name}
                         "*"
                         (first args)
                         (second args)
                         *)

      :div
      (div-result :builtin
                  {:builtin name}
                  (first args)
                  (second args))

      :mod
      ;; Nix builtins.mod is C `%` (truncated remainder), paired with the
      ;; truncating `div` (math/div uses `quot`). Clojure `mod` is floored and
      ;; disagrees with `quot` on negative operands, so use `rem`.
      (mod-result :builtin
                  {:builtin name}
                  (first args)
                  (second args))

      :neg
      (unary-integer-result :builtin
                            {:builtin name}
                            "-"
                            (first args)
                            -)

      :abs
      (unary-integer-result :builtin
                            {:builtin name}
                            "abs"
                            (first args)
                            (fn [value]
                              (if (neg? value) (- value) value)))

      :pow
      {:status :ok
       :value (if (and (integer? (first args))
                       (integer? (second args)))
                (long (Math/pow (double (first args)) (double (second args))))
                (Math/pow (double (first args)) (double (second args))))}

      :sqrt
      {:status :ok
       :value (Math/sqrt (double (first args)))}

      :floor
      (rounding-builtin-result name (first args))

      :ceil
      (rounding-builtin-result name (first args))

      :exp
      {:status :ok
       :value (Math/exp (double (first args)))}

      :ln
      {:status :ok
       :value (Math/log (double (first args)))}

      :sin
      {:status :ok
       :value (Math/sin (double (first args)))}

      :cos
      {:status :ok
       :value (Math/cos (double (first args)))}

      :atan2
      {:status :ok
       :value (Math/atan2 (double (first args)) (double (second args)))}

      :compareVersions
      {:status :ok
       :value (version/compare-versions (first args) (second args))}

      :splitVersion
      {:status :ok
       :value (version/split-version (first args))}

      :parseDrvName
      {:status :ok
       :value (version/parse-drv-name (first args))}

      :tryEval
      {:status :ok
       :value {"success" true
               "value" (first args)}}

      :seq
      {:status :ok
       :value (second args)}

      :deepSeq
      (let [r (force-deep (first args))]
        (if (= :ok (:status r))
          {:status :ok :value (second args)}
          r))

      :trace
      {:status :ok
       :value (second args)}

      :functionArgs
      {:status :ok
       :value (function-args-value (first args))}

      :and
      {:status :ok
       :value (boolean (and (first args) (second args)))}

      :or
      {:status :ok
       :value (boolean (or (first args) (second args)))}

      :not
      {:status :ok
       :value (not (first args))}

      :eq
      (nix-equal-result (first args) (second args))

      :lt
      (pnix-less-than-result (first args) (second args))

      :le
      (let [r (pnix-less-than-result (second args) (first args))]
        (if (= :ok (:status r))
          {:status :ok :value (not (:value r))}
          r))

      :gt
      (pnix-less-than-result (second args) (first args))

      :ge
      (let [r (pnix-less-than-result (first args) (second args))]
        (if (= :ok (:status r))
          {:status :ok :value (not (:value r))}
          r))

      :lessThan
      (pnix-less-than-result (first args) (second args))

      :throw
      (message-error-result :throw (first args))

      :abort
      (message-error-result :abort (first args))

      :length
      (let [xs (first args)]
        (if (vector? xs)
          {:status :ok :value (count xs)}
          (err/failed :builtin :length-not-list {:builtin name :arg xs})))

      :stringLength
      (or (audit-string-arg name :s (first args))
          {:status :ok
           :value (let [v (first args)]
                    (if (raw-bytes? v)
                      (count (.bs ^PxBytes v))
                      (utf8-length v)))})

      :typeOf
      {:status :ok
       :value (cond
                (nil? (first args)) "null"
                (path-value? (first args)) "path"
                (string-like? (first args)) "string"
                (or (true? (first args)) (false? (first args))) "bool"
                (integer? (first args)) "int"
                (float? (first args)) "float"
                (vector? (first args)) "list"
                (map? (first args)) (if (contains? #{:builtin :closure
                                                     :lazy-host-fn}
                                                   (:kind (first args)))
                                      "lambda"
                                      "set")
                :else "unknown")}

      :baseNameOf
      {:status :ok
       :value (last (str/split (path-or-string (first args)) #"/"))}

      ;; pathExists/readFile/readDir/getEnv handled in finish-extra-builtin

      :pnixMounts
      (err/failed :builtin
                :pnix-mounts-extension-not-wired
                {:builtin name
                 :nix-builtin? false
                 :extension :pnix-mount-runtime
                 :policy :non-faithful-extension-not-nix-coverage})

      :isAttrs
      ;; Closures/builtins are represented as tagged maps, so a bare map? check
      ;; would call functions attrsets; attrset-value? excludes the :kind tags.
      {:status :ok
       :value (attrset-value? (first args))}

      :isBool
      {:status :ok
       :value (or (true? (first args))
                  (false? (first args)))}

      :isInt
      {:status :ok
       :value (integer? (first args))}

      :isFunction
      {:status :ok
       :value (contains? #{:builtin :closure :lazy-host-fn}
                         (:kind (first args)))}

      :isFloat
      {:status :ok
       :value (float? (first args))}

      :isList
      {:status :ok
       :value (vector? (first args))}

      :isNull
      {:status :ok
       :value (nil? (first args))}

      :isPath
      {:status :ok
       :value (path-value? (first args))}

      :isString
      {:status :ok
       :value (string-like? (first args))}

      ;; :toString is handled context-aware in finish-context-builtin.

      ;; :toJSON is handled context-aware in finish-context-builtin.

      :fromJSON
      (try
        {:status :ok
         :value (json/read-json (first args))}
        (catch Throwable t
          (err/failed-throwable :builtin
                              :from-json-builtin-failed
                              t
                              {:builtin name
                               :input (first args)})))

      :hasAttr
      {:status :ok
       :value (contains? (second args) (first args))}

      :getAttr
      (if (contains? (second args) (first args))
        (force-attr (second args) (first args))
        (err/failed :builtin
                  :get-attr-missing
                  {:builtin name
                   :attr (first args)
                   :available-attrs (vec (sort (keys (second args))))}))

      :get
      (if (contains? (first args) (second args))
        (force-attr (first args) (second args))
        (err/failed :builtin
                  :get-builtin-missing
                  {:builtin name
                   :attr (second args)
                   :available-attrs (sorted-attr-keys (first args))}))

      :set
      {:status :ok
       :value (assoc (first args) (second args) (nth args 2))}

      :keys
      {:status :ok
       :value (sorted-attr-keys (first args))}

      :values
      (let [attrs (first args)]
        {:status :ok
         :value (mapv attrs (sorted-attr-keys attrs))})

      :merge
      {:status :ok
       :value (merge (first args) (second args))}

      :intersectAttrs
      (let [[left right] args]
        {:status :ok
         :value (into {}
                      (filter (fn [[key _]]
                                (contains? left key))
                              right))})

      :removeAttrs
      (let [[attrs names] args]
        (if (not (attrset-value? attrs))
          (err/failed :builtin
                    :remove-attrs-first-argument-not-attrset
                    {:builtin name
                     :value attrs})
          (if (not (vector? names))
            (err/failed :builtin
                      :remove-attrs-second-argument-not-list
                      {:builtin name
                       :value names})
            (let [names-result (force-list-values names)]
              (if (not= :ok (:status names-result))
                names-result
                (let [names (:value names-result)]
                  (if (some #(not (string? %)) names)
                    (err/failed :builtin
                              :remove-attrs-name-not-string
                              {:builtin name
                               :names names})
                    {:status :ok
                     :value (apply dissoc attrs names)})))))))

      :head
      (let [xs (first args)]
        (cond
          (not (vector? xs)) (err/failed :builtin :head-not-list {:builtin name :arg xs})
          (empty? xs) (err/failed :builtin :head-of-empty-list {:builtin name})
          :else (force-value (first xs))))

      :tail
      (let [xs (first args)]
        (cond
          (not (vector? xs)) (err/failed :builtin :tail-not-list {:builtin name :arg xs})
          (empty? xs) (err/failed :builtin :tail-of-empty-list {:builtin name})
          :else {:status :ok :value (vec (rest xs))}))

      :append
      {:status :ok
       :value (vec (concat (first args) (second args)))}

      :take
      (if-not (vector? (second args))
        (err/failed :builtin :take-arg-not-list {:builtin name :arg (second args)})
        {:status :ok
         :value (vec (take (first args) (second args)))})

      :drop
      (if-not (vector? (second args))
        (err/failed :builtin :drop-arg-not-list {:builtin name :arg (second args)})
        {:status :ok
         :value (vec (drop (first args) (second args)))})

      :reverseList
      (if-not (vector? (first args))
        (err/failed :builtin :reverse-list-arg-not-list {:builtin name :arg (first args)})
        {:status :ok
         :value (vec (reverse (first args)))})

      :zip
      {:status :ok
       :value (mapv vector (first args) (second args))}

      :flatten
      (let [r (flatten-list-result (first args))]
        (if (= :ok (:status r))
          {:status :ok :value (vec (:value r))}
          r))

      :cons
      {:status :ok
       :value (vec (cons (first args) (second args)))}

      :elemAt
      (if-not (vector? (first args))
        (err/failed :builtin :elem-at-arg-not-list {:builtin name :arg (first args)})
        (force-value (nth (first args) (second args))))

      :elem
      (loop [remaining (second args)]
        (if (seq remaining)
          (let [r (nix-equal-result (first args) (first remaining) true)]
            (if (not= :ok (:status r))
              r
              (if (:value r)
                {:status :ok :value true}
                (recur (rest remaining)))))
          {:status :ok :value false}))

      :find
      (loop [remaining (second args)]
        (if (seq remaining)
          (let [r (nix-equal-result (first args) (first remaining))]
            (if (not= :ok (:status r))
              r
              (if (:value r)
                (force-value (first remaining))
                (recur (rest remaining)))))
          {:status :ok :value nil}))

      :concatStringsSep
      ;; Context-aware: contents join, contexts of the separator and every
      ;; element union (a context-free result stays a plain String).
      (or (audit-string-arg name :separator (first args))
          (audit-list-arg name :strings (second args))
          (let [strings-result (force-list-values (second args))]
            (if (not= :ok (:status strings-result))
              strings-result
              (or (audit-string-list name :strings (:value strings-result))
                  (let [sep (first args)
                        elements (:value strings-result)
                        ;; RAW-BYTE aware: concat at the BYTE level so per-byte
                        ;; fragments (substring cuts, e.g. the harvested lexer's
                        ;; char_at) reassemble; revalidate -> String when valid.
                        el-bytes (fn [v] (if (raw-bytes? v) (.bs ^PxBytes v)
                                             (str->byte-vec (string-content v))))
                        sep-b (el-bytes sep)
                        joined (loop [xs elements first? true acc []]
                                 (if (empty? xs) acc
                                     (recur (rest xs) false
                                            (into (if first? acc (into acc sep-b))
                                                  (el-bytes (first xs))))))
                        val (bytes-val joined)]
                    {:status :ok
                     :value (if (raw-bytes? val) val
                                (ctx-string val
                                            (into (vec (string-ctx sep))
                                                  (mapcat string-ctx) elements)))})))))

      :concatStrings
      (or (audit-list-arg name :strings (first args))
          (let [strings-result (force-list-values (first args))]
            (if (not= :ok (:status strings-result))
              strings-result
              (or (audit-string-list name :strings (:value strings-result))
                  ;; Context-aware: contents join, element contexts union.
                  (let [elements (:value strings-result)
                        el-bytes (fn [v] (if (raw-bytes? v) (.bs ^PxBytes v)
                                             (str->byte-vec (string-content v))))
                        joined (reduce (fn [acc v] (into acc (el-bytes v))) [] elements)
                        val (bytes-val joined)]
                    {:status :ok
                     :value (if (raw-bytes? val) val
                                (ctx-string val (into [] (mapcat string-ctx) elements)))})))))

      :hasPrefix
      (or (audit-string-arg name :prefix (first args))
          (audit-string-arg name :s (second args))
          {:status :ok
           :value (str/starts-with? (str (second args)) (str (first args)))})

      :hasSuffix
      (or (audit-string-arg name :suffix (first args))
          (audit-string-arg name :s (second args))
          {:status :ok
           :value (str/ends-with? (str (second args)) (str (first args)))})

      :bitAnd
      {:status :ok :value (bit-and (first args) (second args))}

      :bitOr
      {:status :ok :value (bit-or (first args) (second args))}

      :bitXor
      {:status :ok :value (bit-xor (first args) (second args))}

      :foldr
      ;; right fold: foldr f z [a b c] = (f a (f b (f c z)))
      (let [[f z xs] args]
        (loop [remaining (reverse xs)
               acc z]
          (if (seq remaining)
            (let [item (first remaining)
                  result (apply-callable2 f item acc)]
              (if (= :ok (:status result))
                (recur (rest remaining) (:value result))
                result))
            {:status :ok :value acc})))

      :attrByPath
      ;; attrByPath path default attrs: walk attrs along the string path,
      ;; returning default if any segment is missing or not an attrset.
      (let [[path default attrs] args
            path-result (force-list-values path)]
        (if (not= :ok (:status path-result))
          path-result
          (loop [p (:value path-result)
                 cur attrs]
            (if (seq p)
              (let [cur-result (force-value cur)]
                (if (not= :ok (:status cur-result))
                  cur-result
                  (let [cur (:value cur-result)]
                    (if (and (map? cur) (contains? cur (first p)))
                      (recur (rest p) (get cur (first p)))
                      {:status :ok :value default}))))
              (force-value cur)))))

      :dirOf
      {:status :ok
       :value (let [arg (first args)
                    path? (path-value? arg)
                    s (path-or-string arg)
                    i (str/last-index-of s "/")]
                ((if path? path-value identity)
                 (cond
                   (nil? i) "."
                   (zero? i) "/"
                   :else (subs s 0 i))))}

      :mapAttrsToList
      ;; mapAttrsToList f attrs: (f key value) for each attr, in sorted-key order.
      (let [[f attrs] args]
        (if-not (callable-value? f)
          (err/failed :eval
                    :call-target-not-callable
                    {:builtin name
                     :target-kind (:kind f)})
          {:status :ok
           :value (mapv (fn [key]
                          (make-thunk [:mapAttrsToList key]
                            (fn []
                              (apply-callable2 f key (get attrs key)))))
                        (sorted-attr-keys attrs))}))

      :optional
      {:status :ok
       :value (if (first args) [(second args)] [])}

      :optionals
      {:status :ok
       :value (if (first args) (second args) [])}

      :toLower
      (or (audit-string-arg name :s (first args))
          {:status :ok
           :value (str/lower-case (str (first args)))})

      :toUpper
      (or (audit-string-arg name :s (first args))
          {:status :ok
           :value (str/upper-case (str (first args)))})

      :stringToCharacters
      (or (audit-string-arg name :s (first args))
          {:status :ok
           :value (mapv str (str (first args)))})

      :range
      ;; range from to: the inclusive integer list [from .. to] ([] if from > to).
      {:status :ok
       :value (vec (range (first args) (inc (second args))))}

      :last
      (let [xs (first args)]
        (cond
          (not (vector? xs)) (err/failed :builtin :last-not-list {:builtin name :arg xs})
          (empty? xs) (err/failed :builtin :last-of-empty-list {:builtin name})
          :else (force-value (last xs))))

      :init
      (let [xs (first args)]
        (cond
          (not (vector? xs)) (err/failed :builtin :init-not-list {:builtin name :arg xs})
          (empty? xs) (err/failed :builtin :init-of-empty-list {:builtin name})
          :else {:status :ok :value (vec (butlast xs))}))

      :unique
      (if-not (vector? (first args))
        (err/failed :builtin :unique-arg-not-list {:builtin name :arg (first args)})
        (letfn [(seen-result [item seen]
                  (loop [remaining seen]
                    (if (seq remaining)
                      (let [r (nix-equal-result item (first remaining))]
                        (if (not= :ok (:status r))
                          r
                          (if (:value r)
                            {:status :ok :value true}
                            (recur (rest remaining)))))
                      {:status :ok :value false})))]
          (loop [remaining (first args)
                 out []]
            (if (seq remaining)
              (let [r (seen-result (first remaining) out)]
                (if (not= :ok (:status r))
                  r
                  (recur (rest remaining)
                         (if (:value r) out (conj out (first remaining))))))
              {:status :ok :value out}))))

      :concatMapStrings
      ;; concatMapStrings f xs = concatStrings (map f xs); context-aware —
      ;; contexts of the mapped results union onto the output.
      (let [[f xs] args]
        (loop [remaining xs
               acc ""
               ctxs []]
          (if (seq remaining)
            (let [result (apply-callable f (first remaining))]
              (if (= :ok (:status result))
                (let [v (:value result)]
                  (recur (rest remaining)
                         (str acc (pnix-to-string (string-content v)))
                         (into ctxs (string-ctx v))))
                result))
            {:status :ok :value (ctx-string acc ctxs)})))

      :splitString
      ;; splitString sep s: split on the literal separator, keeping empty pieces
      ;; (an empty separator splits into characters).
      (let [[sep s] args
            sep (str sep)
            s (str s)]
        (or (audit-string-arg name :separator (first args))
            (audit-string-arg name :s (second args))
            {:status :ok
             :value (if (empty? sep)
                      (mapv str s)
                      (vec (str/split s
                                      (re-pattern (java.util.regex.Pattern/quote sep))
                                      -1)))}))

      :id
      {:status :ok :value (first args)}

      :replicate
      {:status :ok :value (vec (repeat (first args) (second args)))}

      :recursiveUpdate
      (recursive-update-result (first args) (second args))

      :hasInfix
      (or (audit-string-arg name :needle (first args))
          (audit-string-arg name :s (second args))
          {:status :ok
           :value (str/includes? (str (second args)) (str (first args)))})

      :count
      ;; count pred xs: how many items satisfy pred.
      (let [[pred xs] args]
        (loop [remaining xs
               n 0]
          (if (seq remaining)
            (let [result (apply-callable pred (first remaining))]
              (if (= :ok (:status result))
                (recur (rest remaining) (if (:value result) (inc n) n))
                result))
            {:status :ok :value n})))

      :zipListsWith
      ;; zipListsWith f xs ys: [(f x0 y0) (f x1 y1) ...] up to the shorter list.
      (let [[f xs ys] args]
        (if-not (callable-value? f)
          (err/failed :eval
                    :call-target-not-callable
                    {:builtin name
                     :target-kind (:kind f)})
          {:status :ok
           :value (mapv (fn [idx item-a item-b]
                          (make-thunk [:zipListsWith idx]
                            (fn []
                              (apply-callable2 f item-a item-b))))
                        (range)
                        xs
                        ys)}))

      :boolToString
      {:status :ok :value (if (first args) "true" "false")}

      :pipe
      ;; pipe v [f g h] = h (g (f v))
      (let [[v fns] args]
        (loop [remaining fns
               acc v]
          (if (seq remaining)
            (let [fn-result (force-value (first remaining))]
              (if (not= :ok (:status fn-result))
                fn-result
                (let [result (apply-callable (:value fn-result) acc)]
                  (if (= :ok (:status result))
                    (recur (rest remaining) (:value result))
                    result))))
            {:status :ok :value acc})))

      :findFirst
      ;; findFirst pred default xs: first item satisfying pred, else default.
      (let [[pred default xs] args]
        (loop [remaining xs]
          (if (seq remaining)
            (let [item (first remaining)
                  result (apply-callable pred item)]
              (if (= :ok (:status result))
                (if (:value result)
                  {:status :ok :value item}
                  (recur (rest remaining)))
                result))
            {:status :ok :value default})))

      :flip
      ;; flip f a b = f b a
      (apply-callable2 (first args) (nth args 2) (second args))

      :toInt
      (try
        {:status :ok :value (Long/parseLong (str/trim (str (first args))))}
        (catch Throwable _
          (err/failed :builtin :to-int-failed {:builtin name :value (first args)})))

      :optionalString
      (if (first args)
        (let [s-result (force-value (second args))]
          (if (not= :ok (:status s-result))
            s-result
            (or (audit-string-arg name :s (:value s-result))
                {:status :ok :value (str (:value s-result))})))
        {:status :ok :value ""})

      :removePrefix
      (let [[pre s] args
            pre (str pre)
            s (str s)]
        (or (audit-string-arg name :prefix (first args))
            (audit-string-arg name :s (second args))
            {:status :ok
             :value (if (str/starts-with? s pre) (subs s (count pre)) s)}))

      :removeSuffix
      (let [[suf s] args
            suf (str suf)
            s (str s)]
        (or (audit-string-arg name :suffix (first args))
            (audit-string-arg name :s (second args))
            {:status :ok
             :value (if (str/ends-with? s suf)
                      (subs s 0 (- (count s) (count suf)))
                      s)}))

      :concatMapStringsSep
      ;; concatMapStringsSep sep f xs = concatStringsSep sep (map f xs)
      (let [[sep f xs] args]
        (or (audit-string-arg name :separator sep)
            (audit-list-arg name :xs xs)
            (loop [remaining xs
                   out []]
              (if (seq remaining)
                (let [result (apply-callable f (first remaining))]
                  (if (= :ok (:status result))
                    (recur (rest remaining) (conj out (pnix-to-string (:value result))))
                    result))
                {:status :ok :value (str/join (str sep) out)}))))

      :min
      {:status :ok :value (min (first args) (second args))}

      :max
      {:status :ok :value (max (first args) (second args))}

      :imap0
      ;; imap0 f xs = [(f 0 x0) (f 1 x1) ...]
      (let [[f xs] args]
        (if-not (callable-value? f)
          (err/failed :eval
                    :call-target-not-callable
                    {:builtin name
                     :target-kind (:kind f)})
          {:status :ok
           :value (mapv (fn [idx item]
                          (make-thunk [:imap0 idx]
                            (fn []
                              (apply-callable2 f idx item))))
                        (range)
                        xs)}))

      :imap1
      ;; imap1 f xs = [(f 1 x0) (f 2 x1) ...]
      (let [[f xs] args]
        (if-not (callable-value? f)
          (err/failed :eval
                    :call-target-not-callable
                    {:builtin name
                     :target-kind (:kind f)})
          {:status :ok
           :value (mapv (fn [idx item]
                          (make-thunk [:imap1 idx]
                            (fn []
                              (apply-callable2 f idx item))))
                        (iterate inc 1)
                        xs)}))

      :genericClosure
      ;; genericClosure { startSet; operator }: worklist traversal deduped by the
      ;; `key` attribute. Termination is the caller's responsibility (bounded
      ;; keys), as in Nix.
      (let [arg (first args)
            operator-result (force-attr arg "operator")
            start-result (force-attr arg "startSet")]
        (if (not= :ok (:status operator-result))
          operator-result
          (if (not= :ok (:status start-result))
            start-result
            (loop [worklist (vec (:value start-result))
                   seen #{}
                   result []]
              (if (seq worklist)
                (let [item-result (force-value (first worklist))]
                  (if (not= :ok (:status item-result))
                    item-result
                    (let [item (:value item-result)
                          rest-work (subvec worklist 1)
                          key-result (force-attr item "key")]
                      (if (not= :ok (:status key-result))
                        key-result
                        (let [k (:value key-result)]
                          (if (contains? seen k)
                            (recur rest-work seen result)
                            (let [op-result (apply-callable (:value operator-result) item)]
                              (if (= :ok (:status op-result))
                                (recur (into rest-work (:value op-result))
                                       (conj seen k)
                                       (conj result item))
                                op-result))))))))
                {:status :ok :value result})))))

      :concatLists
      (let [xss (first args)]
        (if (not (vector? xss))
          (err/failed :builtin
                    :concat-lists-argument-not-list
                    {:builtin name
                     :value xss})
          (let [xss-result (force-list-values xss)]
            (if (not= :ok (:status xss-result))
              xss-result
              (let [xss (:value xss-result)]
                (if (some #(not (vector? %)) xss)
                  (err/failed :builtin
                            :concat-lists-element-not-list
                            {:builtin name
                             :value xss})
                  {:status :ok
                   :value (vec (apply concat xss))}))))))

      :substring
      ;; Nix returns "" when `start` is at/after the end of the string instead
      ;; of throwing; a negative `length` means "to the end". Clamp both ends so
      ;; `subs` cannot raise StringIndexOutOfBounds on inputs Nix accepts.
      (let [[start length s] args]
        (or (audit-string-arg name :s s)
            (when (neg? start)
              (strict-violation
               :substring-negative-start
               {:construct :builtin
                :builtin name
                :issue :negative-start
                :start start}))
            (let [slen (utf8-length s)
                  len* (if (neg? length) slen length)
                  cut (utf8-substring s start len*)]
              {:status :ok :value cut})))

      :match
      (let [[pattern s] args]
        (or (audit-string-arg name :pattern pattern)
            (audit-string-arg name :s s)
            (let [m (re-matches (nix-regex-pattern pattern) s)]
              {:status :ok
               :value (cond
                        (nil? m) nil
                        (vector? m) (vec (rest m))
                        :else [])})))

      :replaceStrings
      ;; Nix replaceStrings does a single left-to-right pass: at each position it
      ;; tries each `from` string in order and, on the first prefix match, emits
      ;; the paired `to` and advances past the match. The previous sequential
      ;; reduce of str/replace let an earlier pair's output be rewritten by a
      ;; later pair (e.g. ["a" "b"] ["b" "c"] "a" wrongly yielded "c", not "b").
      ;; An empty `from` matches at every position (including the end), so the
      ;; pass also visits index n: replaceStrings [""] ["X"] "ab" -> "XaXbX".
      (let [[from to s] args]
        (or (audit-list-arg name :from from)
            (audit-list-arg name :to to)
            (audit-string-arg name :s s)
            (let [from-result (force-list-values from)
                  to-result (force-list-values to)]
              (if (not= :ok (:status from-result))
                from-result
                (if (not= :ok (:status to-result))
                  to-result
                  (let [from (:value from-result)
                        to (:value to-result)]
                    (or (audit-string-list name :from from)
                        (audit-string-list name :to to)
                        ;; Context-aware: needles match on content; the result
                        ;; carries the original string's context plus the
                        ;; context of every replacement actually USED (unused
                        ;; pairs contribute nothing — exact, not an
                        ;; over-approximation). Context-free inputs produce a
                        ;; plain String as before.
                        (let [pairs (mapv (fn [f t] [(string-content f) t])
                                          from to)
                              content (string-content s)
                              n (count content)
                              used-ctx (volatile! (vec (string-ctx s)))
                              emit! (fn [^StringBuilder out replacement]
                                      (vswap! used-ctx into (string-ctx replacement))
                                      (.append out (str (string-content replacement))))]
                          {:status :ok
                           :value (loop [i 0
                                         out (StringBuilder.)]
                                    (if (<= i n)
                                      (if-let [[needle replacement]
                                               (some (fn [[needle replacement]]
                                                       (let [nl (count needle)]
                                                         (when (and (<= (+ i nl) n)
                                                                    (= needle (subs content i (+ i nl))))
                                                           [needle replacement])))
                                                     pairs)]
                                        (if (zero? (count needle))
                                          ;; Zero-width match: emit the replacement, then carry the
                                          ;; current character (if any) and step forward by one so
                                          ;; the pass terminates.
                                          (do (emit! out replacement)
                                              (if (< i n)
                                                (recur (inc i) (.append out (.charAt content i)))
                                                (ctx-string (.toString out) @used-ctx)))
                                          (recur (+ i (count needle))
                                                 (do (emit! out replacement) out)))
                                        (if (< i n)
                                          (recur (inc i) (.append out (.charAt content i)))
                                          (ctx-string (.toString out) @used-ctx)))
                                      (ctx-string (.toString out) @used-ctx)))}))))))))

      :split
      ;; Nix `split` interleaves the substrings between matches with a list of
      ;; each match's capture groups: split "a" "xayaz" -> ["x" [] "y" [] "z"],
      ;; split "(a)b" "abc" -> ["" ["a"] "c"]. A group that did not participate
      ;; is null. This differs from a plain string split, which drops the
      ;; capture-group slots entirely.
      (let [[pattern s] args]
        (or (audit-string-arg name :pattern pattern)
            (audit-string-arg name :s s)
            (try
              (let [m (re-matcher (nix-regex-pattern pattern) s)]
                (loop [result [] last-end 0]
                  (if (.find m)
                    (let [start (.start m)
                          end (.end m)
                          groups (mapv #(.group m %)
                                       (range 1 (inc (.groupCount m))))]
                      (recur (conj result (subs s last-end start) groups) end))
                    {:status :ok
                     :value (conj result (subs s last-end))})))
              (catch Throwable t
                (err/failed-throwable :builtin :split-builtin-failed t
                                    {:builtin name :pattern pattern})))))

      :listToAttrs
      ;; Nix listToAttrs keeps the FIRST occurrence of a duplicated name, so
      ;; fold left and skip names already seen (a plain `into {}` would let the
      ;; last entry win).
      (loop [remaining (first args)
             out {}]
        (if (seq remaining)
          (let [row-result (force-value (first remaining))]
            (if (not= :ok (:status row-result))
              row-result
              (let [row (:value row-result)
                    key-result (force-attr row "name")]
                (if (not= :ok (:status key-result))
                  key-result
                  (let [k (:value key-result)]
                    (recur (rest remaining)
                           (if (contains? out k)
                             out
                             (assoc out k (get row "value")))))))))
          {:status :ok :value out}))

      :catAttrs
      (let [[attr-name rows] args]
        (loop [remaining rows
               out []]
          (if (seq remaining)
            (let [row-result (force-value (first remaining))]
              (if (not= :ok (:status row-result))
                row-result
                (let [row (:value row-result)]
                  (recur (rest remaining)
                         (cond-> out
                           (contains? row attr-name) (conj (get row attr-name)))))))
            {:status :ok :value out})))

      :mapAttrs
      ;; Nix strictness (oracle-confirmed, D2): an empty attrset short-circuits
      ;; BEFORE the lazily-collected function argument is forced.
      (let [attrs (second args)]
        (if (and (attrset-value? attrs) (empty? attrs))
          {:status :ok :value {}}
          (let [fr (force-value (first args))
                f (:value fr)]
            (cond
              (not= :ok (:status fr))
              fr

              (not (callable-value? f))
              (err/failed :eval
                        :call-target-not-callable
                        {:builtin name
                         :target-kind (:kind f)})

              :else
              {:status :ok
               :value (into {}
                            (map (fn [key]
                                   [key
                                    (make-thunk [:mapAttrs key]
                                      (fn []
                                        (apply-callable2 f key (get attrs key))))]))
                            (sorted-attr-keys attrs))}))))

      :mapAttrs'
      ;; Apply (f name value) over each attribute; f returns a { name; value; }
      ;; pair and the results are collected into a new attrset, with the first
      ;; occurrence of a duplicated name winning (as builtins.listToAttrs).
      (let [[f attrs] args]
        (loop [remaining (sorted-attr-keys attrs)
               out {}]
          (if-let [key (first remaining)]
            (let [result (apply-callable2 f key (get attrs key))]
              (if (not= :ok (:status result))
                result
                (let [pair-result (force-value (:value result))]
                  (if (not= :ok (:status pair-result))
                    pair-result
                    (let [pair (:value pair-result)]
                      (if (and (map? pair)
                               (contains? pair "name")
                               (contains? pair "value"))
                        (let [key-result (force-attr pair "name")]
                          (if (not= :ok (:status key-result))
                            key-result
                            (let [k (:value key-result)]
                              (recur (rest remaining)
                                     (if (contains? out k)
                                       out
                                       (assoc out k (get pair "value")))))))
                        (err/failed :builtin :map-attrs-prime-bad-pair
                                  {:builtin name :pair pair})))))))
            {:status :ok
             :value out})))

      :genAttrs
      ;; genAttrs names f -> { name = f name; ... } for each name.
      (let [[names f] args]
        (let [names-result (force-list-values names)]
          (if (not= :ok (:status names-result))
            names-result
            (loop [remaining (:value names-result)
                   out {}]
              (if-let [n (first remaining)]
                (recur (rest remaining)
                       (assoc out n
                              (make-thunk [:genAttrs n]
                                          (fn [] (apply-callable f n)))))
                {:status :ok
                 :value out})))))

      :nameValuePair
      (let [[n v] args]
        {:status :ok
         :value {"name" n "value" v}})

      :foldlAttrs
      ;; foldlAttrs (acc: name: value: ...) init attrs, folding over sorted keys.
      (let [[f init attrs] args]
        (loop [remaining (sorted-attr-keys attrs)
               acc init]
          (if-let [key (first remaining)]
            (let [r1 (apply-callable f acc)]
              (if (not= :ok (:status r1))
                r1
                (let [r2 (apply-callable (:value r1) key)]
                  (if (not= :ok (:status r2))
                    r2
                    (let [r3 (apply-callable (:value r2) (get attrs key))]
                      (if (= :ok (:status r3))
                        (recur (rest remaining) (:value r3))
                        r3))))))
            {:status :ok
             :value acc})))

      :addErrorContext
      ;; addErrorContext ctx v adds context to errors raised while evaluating v.
      ;; v is already evaluated here (strict), so this is an identity passthrough.
      {:status :ok
       :value (second args)}

      :import
      (let [target (path-or-string (first args))]
        (if *import-resolver*
          (*import-resolver* (import-resolver-context nil) target nil)
          (err/failed :eval
                    :import-evaluation-not-wired
                    {:target target})))

      :scopedImport
      ;; Nix `scopedImport scope path`: scope is the FIRST arg, path the SECOND
      ;; (verified against nix-instantiate 2.34.7). The scope attrs are added on
      ;; top of the global scope for the imported file (base globals stay
      ;; available; scope shadows on name conflict; scope does NOT propagate
      ;; through nested plain `import`). The resolver injects the scope.
      (let [scope (first args)
            target (path-or-string (second args))]
        (cond
          (not (attrset-value? scope))
          (err/failed :eval
                    :scoped-import-scope-not-attrset
                    {:builtin :scopedImport :scope scope})

          *import-resolver*
          (*import-resolver* (import-resolver-context nil) target scope)

          :else
          (err/failed :eval
                    :import-evaluation-not-wired
                    {:target target
                     :builtin :scopedImport})))

      :unsafeGetAttrPos
      ;; Return the parser span for an attr when it is still available on the
      ;; attrset metadata. Nix uses file/line/column; this lane currently exposes
      ;; byte/char offsets as a stable span receipt and returns null when absent.
      (let [[attr attrs] args]
        (if-not (attrset-value? attrs)
          (err/failed :builtin
                    :unsafe-get-attr-pos-target-not-attrset
                    {:builtin name :target attrs})
          {:status :ok
           :value (get (attr-positions attrs) attr)}))

      :filterAttrs
      (let [[pred attrs] args]
        (loop [remaining (sorted-attr-keys attrs)
               out {}]
          (if-let [key (first remaining)]
            (let [result (apply-callable2 pred key (get attrs key))]
              (if (= :ok (:status result))
                (recur (rest remaining)
                       (cond-> out
                         (:value result) (assoc key (get attrs key))))
                result))
            {:status :ok
             :value out})))

      :groupBy
      (let [[f xs] args]
        (loop [remaining xs
               out {}]
          (if (seq remaining)
            (let [item (first remaining)
                  result (apply-callable f item)]
              (if (= :ok (:status result))
                (recur (rest remaining)
                       (update out (:value result) (fnil conj []) item))
                result))
            {:status :ok
             :value out})))

      :partition
      (let [[pred xs] args]
        (if-not (vector? xs)
          (err/failed :builtin :partition-arg-not-list {:builtin name :arg xs})
          (loop [remaining xs
               out {"right" []
                    "wrong" []}]
          (if (seq remaining)
            (let [item-result (force-value (first remaining))]
              (if (not= :ok (:status item-result))
                item-result
                (let [item (:value item-result)
                      result (apply-callable pred item)]
                  (if (= :ok (:status result))
                    (recur (rest remaining)
                           (update out
                                   (if (:value result) "right" "wrong")
                                   conj
                                   item))
                    result))))
            {:status :ok
             :value out}))))

      :genList
      (let [[f n] args]
        {:status :ok
         :value (mapv (fn [i]
                        (make-thunk [:genList i]
                          (fn []
                            (apply-callable f i))))
                      (range n))})

      :zipAttrsWith
      (let [[f rows] args
            rows-result (force-list-values rows)]
        (cond
          (not (callable-value? f))
          (err/failed :eval
                    :call-target-not-callable
                    {:builtin name
                     :target-kind (:kind f)})

          (not= :ok (:status rows-result))
          rows-result

          :else
          (let [rows (:value rows-result)
                attr-names (->> rows
                                (mapcat keys)
                                distinct
                                sort
                                vec)]
            (loop [remaining attr-names
                   out {}]
              (if-let [attr-name (first remaining)]
                (let [values (reduce (fn [acc row]
                                       (cond-> acc
                                         (contains? row attr-name)
                                         (conj (get row attr-name))))
                                     []
                                     rows)
                      value (make-thunk [:zipAttrsWith attr-name]
                              (fn []
                                (apply-callable2 f attr-name values)))]
                  (recur (rest remaining)
                         (assoc out attr-name value)))
                {:status :ok
                 :value out})))))

      :map
      ;; The list argument must be a list; iterating a string would leak raw
      ;; host characters as pnix values. Nix strictness (oracle-confirmed, D2):
      ;; an empty list short-circuits BEFORE the lazily-collected function
      ;; argument is forced; a non-list forces the function first.
      (let [xs (second args)]
        (if (and (vector? xs) (empty? xs))
          {:status :ok :value []}
          (let [fr (force-value (first args))
                f (:value fr)]
            (cond
              (not= :ok (:status fr))
              fr

              (not (vector? xs))
              (err/failed :builtin :map-arg-not-list {:builtin name :arg xs})

              (not (callable-value? f))
              (err/failed :eval
                        :call-target-not-callable
                        {:builtin name
                         :target-kind (:kind f)})

              :else
              {:status :ok
               :value (mapv (fn [idx item]
                              (make-thunk [:map idx]
                                (fn []
                                  (apply-callable f item))))
                            (range)
                            xs)}))))

      :concatMap
      (if-not (vector? (second args))
        (err/failed :builtin :concat-map-arg-not-list {:builtin name :arg (second args)})
        (loop [remaining (second args)
               values []]
        (if (seq remaining)
          (let [item (first remaining)
                result (apply-callable (first args) item)]
            (cond
              (not= :ok (:status result))
              result

              (not (vector? (:value result)))
              (err/failed :builtin
                        :concat-map-result-not-list
                        {:builtin name
                         :item item
                         :value (:value result)})

              :else
              (recur (rest remaining)
                     (into values (:value result)))))
          {:status :ok
           :value values})))

      :filter
      ;; Nix strictness (oracle-confirmed, D2): an empty list short-circuits
      ;; BEFORE the lazily-collected predicate is forced.
      (let [xs (second args)]
        (if (and (vector? xs) (empty? xs))
          {:status :ok :value []}
          (let [fr (force-value (first args))
                f (:value fr)]
            (cond
              (not= :ok (:status fr))
              fr

              (not (vector? xs))
              (err/failed :builtin :filter-arg-not-list {:builtin name :arg xs})

              :else
              (loop [remaining xs
                     values []]
                (if (seq remaining)
                  (let [item (first remaining)
                        result (apply-callable f item)]
                    (if (= :ok (:status result))
                      (recur (rest remaining)
                             (cond-> values
                               (:value result) (conj item)))
                      result))
                  {:status :ok
                   :value values}))))))

      :all
      (if-not (vector? (second args))
        (err/failed :builtin :all-arg-not-list {:builtin name :arg (second args)})
        (loop [remaining (second args)]
        (if (seq remaining)
          (let [item (first remaining)
                result (apply-callable (first args) item)]
            (if (not= :ok (:status result))
              result
              (if (:value result)
                (recur (rest remaining))
                {:status :ok
                 :value false})))
          {:status :ok
           :value true})))

      :any
      (if-not (vector? (second args))
        (err/failed :builtin :any-arg-not-list {:builtin name :arg (second args)})
        (loop [remaining (second args)]
        (if (seq remaining)
          (let [item (first remaining)
                result (apply-callable (first args) item)]
            (if (not= :ok (:status result))
              result
              (if (:value result)
                {:status :ok
                 :value true}
                (recur (rest remaining)))))
          {:status :ok
           :value false})))

      :foldl'
      ;; Nix strictness (oracle-confirmed, D2): the initial accumulator is
      ;; passed lazily (an operator that ignores it never evaluates it), but
      ;; each application result and the final result ARE forced — foldl' is
      ;; the strict fold.
      (if-not (vector? (nth args 2))
        (err/failed :builtin :foldl-arg-not-list {:builtin name :arg (nth args 2)})
        (loop [remaining (nth args 2)
             acc (second args)]
        (if (seq remaining)
          (let [item (first remaining)
                first-result (apply-callable (first args) acc)]
            (if (not= :ok (:status first-result))
              first-result
              (let [second-result (apply-callable (:value first-result) item)]
                (if (= :ok (:status second-result))
                  (let [forced (force-value (:value second-result))]
                    (if (= :ok (:status forced))
                      (recur (rest remaining) (:value forced))
                      forced))
                  second-result))))
          (force-value acc))))

      :sort
      ;; Nix strictness (oracle-confirmed, D2): an empty list short-circuits
      ;; BEFORE the lazily-collected comparator is forced; a non-empty sort
      ;; forces each ELEMENT to WHNF before comparing (a poisoned element
      ;; holds, a poisoned nested value does not).
      (let [xs (second args)]
        (if (and (vector? xs) (empty? xs))
          {:status :ok :value []}
          (let [fr (force-value (first args))
                f (:value fr)]
            (cond
              (not= :ok (:status fr))
              fr

              (not (vector? xs))
              (err/failed :builtin :sort-arg-not-list {:builtin name :arg xs})

              :else
              (let [forced-elems
                    (loop [remaining xs
                           out []]
                      (if (seq remaining)
                        (let [r (force-value (first remaining))]
                          (if (= :ok (:status r))
                            (recur (rest remaining) (conj out (:value r)))
                            r))
                        {:status :ok :value out}))]
                (if (not= :ok (:status forced-elems))
                  forced-elems
                  {:status :ok
                   :value (vec
                           (sort (fn [a b]
                                   (let [first-result (apply-callable f a)]
                                     (when (not= :ok (:status first-result))
                                       (throw (ex-info "sort comparator first call failed"
                                                       {:result first-result})))
                                     (let [second-result (apply-callable (:value first-result) b)]
                                       (when (not= :ok (:status second-result))
                                         (throw (ex-info "sort comparator second call failed"
                                                         {:result second-result})))
                                       (true? (:value second-result)))))
                                 (:value forced-elems)))}))))))

      (throw (ex-info "unsupported builtin"
                      {:builtin name})))))
    (catch Throwable t
      ;; A fuel-exhausted signal must reach eval-ast-with-fuel, not be
      ;; swallowed as a builtin failure.
      (when (:pnix-fuel-exhausted (ex-data t)) (throw t))
      ;; The pnix-to-string / nix-coerce-to-string context backstops throw with
      ;; :string-context-frontier ex-data; surface that reason instead of the
      ;; generic dispatch failure so the frontier stays legible.
      (if (= :string-context-frontier (:reason (ex-data t)))
        (err/failed :builtin
                  :string-context-frontier
                  {:builtin name
                   :policy :deny-by-default-until-context-aware})
        (err/failed-throwable :builtin
                            :builtin-dispatch-failed
                            t
                            {:builtin name})))))

(defn apply-callable
  [f arg]
  (cond
    (= :builtin (:kind f))
    ;; Builtins are strict in their arguments except for the Nix-lazy positions
    ;; listed in lazy-builtin-arg? (elem needle, map/filter/mapAttrs/sort
    ;; function args, foldl' initial accumulator), which finish-builtin forces
    ;; only when Nix would (oracle-confirmed strictness matrix, D2).
    (let [forced (if (lazy-builtin-arg? f)
                   {:status :ok :value arg}
                   (force-value arg))]
      (if (not= :ok (:status forced))
        forced
        (let [f' (update f :args conj (:value forced))]
          (if (= (:arity f') (count (:args f')))
            (finish-builtin f')
            {:status :ok
             :value f'}))))

    (= :closure (:kind f))
    (let [base-env (or (some-> f :env-ref deref)
                       (:env f))]
      (if-let [pattern (:param-pattern f)]
        ;; pattern lambdas need the attrset structure, so force the arg to WHNF.
        ;;
        ;; D19 (oracle-gated, nix-instantiate 2.34.7): real Nix pattern
        ;; semantics — (1) the argument must be an attrset; (2) every
        ;; REQUIRED formal is checked at application time, in pattern order,
        ;; BEFORE the extra-key check ('({ a, b }: a) { a=1; c=1; }' reports
        ;; b, not c); (3) without `...` an extra argument key is an error;
        ;; (4) defaults are LAZY thunks in a KNOT-TIED recursive scope — the
        ;; whole formal set plus @as is visible ('({ a ? b, b ? 2 }: a) { }'
        ;; is 2, an unused default is never evaluated, a default cycle is
        ;; infinite recursion); (5) @as binds the ACTUAL argument only,
        ;; defaults are not in it. The pre-D19 loop was eager + sequential +
        ;; extra-key lenient (all three oracle-refuted).
        (let [arg-forced (force-value arg)]
          (if (not= :ok (:status arg-forced))
            arg-forced
            (let [arg-val (:value arg-forced)
                  as-name (:as pattern)
                  params (:params pattern)]
              (cond
                (not (attrset-value? arg-val))
                (err/failed :eval
                          :lambda-pattern-arg-not-attrset
                          {:value-type (strict-type arg-val)})

                :else
                (if-let [missing (some (fn [{:keys [name default]}]
                                         (when (and (not default)
                                                    (not (contains? arg-val name)))
                                           name))
                                       params)]
                  (err/failed :eval
                            :missing-lambda-pattern-arg
                            {:param missing})
                  (let [formal-names (into #{} (map :name) params)
                        extra (when-not (:ellipsis? pattern)
                                (first (sort (remove formal-names
                                                     (keys arg-val)))))]
                    (if extra
                      (err/failed :eval
                                :unexpected-lambda-pattern-arg
                                {:arg extra})
                      (let [env-ref (atom nil)
                            final-env
                            (reduce (fn [acc {:keys [name default]}]
                                      (assoc acc name
                                             (if (contains? arg-val name)
                                               ;; raw slot: stays lazy
                                               (get arg-val name)
                                               (make-thunk name
                                                           (fn [] (eval-ast* @env-ref
                                                                             default))))))
                                    (cond-> base-env
                                      as-name (assoc as-name arg-val))
                                    params)]
                        (reset! env-ref final-env)
                        (eval-ast* final-env (:body f))))))))))
        ;; simple param: bind the (possibly thunk) arg lazily; the body forces it
        ;; on reference, so an unused argument is never evaluated.
        (eval-ast* (assoc base-env (:param f) arg)
                   (:body f))))

    (= :host-fn (:kind f))
    (apply-host-fn f arg)

    (= :lazy-host-fn (:kind f))
    (try
      (let [result ((:impl f) arg)]
        (if (and (map? result) (contains? result :status))
          result
          {:status :ok :value result}))
      (catch clojure.lang.ExceptionInfo e
        (if (:pnix-fuel-exhausted (ex-data e))
          (throw e)
          (err/failed :eval
                    :lazy-host-fn-apply-failed
                    {:label (:label f)
                     :reason (:reason (ex-data e))})))
      (catch Throwable t
        (err/failed-throwable :eval
                            :lazy-host-fn-apply-failed
                            t
                            {:label (:label f)})))

    :else
    (err/failed :eval
              :call-target-not-callable
              {:target-kind (:kind f)})))

(defn- apply-host-fn
  [f arg]
  (let [forced (force-value arg)]
    (if (not= :ok (:status forced))
      forced
      (let [f' (update f :args conj (:value forced))]
        (if (= (:arity f') (count (:args f')))
          (try
            (let [result (apply (:impl f') (:args f'))]
              (if (and (map? result)
                       (contains? result :status))
                result
                {:status :ok :value result}))
            (catch Throwable t
              (err/failed-throwable :eval
                                  :host-fn-apply-failed
                                  t
                                  {:fn (:impl f')
                                   :args (:args f')})))
          {:status :ok
           :value f'})))))

(defn- eval-string-template
  [env parts]
  (loop [remaining parts
         chunks []]
    (if-let [part (first remaining)]
      (case (:kind part)
        :text
        (recur (rest remaining) (conj chunks (:value part)))

        :expr
        (let [result (force-result-value (eval-ast* env (:expr part)))]
          (if (= :ok (:status result))
            (let [string-result (interpolation-to-string (:value result))]
              (if (= :ok (:status string-result))
                (recur (rest remaining) (conj chunks (:value string-result)))
                string-result))
            result)))
      ;; Chunks are plain strings or ctx-strings; join the contents and union
      ;; the contexts. ctx-string returns a plain String when no chunk carried
      ;; context, so context-free templates are byte-identical to before.
      {:status :ok
       :value (ctx-string (apply str (map string-content chunks))
                          (into [] (mapcat string-ctx) chunks))})))

(defn- eval-call
  ;; NB: bind the :fn AST as `fn-form`, not `fn` — a local named `fn` would
  ;; shadow clojure.core/fn and turn the thunk's `(fn [] ...)` into a map lookup.
  [env {:keys [arg] fn-form :fn}]
  (let [fn-result (force-result-value (eval-ast* env fn-form))]
    (if (not= :ok (:status fn-result))
      fn-result
      (if (and (= :builtin (:kind (:value fn-result)))
               (= :tryEval (:name (:value fn-result))))
        (let [arg-result (eval-ast* env arg)]
          (cond
            (= :ok (:status arg-result))
            {:status :ok
             :value {"success" true
                     "value" (:value arg-result)}}

            ;; Nix tryEval only catches `throw` and `assert`; abort, type
            ;; errors, division by zero, out-of-bounds, etc. are not caught and
            ;; propagate as held.
            (contains? #{:throw-builtin-called :assertion-failed}
                       (:reason arg-result))
            {:status :ok
             :value {"success" false
                     "value" false}}

            :else
            arg-result))
        ;; Call-by-need: pass the argument as a thunk. apply-callable forces it
        ;; for builtins and pattern lambdas; a simple-param closure binds it
        ;; lazily, so an unused argument is never evaluated.
        (binding [*import-origin* (or (get env import-context-key)
                                     *import-origin*)]
          (apply-callable (:value fn-result)
                          (make-thunk :arg (fn [] (eval-ast* env arg)))))))))

(defn eval-ast*
  "Evaluate the direct pnix semantic lane for the current small expression core."
  [env ast]
  (when-let [f *fuel*]
    (when (neg? (long (vswap! f unchecked-dec)))
      (throw (ex-info "pnix fuel exhausted" {:pnix-fuel-exhausted true}))))
  (cover! :op (:op ast))
  (case (:op ast)
    (:int :float :bool :null :string)
    {:status :ok
     :value (:value ast)}

    :path
    {:status :ok
     :value (path-value (:value ast))}

    :string-template
    (eval-string-template env (:parts ast))

    :var
    (let [name (:name ast)]
      (cond
        (= "__curPos" name)
        {:status :ok
         :value (source-position (:span ast))}

        ;; Lexical bindings (let/lambda) win. Values bound by `let` are thunks;
        ;; force on reference (memoized). Everything else passes through.
        (contains? env name)
        (force-value (get env name))

        ;; Otherwise fall back to `with` scopes, innermost first.
        :else
        (if-let [scope (some (fn [attrs] (when (contains? attrs name) attrs))
                             (::with-scopes env))]
          (force-value (get scope name))
          (err/failed :eval
                    :unbound-var
                    {:name name}))))

    :list
    (eval-items env (:items ast))

    :attrset
    (eval-attrs env (:attrs ast) (:recursive ast))

    :let
    (eval-let env (:bindings ast) (:body ast))

    :if
    (let [condition-result (force-result-value (eval-ast* env (:condition ast)))]
      (if (not= :ok (:status condition-result))
        condition-result
        (or (if-condition-violation (:value condition-result) (:span ast))
            (if (:value condition-result)
              (do
                (cover! :branch :if/then)
                (eval-ast* env (:then ast)))
              (do
                (cover! :branch :if/else)
                (eval-ast* env (:else ast)))))))

    :assert
    (let [condition-result (force-result-value (eval-ast* env (:condition ast)))]
      (if (not= :ok (:status condition-result))
        condition-result
        (do
          (or (when-not (boolean? (:value condition-result))
                (strict-violation
                 :non-bool-assert-condition
                 {:construct :assert
                  :issue :non-bool-condition
                  :value-type (strict-type (:value condition-result))
                  :span (:span ast)}))
              (if (:value condition-result)
                (do
                  (cover! :branch :assert/pass)
                  (eval-ast* env (:body ast)))
                (do
                  (cover! :branch :assert/fail)
                  (err/failed :eval
                            :assertion-failed
                            {:span (:span ast)})))))))

    :with
    ;; `with attrs; body`: attrs become a fallback scope behind the lexical env
    ;; (lexical bindings still win; an inner `with` shadows an outer one). The
    ;; scope list rides in the env so closures capture it.
    (let [env-result (force-result-value (eval-ast* env (:env-expr ast)))]
      (if (not= :ok (:status env-result))
        env-result
        (if (attrset-value? (:value env-result))
          (eval-ast* (assoc env ::with-scopes
                            (cons (:value env-result)
                                  (::with-scopes env)))
                     (:body ast))
          (err/failed :eval
                    :with-not-attrset
                    {:value (:value env-result)}))))

    :lambda
    {:status :ok
     :value {:kind :closure
             :param (:param ast)
             :param-pattern (:param-pattern ast)
             :body (:body ast)
             :env env}}

    :select
    (eval-select env ast)

    :has-attr
    (eval-has-attr env ast)

    :not
    (let [result (force-result-value (eval-ast* env (:expr ast)))]
      (if (= :ok (:status result))
        (not-value-result (:value result) (:span ast))
        result))

    :neg
    (let [result (force-result-value (eval-ast* env (:expr ast)))]
      (if (= :ok (:status result))
        (neg-value-result (:value result))
        result))

    :import
    (if *import-resolver*
      (*import-resolver* (import-resolver-context env)
                         (:target ast)
                         nil)
      (err/failed :eval
                :import-evaluation-not-wired
                {:target (:target ast)}))

    :call
    (eval-call env ast)

    :binary
    (eval-binary env ast)

    (err/failed :eval
              :unsupported-eval-op
              {:op (:op ast)})))

(defn eval-ast
  ([ast]
   (eval-ast ast default-env))
  ([ast base-env]
   (let [result (eval-ast* base-env ast)]
     (if (= :ok (:status result))
       (force-deep (:value result))
       result))))

(defn eval-ast-whnf
  "Like `eval-ast` but WITHOUT the deep realization at the API boundary: the
  value is returned in weak-head normal form, thunks intact. This is what
  `import` must use — real Nix's `import` yields a LAZY value, so deep-forcing
  an imported module diverges on the mutually-recursive schema modules
  (`rec { list = { elem = string; }; ... }`) that Nix imports happily. The
  outer evaluation forces whatever it actually needs, and the top-level
  `eval-ast` still deep-realizes the FINAL result."
  ([ast]
   (eval-ast-whnf ast default-env))
  ([ast base-env]
   (eval-ast* base-env ast)))

(defn eval-ast-with-fuel
  "Evaluate `ast` under a step budget. Every eval-ast* entry costs one unit;
  when the budget runs out the evaluation is suspended with a stable
  resource-budget reason (a
  structured verdict, not a crash). Used by specialize's bounded folding and
  the safe-eval tier."
  [ast fuel]
  ;; Run on a fixed 8MB stack (like core's evaluator lane) so deep recursion
  ;; burns FUEL before it can blow the stack; any Throwable that still escapes
  ;; (StackOverflowError included) becomes a structured failure, never a crash.
  (let [result (promise)
        ;; bound-fn so caller bindings (*pure-eval*, *strict*, etc.) reach the
        ;; fuel thread — plain fn would rebind only *fuel* and drop purity.
        run (bound-fn []
              (deliver
               result
               (binding [*fuel* (volatile! (long fuel))]
                 (try
                   (eval-ast ast)
                   (catch clojure.lang.ExceptionInfo e
                     (if (:pnix-fuel-exhausted (ex-data e))
                       {:status :suspended
                        :reason :resource-budget-exhausted
                        :resource-reason :transition-budget-exhausted
                        :fuel fuel}
                       {:status :failed
                        :reason :fuel-eval-failed
                        :error {:phase :eval
                                :class :fuel-eval-failed}}))
                   (catch Throwable _
                     {:status :failed
                      :reason :fuel-eval-failed
                      :error {:phase :eval
                              :class :fuel-eval-failed}})))))
        thread (Thread. nil ^Runnable run "pnix-fuel-eval"
                        (long (* 8 1024 1024)))]
    (.start thread)
    @result))

(defn force-whnf
  [value]
  (force-value value))

(defn force-normal
  [value]
  (force-deep value))

;; ── value bridge API ─────────────────────────────────────────────────
;; Seams for sibling lanes (lowering) to DELEGATE to this evaluator's
;; builtins without giving up call-by-need: their lazy slots enter as
;; thunks, their functions enter as :lazy-host-fn callables that receive
;; arguments RAW (a :host-fn forces to WHNF first, which would make every
;; delegated function strict).

(defn value-thunk?
  [v]
  (thunk? v))

(defn make-value-thunk
  "Wrap `compute` (returning an eval result map) as this evaluator's
  call-by-need thunk."
  [label compute]
  (make-thunk label compute))

(defn lazy-host-fn
  "A callable whose impl receives its argument RAW (possibly a thunk)."
  [label impl]
  {:kind :lazy-host-fn :label label :impl impl})
