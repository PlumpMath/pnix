(ns pnix.clj-meta.frontend-selfhost
  "Tiny frontend self-host witness.

  This lane intentionally avoids clojure.tools.analyzer.jvm and the Clojure
  reader for the accepted rows.  It parses a tiny source subset, builds its own
  AST, emits JVM bytecode directly with clojure.asm, and checks the result.
  It is a frontier witness, not a replacement for the production frontend."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp])
  (:import [clojure.asm ClassWriter Label Opcodes Type]
           [clojure.asm.commons GeneratorAdapter Method]
           [clojure.lang AFunction BigInt DynamicClassLoader IFn Keyword Numbers Ratio Reflector RestFn RT Symbol Util Var]
           [java.math BigDecimal BigInteger]
           [java.security MessageDigest]
           [java.util.regex Pattern]))

(def receipt-path "clj-meta/proof/frontend-selfhost.receipt.edn")

(def ^:private obj-type (Type/getType Object))
(def ^:private afn-type (Type/getType AFunction))
(def ^:private restfn-type (Type/getType RestFn))
(def ^:private numbers-type (Type/getType Numbers))
(def ^:private rt-type (Type/getType RT))
(def ^:private util-type (Type/getType Util))
(def ^:private keyword-type (Type/getType Keyword))
(def ^:private symbol-type (Type/getType Symbol))
(def ^:private throwable-type (Type/getType Throwable))
(def ^:private string-type (Type/getType String))
(def ^:private java-biginteger-type (Type/getType BigInteger))
(def ^:private clj-bigint-type (Type/getType BigInt))
(def ^:private bigdec-type (Type/getType BigDecimal))
(def ^:private ratio-type (Type/getType Ratio))
(def ^:private pattern-type (Type/getType Pattern))
(def ^:private object-array-class (Class/forName "[Ljava.lang.Object;"))
(def ^:private init-method
  (Method. "<init>" Type/VOID_TYPE (into-array Type [])))
(def ^:private string-arg-ctor-method
  (Method. "<init>" Type/VOID_TYPE (into-array Type [string-type])))
(def ^:private ratio-ctor-method
  (Method. "<init>" Type/VOID_TYPE (into-array Type [java-biginteger-type java-biginteger-type])))
(def ^:private class-counter (atom -1))

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- reflect-asm-method
  ^Method [^Class cls ^String mname param-classes]
  (Method/getMethod (.getMethod cls mname (into-array Class param-classes))))

(def ^:private pattern-compile-method
  (reflect-asm-method Pattern "compile" [String]))
(def ^:private numbers-add-method
  (reflect-asm-method Numbers "add" [Object Object]))
(def ^:private numbers-minus-method
  (reflect-asm-method Numbers "minus" [Object Object]))
(def ^:private numbers-multiply-method
  (reflect-asm-method Numbers "multiply" [Object Object]))
(def ^:private numbers-lt-method
  (reflect-asm-method Numbers "lt" [Object Object]))
(def ^:private util-equiv-method
  (reflect-asm-method Util "equiv" [Object Object]))
(def ^:private rt-booleancast-method
  (reflect-asm-method RT "booleanCast" [Object]))
(def ^:private rt-vector-method
  (reflect-asm-method RT "vector" [object-array-class]))
(def ^:private rt-map-method
  (reflect-asm-method RT "map" [object-array-class]))
(def ^:private rt-set-method
  (reflect-asm-method RT "set" [object-array-class]))
(def ^:private keyword-intern-method
  (reflect-asm-method Keyword "intern" [String]))
(def ^:private symbol-intern-method
  (reflect-asm-method Symbol "intern" [String]))
(def ^:private numbers-gt-method
  (reflect-asm-method Numbers "gt" [Object Object]))
(def ^:private numbers-gte-method
  (reflect-asm-method Numbers "gte" [Object Object]))
(def ^:private numbers-lte-method
  (reflect-asm-method Numbers "lte" [Object Object]))
(def ^:private numbers-quotient-method
  (reflect-asm-method Numbers "quotient" [Object Object]))
(def ^:private numbers-remainder-method
  (reflect-asm-method Numbers "remainder" [Object Object]))
(def ^:private numbers-minus-unary-method
  (reflect-asm-method Numbers "minus" [Object]))
(def ^:private numbers-inc-method
  (reflect-asm-method Numbers "inc" [Object]))
(def ^:private numbers-dec-method
  (reflect-asm-method Numbers "dec" [Object]))
(def ^:private numbers-iszero-method
  (reflect-asm-method Numbers "isZero" [Object]))
(def ^:private numbers-ispos-method
  (reflect-asm-method Numbers "isPos" [Object]))
(def ^:private numbers-isneg-method
  (reflect-asm-method Numbers "isNeg" [Object]))
(def ^:private rt-first-method
  (reflect-asm-method RT "first" [Object]))
(def ^:private rt-next-method
  (reflect-asm-method RT "next" [Object]))
(def ^:private rt-get-method
  (reflect-asm-method RT "get" [Object Object]))
(def ^:private rt-get3-method
  (reflect-asm-method RT "get" [Object Object Object]))
(def ^:private rt-count-method
  (reflect-asm-method RT "count" [Object]))
(def ^:private reflector-type (Type/getType Reflector))
(def ^:private reflector-invoke-instance-method-method
  (reflect-asm-method Reflector "invokeInstanceMethod" [Object String object-array-class]))
(def ^:private reflector-invoke-static-method-method
  (reflect-asm-method Reflector "invokeStaticMethod" [Class String object-array-class]))
(def ^:private var-type (Type/getType Var))
(def ^:private ifn-type (Type/getType IFn))
(def ^:private rt-var-method
  (reflect-asm-method RT "var" [String String]))
(def ^:private var-getrawroot-method
  (reflect-asm-method Var "getRawRoot" []))
(def ^:private var-get-method
  (reflect-asm-method Var "get" []))
(def ^:private reflector-invoke-noarg-instance-member-method
  (reflect-asm-method Reflector "invokeNoArgInstanceMember" [Object String Boolean/TYPE]))
(def ^:private reflector-set-instance-field-method
  (reflect-asm-method Reflector "setInstanceField" [Object String Object]))
(def ^:private rt-classfor-name-method
  (reflect-asm-method RT "classForName" [String]))
(def ^:private reflector-invoke-constructor-method
  (reflect-asm-method Reflector "invokeConstructor" [Class object-array-class]))
(def ^:private bigint-frombiginteger-method
  (reflect-asm-method BigInt "fromBigInteger" [BigInteger]))

(defn- next-class-name
  []
  (str "pnix.clj_meta.frontend_selfhost.Fn__" (swap! class-counter inc)))

(defn reset-compiler-state!
  "Reset deterministic class naming for receipt-generating callers."
  []
  (reset! class-counter -1))

(defn- tokenize
  [source]
  (->> (re-seq #"\s*(#\{|#\"[^\"]*\"|\"[^\"]*\"|[\(\)\[\]\{\}]|[^\s\(\)\[\]\{\}]+)" source)
       (mapv second)))

(declare parse-one)

(defn- parse-list*
  [tokens close-token]
  (loop [tokens tokens
         acc []]
    (cond
      (empty? tokens)
      (throw (ex-info "tiny reader: missing closing delimiter"
                      {:close-token close-token}))

      (= close-token (first tokens))
      [acc (rest tokens)]

      :else
      (let [[x tokens'] (parse-one tokens)]
        (recur tokens' (conj acc x))))))

(defn- parse-atom
  [token]
  (case token
    "nil" nil
    "true" true
    "false" false
    (cond
      (and (.startsWith ^String token "\"")
           (.endsWith ^String token "\""))
      (subs token 1 (dec (count token)))

      (and (.startsWith ^String token "#\"")
           (.endsWith ^String token "\""))
      (Pattern/compile (subs token 2 (dec (count token))))

      (and (.startsWith ^String token ":")
           (< 1 (count token)))
      (keyword (subs token 1))

      ;; `N`-suffixed literals are `clojure.lang.BigInt`, NOT a raw
      ;; `java.math.BigInteger` -- confirmed live: `(class 5N)` is
      ;; `clojure.lang.BigInt`, and a first draft that built a plain
      ;; `BigInteger` here matched by value (Clojure's `=` compares across
      ;; numeric types) but NOT by class, a real gap caught by explicitly
      ;; checking classes, not just values, before trusting this fixture.
      (re-matches #"-?\d+N" token)
      (BigInt/fromBigInteger (BigInteger. ^String (subs token 0 (dec (count token)))))

      (re-matches #"-?\d+\.\d+M" token)
      (BigDecimal. ^String (subs token 0 (dec (count token))))

      ;; Ratio literals reduce (or collapse to `Long` when evenly divisible)
      ;; at *parse* time via `Numbers/divide` -- the exact mechanism real
      ;; Clojure's reader itself uses, confirmed live: `(Numbers/divide 2 4)`
      ;; => `1/2` (auto-reduced `Ratio`), `(Numbers/divide 4 2)` => `2`
      ;; (`Long`, not `Ratio`). Deliberately NOT via `RT/readString` (real
      ;; host's actual bytecode mechanism), matching the same independence
      ;; principle already applied to BigInt/BigDecimal literals above.
      (re-matches #"-?\d+/\d+" token)
      (let [[_ n d] (re-matches #"(-?\d+)/(\d+)" token)]
        (Numbers/divide (Long/parseLong ^String n) (Long/parseLong ^String d)))

      (re-matches #"-?\d+" token)
      (Long/parseLong token)

      :else
      (symbol token))))

(defn- parse-one
  [tokens]
  (let [token (first tokens)]
    (case token
      "(" (let [[xs tokens'] (parse-list* (rest tokens) ")")]
            [(apply list xs) tokens'])
      "[" (let [[xs tokens'] (parse-list* (rest tokens) "]")]
            [(vec xs) tokens'])
      "{" (let [[xs tokens'] (parse-list* (rest tokens) "}")]
            (when-not (even? (count xs))
              (throw (ex-info "tiny reader: map literal arity"
                              {:items xs})))
            [(apply array-map xs) tokens'])
      "#{" (let [[xs tokens'] (parse-list* (rest tokens) "}")]
             [(set xs) tokens'])
      (")" "]" "}") (throw (ex-info "tiny reader: unexpected closing delimiter"
                                    {:token token}))
      [(parse-atom token) (rest tokens)])))

(defn- tiny-read
  [source]
  (let [[form rest-tokens] (parse-one (tokenize source))]
    (when (seq rest-tokens)
      (throw (ex-info "tiny reader: trailing tokens" {:tokens rest-tokens})))
    form))

(def ^:private tiny-macro-rules
  [:when :and :or :not :nil? :thread-first :thread-last :cond :when-not :if-not
   :if-let :when-let :as-> :cond-> :cond->> :some-> :some->> :case])

(defn- macro-local
  [prefix counter]
  (symbol (str prefix "__" (swap! counter inc))))

(declare expand-form)

(defn- expand-and
  [counter args]
  (case (count args)
    0 true
    1 (first args)
    (let [local (macro-local "and" counter)]
      (list 'let [local (first args)]
            (list 'if local
                  (expand-and counter (rest args))
                  local)))))

(defn- expand-or
  [counter args]
  (case (count args)
    0 nil
    1 (first args)
    (let [local (macro-local "or" counter)]
      (list 'let [local (first args)]
            (list 'if local
                  local
                  (expand-or counter (rest args)))))))

(defn- thread-first-step
  [x form]
  (if (seq? form)
    (apply list (first form) x (rest form))
    (list form x)))

(defn- thread-last-step
  [x form]
  (if (seq? form)
    (apply list (concat form [x]))
    (list form x)))

(defn- expand-cond
  [args]
  (cond
    (empty? args) nil
    (= 1 (count args))
    (throw (ex-info "tiny macroexpander: cond needs even forms" {:args args}))
    :else (list 'if (first args) (second args) (expand-cond (drop 2 args)))))

;; `(case x v1 r1 v2 r2 ... default)` -> a `let`-bound test value plus a
;; nested `if`/`=` chain. Real `case` uses hash-based O(1) dispatch (a JVM
;; lookupswitch/tableswitch), not a linear `=` chain -- this backend's bar is
;; behavior equivalence on the covered fixtures, not bytecode-shape
;; equivalence, and a sequential `=` chain gives the identical result for
;; every fixture here (int/keyword/string literal tests, verified against
;; the real host before being added). Case-test values are spliced in
;; unquoted: this tiny language's `analyze-expr` already treats a bare
;; integer/keyword/string/nil/boolean atom as a self-evaluating constant (no
;; `quote` needed), matching how the tiny reader itself hands them back.
;;
;; No-default no-match now throws `IllegalArgumentException`, matching real
;; `case` (confirmed live: real host's message is dynamic, "No matching
;; clause: <value>", built via `str`; this tiny language has no string
;; concatenation, so the thrown message here is a fixed
;; "No matching clause" instead -- an honest, narrower approximation of the
;; real message text, not a claim of exact message equality. The exception
;; *class* and *that it throws at all* match the real host exactly, which is
;; what `try`/`catch` fixtures actually observe.
(defn- expand-case
  [counter args]
  (when (empty? args)
    (throw (ex-info "tiny macroexpander: case needs a test expr" {:args args})))
  (let [[expr & clauses] args
        has-default? (odd? (count clauses))
        default (if has-default?
                  (last clauses)
                  (list 'throw (list 'IllegalArgumentException. "No matching clause")))
        pairs (partition 2 (if has-default? (butlast clauses) clauses))
        g (macro-local "case_test" counter)]
    (list 'let [g expr]
          (reduce (fn [else-form [test-val result]]
                    (list 'if (list '= g test-val) result else-form))
                  default
                  (reverse pairs)))))

(defn- expand-if-let
  [counter args]
  (let [[binding then else] args]
    (when-not (and (vector? binding) (= 2 (count binding)))
      (throw (ex-info "tiny macroexpander: if-let binding" {:args args})))
    (let [[b v] binding
          temp  (macro-local "if_let" counter)]
      (list 'let [temp v]
            (list 'if temp
                  (list 'let [b temp] then)
                  else)))))

(defn- raw-form-contains-symbol?
  [form syms]
  (cond
    (symbol? form) (contains? syms form)
    (coll? form) (boolean (some #(raw-form-contains-symbol? % syms) form))
    :else false))

;; `letfn` desugars into nested `let`s of self-named `fn`s -- confirmed via
;; `javap -c` on a host-AOT-compiled single-binding, non-mutually-recursive
;; `(letfn [(add-x [y] (+ x y))] (add-x 10))`: it is exactly the same shape
;; as `(let [add-x (fn add-x [y] (+ x y))] (add-x 10))` -- capturing `x` as
;; a real closure field, storing the constructed instance in an ordinary
;; local slot, calling it via the same `local-fn-call` mechanism -- ALL
;; already-built machinery, needing zero new bytecode here. Real host's
;; MUTUALLY recursive shape (`even?` calling `odd?` and vice versa) is
;; structurally different (each binding's constructor is called with the
;; OTHER, still-under-construction bindings' current -- possibly still-null
;; -- values, then every field referencing a not-yet-constructed sibling is
;; backpatched via a direct `putfield` once all bindings exist) -- a real,
;; separately-sizeable extension (needs non-final capture fields and a
;; two-phase construct-then-backpatch emission), not attempted here.
;; Detected and rejected with a clear error at macro-expansion time (a
;; simple raw-form symbol-membership scan, cheaper than deferring to
;; analysis's own "unknown local" for a forward-referenced sibling), rather
;; than a confusing downstream failure.
(defn- expand-letfn
  [args]
  (let [[bindings & body] args]
    (when-not (and (vector? bindings)
                   (seq bindings)
                   (every? #(and (seq? %) (symbol? (first %)) (vector? (second %)))
                           bindings))
      (throw (ex-info "tiny macroexpander: letfn requires a vector of (name [params] body...) bindings"
                      {:bindings bindings})))
    (let [fnames (set (map first bindings))]
      (doseq [[fname _params & fbody] bindings]
        (when (raw-form-contains-symbol? fbody (disj fnames fname))
          (throw (ex-info "tiny macroexpander: letfn bindings mutually referencing OTHER letfn siblings are not yet supported -- only self-recursion and references to the enclosing scope are"
                          {:binding-name fname :bindings bindings}))))
      (reduce (fn [inner-body [fname params & fbody]]
                (list 'let [fname (list* 'fn fname params fbody)] inner-body))
              (cons 'do body)
              (reverse bindings)))))

(defn- expand-as->
  [args]
  (let [[expr name & forms] args]
    (when-not (symbol? name)
      (throw (ex-info "tiny macroexpander: as-> name" {:args args})))
    (list 'let (vec (concat [name expr]
                            (mapcat (fn [f] [name f]) forms)))
          name)))

(defn- expand-cond->
  [counter args step-op]
  (let [[expr & clauses] args
        g (macro-local "cond_thread" counter)]
    (when-not (even? (count clauses))
      (throw (ex-info "tiny macroexpander: cond-> clauses" {:args args})))
    (list 'let (vec (concat [g expr]
                            (mapcat (fn [[test form]]
                                      [g (list 'if test (list step-op g form) g)])
                                    (partition 2 clauses))))
          g)))

(defn- expand-some->
  [counter args step-op]
  (let [[expr & forms] args
        g (macro-local "some_thread" counter)]
    (list 'let (vec (concat [g expr]
                            (mapcat (fn [f]
                                      [g (list 'if (list '= g nil)
                                               nil
                                               (list step-op g f))])
                                    forms)))
          g)))

;; 구조분해(vector destructuring): host 의 clojure.core/destructure 없이, 자작 규칙으로
;; `[a b]` 위치 분해를 RT.first/RT.next(=clojure.lang, clojure.core 아님)로 환원한다.
;; `(let [[a b] v] …)` → `(let [g v a (first g) b (first (next g))] …)`.
(defn- nth-via-first-next
  [g i]
  (list 'first (reduce (fn [acc _] (list 'next acc)) g (range i))))

(defn- destructure-pair
  [counter bform init]
  (cond
    (symbol? bform) [bform init]
    (vector? bform)
    (let [g (macro-local "vec_destr" counter)]
      (into [g init]
            (mapcat (fn [i sub]
                      (destructure-pair counter sub (nth-via-first-next g i)))
                    (range)
                    bform)))
    :else
    (throw (ex-info "tiny destructure: unsupported binding form" {:bform bform}))))

(defn- destructure-bindings
  [counter bindings]
  (when-not (and (vector? bindings) (even? (count bindings)))
    (throw (ex-info "tiny destructure: malformed bindings" {:bindings bindings})))
  (vec (mapcat (fn [[b v]] (destructure-pair counter b v))
               (partition 2 bindings))))

(defn- expand-list
  [counter form]
  (let [op (first form)
        args (rest form)]
    (case op
      quote form
      let (let [[bindings & body] args]
            (when-not (vector? bindings)
              (throw (ex-info "tiny macroexpander: let bindings" {:form form})))
            (let [destr (destructure-bindings counter bindings)
                  expanded-binds (vec (mapcat (fn [[s v]] [s (expand-form counter v)])
                                              (partition 2 destr)))]
              (list* 'let expanded-binds
                     (map #(expand-form counter %) body))))
      when (do
             (when (empty? args)
               (throw (ex-info "tiny macroexpander: when arity"
                               {:form form})))
             (expand-form counter
                          (list 'if (first args)
                                (cons 'do (rest args))
                                nil)))
      when-not (do
                 (when (empty? args)
                   (throw (ex-info "tiny macroexpander: when-not arity"
                                   {:form form})))
                 (expand-form counter
                              (list 'if (first args)
                                    nil
                                    (cons 'do (rest args)))))
      if-not (do
               (when-not (<= 2 (count args) 3)
                 (throw (ex-info "tiny macroexpander: if-not arity" {:form form})))
               (expand-form counter
                            (list 'if (first args)
                                  (if (= 3 (count args)) (nth args 2) nil)
                                  (second args))))
      cond (expand-form counter (expand-cond args))
      case (expand-form counter (expand-case counter args))
      if-let (expand-form counter (expand-if-let counter args))
      letfn (expand-form counter (expand-letfn args))
      when-let (do
                 (when (empty? args)
                   (throw (ex-info "tiny macroexpander: when-let arity"
                                   {:form form})))
                 (expand-form counter
                              (expand-if-let counter
                                             (list (first args)
                                                   (cons 'do (rest args))
                                                   nil))))
      and (expand-form counter (expand-and counter args))
      or (expand-form counter (expand-or counter args))
      not (do
            (when-not (= 1 (count args))
              (throw (ex-info "tiny macroexpander: not arity" {:form form})))
            (expand-form counter (list 'if (first args) false true)))
      nil? (do
             (when-not (= 1 (count args))
               (throw (ex-info "tiny macroexpander: nil? arity" {:form form})))
             (expand-form counter (list '= (first args) nil)))
      as-> (expand-form counter (expand-as-> args))
      cond-> (expand-form counter (expand-cond-> counter args '->))
      cond->> (expand-form counter (expand-cond-> counter args '->>))
      some-> (expand-form counter (expand-some-> counter args '->))
      some->> (expand-form counter (expand-some-> counter args '->>))
      -> (do
           (when (empty? args)
             (throw (ex-info "tiny macroexpander: -> arity"
                             {:form form})))
           (expand-form counter (reduce thread-first-step args)))
      ->> (do
            (when (empty? args)
              (throw (ex-info "tiny macroexpander: ->> arity"
                              {:form form})))
            (expand-form counter (reduce thread-last-step args)))
      (apply list (map #(expand-form counter %) form)))))

(defn- expand-form
  [counter form]
  (cond
    (seq? form)
    (expand-list counter form)

    (vector? form)
    (mapv #(expand-form counter %) form)

    (map? form)
    (into (empty form)
          (map (fn [[k v]]
                 [(expand-form counter k)
                  (expand-form counter v)]))
          form)

    (set? form)
    (set (map #(expand-form counter %) form))

    :else form))

(defn- tiny-expand
  [form]
  (expand-form (atom -1) form))

(declare analyze-expr)
(declare analyze-quoted)
(declare analyze-nested-fn)
(declare analyze-reify)

(def ^:private recur-arity-key ::recur-arity)
(def ^:private recur-target-key ::recur-target)

(defn- analyze-body
  [env body]
  (case (count body)
    0 {:op :nil}
    1 (analyze-expr env (first body))
    {:op :do
     :exprs (mapv #(analyze-expr env %) body)}))

(defn- analyze-let
  [env form args]
  (let [[bindings & body] args]
    (when-not (and (vector? bindings)
                   (even? (count bindings))
                   (seq body))
      (throw (ex-info "tiny analyzer: malformed let" {:form form})))
    (loop [pairs (partition 2 bindings)
           env env
           acc []]
      (if (seq pairs)
        (let [[name init] (first pairs)]
          (when-not (symbol? name)
            (throw (ex-info "tiny analyzer: let binding name"
                            {:form form :name name})))
          (let [init-node (analyze-expr env init)]
            (recur (rest pairs)
                   (assoc env name true)
                   (conj acc {:name name :init init-node}))))
        {:op :let
         :bindings acc
         :body (analyze-body env body)}))))

(defn- analyze-loop
  [env form args]
  (let [[bindings & body] args]
    (when-not (and (vector? bindings)
                   (even? (count bindings))
                   (seq body))
      (throw (ex-info "tiny analyzer: malformed loop" {:form form})))
    (loop [pairs (partition 2 bindings)
           env env
           acc []]
      (if (seq pairs)
        (let [[name init] (first pairs)]
          (when-not (symbol? name)
            (throw (ex-info "tiny analyzer: loop binding name"
                            {:form form :name name})))
          (let [init-node (analyze-expr env init)]
            (recur (rest pairs)
                   (assoc env name true)
                   (conj acc {:name name :init init-node}))))
        {:op :loop
         :bindings acc
         :body (analyze-body (assoc env recur-arity-key (count acc))
                             body)}))))

(defn- analyze-recur
  [env form args]
  (let [arity (get env recur-arity-key)]
    (when-not arity
      (throw (ex-info "tiny analyzer: recur outside loop" {:form form})))
    (when-not (= arity (count args))
      (throw (ex-info "tiny analyzer: recur arity"
                      {:form form :expected arity :actual (count args)})))
    {:op :recur
     :exprs (mapv #(analyze-expr env %) args)}))

(defn- analyze-quote
  [form args]
  (when-not (= 1 (count args))
    (throw (ex-info "tiny analyzer: quote arity" {:form form})))
  (analyze-quoted (first args)))

;; A single real, host-interned `:dynamic` Var this backend's `binding`
;; special form (and bare-symbol deref) is allowed to target -- confirmed
;; via `javap -c` that real host's own `binding` compiles to
;; `push-thread-bindings`/`pop-thread-bindings` calls (via
;; `RT.var("clojure.core", ...)` + `IFn.invoke`, the SAME generic Var-call
;; mechanism `str` already uses) wrapped around the body in a try/finally
;; structurally identical to `emit-locking`'s lock-acquire/monitor-exit
;; shape. This tiny language has no `def` at all, so rather than inventing
;; namespace-init machinery, the var this witness's `binding`/deref support
;; targets is simply declared here in the host Clojure source and referenced
;; by bare symbol name -- a small allowlist, same shape as
;; `known-exception-classes` below.
(def ^:dynamic *tiny-dynamic-var* :tiny-dynamic-var-root)

(def ^:private known-dynamic-vars
  {'*tiny-dynamic-var* ["pnix.clj-meta.frontend-selfhost" "*tiny-dynamic-var*"]})

;; Matched by bare NAME regardless of any namespace qualifier on `sym`, so
;; both `*tiny-dynamic-var*` and the fully-qualified
;; `pnix.clj-meta.frontend-selfhost/*tiny-dynamic-var*` resolve to the same
;; allowlist entry -- the DDC row's shared fixture source needs the
;; qualified form so real host `eval`/`compiler.clj` (running with a
;; different `*ns*`) can resolve it too, while this tiny reader's own
;; standalone fixtures use the shorter bare form.
(defn- dynamic-var-target
  [sym]
  (when (symbol? sym)
    (get known-dynamic-vars (symbol (name sym)))))

;; Real host resolves ANY non-special-form call head at compile time via its
;; own namespace/Var machinery, throwing immediately if the symbol doesn't
;; resolve (`Unable to resolve symbol` -- a compile-time error, confirmed
;; live). `clojure.lang.RT.var(ns,name)` itself has no such check -- it
;; happily interns a fresh, unbound Var for a name that doesn't exist yet,
;; which is fine for real host since IT only ever emits that bytecode call
;; after its own compile-time resolution already succeeded. This tiny
;; analyzer replicates that same fail-fast discipline by checking here, at
;; analyze time, before accepting a bare symbol as a general `clojure.core`
;; fn reference (call head or value) -- so a genuine typo still fails
;; loudly instead of silently compiling to a call on a bogus unbound Var.
(defn- core-var-exists?
  [name]
  (some? (ns-resolve (find-ns 'clojure.core) (symbol name))))

;; Deliberately narrow allowlist, not general Java class resolution: this
;; tiny language has no way to name arbitrary host classes, so both `catch`
;; targets and constructible exception types (below) are restricted to a
;; small set actually reachable from ops this backend supports (`quot`/`rem`
;; throw ArithmeticException on divide-by-zero; `IllegalArgumentException`
;; is what `case`'s no-default no-match path needs to match real host
;; behavior, see `expand-case`).
(def ^:private known-exception-classes
  {'ArithmeticException ArithmeticException
   'Exception Exception
   'RuntimeException RuntimeException
   'Throwable Throwable
   'IllegalArgumentException IllegalArgumentException})

;; General fallback for a `catch` class NOT in the small
;; `known-exception-classes` allowlist above (checked first; this is only
;; reached when that lookup misses) -- e.g. `clojure.lang.ExceptionInfo`,
;; the type `ex-info` actually throws. Unlike `general-static-interop`/
;; `general-constructor-class-name`, this needs NO runtime bytecode call at
;; all: a JVM exception-table entry (`visitTryCatchBlock`) only ever needs
;; the class's INTERNAL NAME as a compile-time string constant, and
;; `Class/forName` on the HOST side at analyze time already gives that for
;; free via `Type/getInternalName` -- confirmed by checking every existing
;; `:catch-class` use site only ever calls `Type/getInternalName` on it,
;; never emits a class-resolution call into the generated bytecode.
(defn- general-catch-class
  [sym]
  (when (and (symbol? sym) (not (namespace sym)))
    (try
      (let [cls (Class/forName (name sym))]
        (when (.isAssignableFrom Throwable cls)
          cls))
      (catch Throwable _ nil))))

(defn- exception-constructor-class
  "`ClassName.` (the real host's dot-suffixed constructor-call syntax) for
  an allowlisted exception class -> the Class; else nil. Reader-level: the
  tiny reader already reads `IllegalArgumentException.` as a plain symbol
  (the trailing `.` is just part of the symbol's name), so no reader change
  is needed to recognize this shape."
  [op]
  (when (and (symbol? op) (.endsWith (name op) "."))
    (get known-exception-classes (symbol (subs (name op) 0 (dec (count (name op))))))))

;; General `ClassName.` construction, for any class NOT in the small
;; exception allowlist above (checked first; this is only reached as a
;; fallback). Unlike the exception path's direct `NEW`/`INVOKESPECIAL`
;; (valid there because every allowlisted class only ever needs a 0- or
;; 1-arg constructor this backend hand-picks), this is genuinely general:
;; any class name, any arg count. Confirmed via `javap -c` that real host
;; ALSO falls back to exactly this mechanism -- `RT.classForName(String)` +
;; `Reflector.invokeConstructor(Class, Object[])` -- whenever the
;; (class, arity) pair does not uniquely identify one constructor at
;; compile time (e.g. `java.util.ArrayList(int)` is ambiguous with
;; `ArrayList(Collection)`); when it IS unique (e.g. `java.awt.Point(int,
;; int)`, `java.util.ArrayList()`), real host instead emits a direct
;; `NEW`/`INVOKESPECIAL`, an optimization this backend does not attempt to
;; replicate for the general case -- same behavior-not-bytecode-shape bar
;; already established for `case`/static interop. Honest caveat: an
;; unresolvable class name here fails at RUNTIME (when `RT.classForName`
;; runs), not at analyze time the way real host's own compile-time
;; resolution would reject it -- a difference only for malformed source,
;; not for any value this backend's fixtures compute.
(defn- general-constructor-class-name
  [op]
  (when (and (symbol? op) (< 1 (count (name op))) (.endsWith (name op) "."))
    (subs (name op) 0 (dec (count (name op))))))

;; `.methodName receiver args...` -- the real host's own dot-prefixed
;; instance-interop syntax. Unlike the constructor allowlist above, this is
;; NOT restricted to known classes: real Clojure itself falls back to
;; `clojure.lang.Reflector.invokeInstanceMethod` (dynamic, name+arg-based
;; dispatch, resolved at runtime) whenever it cannot statically prove the
;; receiver's type from a type hint -- confirmed live via `javap -c` on a
;; host-AOT-compiled untyped `(.getMessage e)` and `(.equals a b)`. This
;; tiny language has no type hints at all, so every interop call here takes
;; that same fallback path on the real host too -- meaning this is not an
;; approximation of real host behavior, it is the same mechanism.
;; `.-fieldName receiver` -- the real host's own dot-dash-prefixed field
;; access syntax. Checked and dispatched BEFORE `interop-method-name` below:
;; `.-x` also satisfies a naive "starts with `.`" test, so `field-access-name`
;; must win first, and `interop-method-name` explicitly excludes `.-`-prefixed
;; names as a second, redundant safeguard against that exact collision.
(defn- field-access-name
  [op]
  (when (and (symbol? op) (< 2 (count (name op))) (.startsWith (name op) ".-"))
    (subs (name op) 2)))

(defn- interop-method-name
  [op]
  (when (and (symbol? op)
             (< 1 (count (name op)))
             (.startsWith (name op) ".")
             (not (.startsWith (name op) ".-")))
    (subs (name op) 1)))

;; `ClassName/methodName args...` -- static interop. Real host resolves the
;; target class+method at COMPILE time (it can, since the class name is
;; syntactically present) and often emits a direct `invokestatic`, unlike
;; the always-runtime-reflective instance-interop path above -- confirmed
;; via `javap -c` on `(Integer/toString x)` for an untyped `x`, which still
;; got compile-time-resolved to a direct call, not Reflector. Matching that
;; exact compile-time-overload-resolution mechanism is out of scope here;
;; instead this uses `clojure.lang.Reflector.invokeStaticMethod`, the same
;; *runtime* dispatch primitive Reflector itself exposes for static calls --
;; same behavior-equivalence bar as `case`'s `=`-chain-instead-of-switch
;; choice: matching results, not matching bytecode shape. Deliberately
;; scoped to a small class allowlist (this tiny language has no general
;; class-name resolution), reusing the reader's own namespace-qualified
;; symbol parsing: `(symbol "Math/sqrt")` already splits into namespace
;; "Math" + name "sqrt" with zero reader changes needed.
(def ^:private known-static-classes
  {"Math" Math
   "Integer" Integer
   "Long" Long
   "String" String})

(defn- static-interop-target
  [op]
  (when (and (symbol? op) (namespace op))
    (when-let [cls (get known-static-classes (namespace op))]
      [cls (name op)])))

;; `pkg.pkg.ClassName/methodName args...` -- the general fallback for any
;; class NOT in the small `known-static-classes` allowlist above (checked
;; first; this is only reached when that lookup misses). Confirmed via
;; `javap -c` on `(Character/isDigit c)` for an untyped `c`: real host
;; resolves even a SHORT class name like `Character` to its fully-qualified
;; `java.lang.Character` (via its own default-imports table for
;; `java.lang.*`) and, since `Character.isDigit` has two ambiguous 1-arg
;; overloads (`char` and `int`), falls back to the exact same
;; `RT.classForName(String)` + `Reflector.invokeStaticMethod(Class, String,
;; Object[])` mechanism `general-constructor-class-name` already uses for
;; construction. This tiny language has no import table of its own, so
;; unlike real host it requires the FULLY QUALIFIED class name here (e.g.
;; `java.lang.Character/isDigit`, not bare `Character/isDigit`) -- an
;; honest, narrower scope than real host's own name resolution, not a
;; claim of matching it.
(defn- general-static-interop-target
  [op]
  (when (and (symbol? op) (namespace op) (not (contains? known-static-classes (namespace op))))
    [(namespace op) (name op)]))

;; Bound (only) by `compile-source`'s `(do (deftype ...)... (fn ...))`
;; top-level program path -- see there -- to a `{type-name-symbol Class}`
;; registry of `deftype`s defined earlier in the SAME program, so
;; `(Name. args...)` inside the trailing `fn` can construct them directly.
;; A dynamic var, matching real host's own use of compilation-scoped
;; dynamic state (`*ns*` et al.), rather than threading an extra parameter
;; through every analyze/emit function signature in this file.
(def ^:private ^:dynamic *known-deftype-classes* {})

(defn- known-deftype-constructor-class
  [op]
  (when (and (symbol? op) (< 1 (count (name op))) (.endsWith (name op) "."))
    (get *known-deftype-classes* (symbol (subs (name op) 0 (dec (count (name op))))))))

;; Bound alongside `*known-deftype-classes*` by the SAME top-level program
;; path, now generalized to also accept leading `defprotocol` forms (see
;; `compile-source`). `*known-protocol-methods*` maps a protocol method
;; NAME symbol to `{:interface Class :method String :arity N}` (`N` =
;; explicit args, not counting the receiver) so `(methodName instance
;; args...)` anywhere in the trailing `fn` compiles as a direct protocol
;; dispatch call; `*known-protocol-interfaces*` maps the PROTOCOL name
;; itself to its generated interface Class so `(reify ProtocolName ...)`
;; can implement it (see `analyze-reify`'s interface resolution).
(def ^:private ^:dynamic *known-protocol-methods* {})
(def ^:private ^:dynamic *known-protocol-interfaces* {})

(defn- known-protocol-method
  [op arg-count]
  (when (symbol? op)
    (when-let [m (get *known-protocol-methods* op)]
      (when (= (:arity m) (dec arg-count))
        m))))

(defn- analyze-call
  [env form]
  (let [[op & args] form]
    (cond
      (known-deftype-constructor-class op)
      {:op :deftype-new
       :class (known-deftype-constructor-class op)
       :args (mapv #(analyze-expr env %) args)}

      ;; `(methodName instance args...)` -- a protocol method call,
      ;; confirmed via `javap -c` on real host's FAST PATH (an
      ;; `instance?`-implements-the-generated-interface check emitted
      ;; inline by callers, `checkcast Interface; invokeinterface
      ;; method`): a plain interface dispatch call, nothing more. Real
      ;; host's FULL mechanism additionally falls back to
      ;; `clojure.core/-cache-protocol-fn` (backed by
      ;; `AFunction.__methodImplCache`) for values that DON'T directly
      ;; implement the generated interface -- e.g. a protocol extended onto
      ;; `java.lang.String` via `extend-protocol` -- which is a real,
      ;; separately-sizeable reimplementation of a chunk of Clojure's own
      ;; protocol runtime, not attempted here: this witness's protocol
      ;; method calls only work on values that directly `reify` (or, once
      ;; extended, `deftype`) the protocol.
      (known-protocol-method op (count args))
      (let [{:keys [interface method]} (known-protocol-method op (count args))]
        {:op :protocol-call
         :interface interface
         :method method
         :instance (analyze-expr env (first args))
         :args (mapv #(analyze-expr env %) (rest args))})

      (exception-constructor-class op)
      (let [cls (exception-constructor-class op)]
        (when-not (<= 0 (count args) 1)
          (throw (ex-info "tiny analyzer: exception constructor takes 0 or 1 (String message) args"
                          {:form form})))
        {:op :new
         :class cls
         :arg (when (seq args) (analyze-expr env (first args)))})

      (general-constructor-class-name op)
      {:op :general-new
       :class-name (general-constructor-class-name op)
       :args (mapv #(analyze-expr env %) args)}

      (field-access-name op)
      (do
        (when-not (= 1 (count args))
          (throw (ex-info "tiny analyzer: field access takes exactly one receiver" {:form form})))
        {:op :field-get
         :field (field-access-name op)
         :receiver (analyze-expr env (first args))})

      (interop-method-name op)
      (do
        (when (empty? args)
          (throw (ex-info "tiny analyzer: interop call needs a receiver" {:form form})))
        {:op :interop-call
         :method (interop-method-name op)
         :receiver (analyze-expr env (first args))
         :args (mapv #(analyze-expr env %) (rest args))})

      (static-interop-target op)
      (let [[cls method] (static-interop-target op)]
        {:op :static-interop-call
         :class cls
         :method method
         :args (mapv #(analyze-expr env %) args)})

      (general-static-interop-target op)
      (let [[class-name method] (general-static-interop-target op)]
        {:op :general-static-interop-call
         :class-name class-name
         :method method
         :args (mapv #(analyze-expr env %) args)})

      :else
      (case op
      quote (analyze-quote form args)
      do (analyze-body env args)
      let (analyze-let env form args)
      loop (analyze-loop env form args)
      recur (analyze-recur env form args)
      try
      (do
        ;; Deliberately narrow scope: exactly one body expression, then zero
        ;; or more `catch` clauses (each a distinct allowlisted class),
        ;; optionally followed by exactly one `finally` clause (no
        ;; multi-form try/catch/finally bodies, no `finally` before
        ;; `catch`). Multi-catch (N catches, no finally) reuses the same
        ;; per-clause helpers as the single-catch/finally/catch+finally
        ;; shapes below; combining multi-catch WITH finally is a further,
        ;; separate slice not attempted here.
        (when (empty? args)
          (throw (ex-info "tiny analyzer: try requires a body form" {:form form})))
        (let [[body-form & clause-forms] args
              catch-form? #(and (seq? %) (= 'catch (first %)))
              finally-form? #(and (seq? %) (= 'finally (first %)))
              analyze-catch-clause
              (fn [clause-form]
                (let [[_ class-sym name-sym catch-body-form] clause-form
                      catch-class (or (get known-exception-classes class-sym)
                                      (general-catch-class class-sym))]
                  (when-not (and (= 4 (count clause-form))
                                 catch-class
                                 (symbol? name-sym))
                    (throw (ex-info "tiny analyzer: malformed catch clause"
                                    {:form form})))
                  {:catch-class catch-class
                   :catch-name name-sym
                   :catch-body (analyze-expr (assoc env name-sym true) catch-body-form)}))
              analyze-finally-clause
              (fn [clause-form]
                (when-not (= 2 (count clause-form))
                  (throw (ex-info "tiny analyzer: finally takes exactly one body form"
                                  {:form form})))
                (analyze-expr env (second clause-form)))]
          (cond
            (empty? clause-forms)
            (analyze-expr env body-form)

            (and (= 1 (count clause-forms)) (catch-form? (first clause-forms)))
            (merge {:op :try :body (analyze-expr env body-form)}
                   (analyze-catch-clause (first clause-forms)))

            (and (= 1 (count clause-forms)) (finally-form? (first clause-forms)))
            {:op :try-finally
             :body (analyze-expr env body-form)
             :finally-body (analyze-finally-clause (first clause-forms))}

            (and (= 2 (count clause-forms))
                 (catch-form? (first clause-forms))
                 (finally-form? (second clause-forms)))
            (merge {:op :try-catch-finally :body (analyze-expr env body-form)}
                   (analyze-catch-clause (first clause-forms))
                   {:finally-body (analyze-finally-clause (second clause-forms))})

            (and (< 2 (count clause-forms))
                 (finally-form? (last clause-forms))
                 (every? catch-form? (butlast clause-forms)))
            (let [catches (mapv analyze-catch-clause (butlast clause-forms))
                  classes (map :catch-class catches)]
              (when-not (= (count classes) (count (distinct classes)))
                (throw (ex-info "tiny analyzer: duplicate catch class in multi-catch"
                                {:form form})))
              {:op :try-multi-catch-finally
               :body (analyze-expr env body-form)
               :catches catches
               :finally-body (analyze-finally-clause (last clause-forms))})

            (and (< 1 (count clause-forms)) (every? catch-form? clause-forms))
            (let [catches (mapv analyze-catch-clause clause-forms)
                  classes (map :catch-class catches)]
              (when-not (= (count classes) (count (distinct classes)))
                (throw (ex-info "tiny analyzer: duplicate catch class in multi-catch"
                                {:form form})))
              {:op :try-multi-catch
               :body (analyze-expr env body-form)
               :catches catches})

            :else
            (throw (ex-info "tiny analyzer: try's clauses must be zero or more catch clauses (each a distinct allowlisted class) optionally followed by one finally clause"
                            {:form form})))))
      if (do
           (when-not (= 3 (count args))
             (throw (ex-info "tiny analyzer: if arity" {:form form})))
           {:op :if
            :test (analyze-expr env (nth args 0))
            :then (analyze-expr env (nth args 1))
            :else (analyze-expr env (nth args 2))})
      (quot rem)
      (do
        (when-not (= 2 (count args))
          (throw (ex-info "tiny analyzer: binary op arity"
                          {:form form})))
        {:op :binary
         :fn op
         :lhs (analyze-expr env (first args))
         :rhs (analyze-expr env (second args))})
      ;; Unlike `+`/`-`/`*`, real host does NOT inline-fold `<`/`=`/`>`/
      ;; `>=`/`<=` for arities other than exactly 2 -- confirmed via
      ;; `javap -c` on `(< a b c)` AND `(< a)`: both compile to a plain
      ;; `RT.var("clojure.core","<").getRawRoot()` + `IFn.invoke(...)`
      ;; call, the SAME general Var-call mechanism `core-fn-call` already
      ;; uses, not a chained/folded `Numbers.lt` sequence. So the 2-arg
      ;; case keeps the existing direct `Numbers.lt`-style `:binary` fast
      ;; path (matching real host's own 2-arg inliner), and any OTHER
      ;; arity falls back to `core-fn-call` -- which is not merely
      ;; behaviorally equivalent here, it is the exact same bytecode
      ;; mechanism real host itself falls back to.
      (< = > >= <=)
      (if (= 2 (count args))
        {:op :binary
         :fn op
         :lhs (analyze-expr env (first args))
         :rhs (analyze-expr env (second args))}
        {:op :core-fn-call
         :fn-name (name op)
         :args (mapv #(analyze-expr env %) args)})
      ;; `get` has a real 2-arg AND 3-arg (default value) form on real
      ;; host, each a DIFFERENT direct `RT.get` overload -- confirmed via
      ;; `javap -c` on `(get m k d)`: `RT.get(Object,Object,Object)`, not a
      ;; Var-call fallback like the comparison ops above.
      get
      (cond
        (= 2 (count args))
        {:op :binary
         :fn op
         :lhs (analyze-expr env (first args))
         :rhs (analyze-expr env (second args))}

        (= 3 (count args))
        {:op :get3
         :map (analyze-expr env (first args))
         :key (analyze-expr env (second args))
         :default (analyze-expr env (nth args 2))}

        :else
        (throw (ex-info "tiny analyzer: get arity" {:form form})))
      ;; `+`/`-`/`*` are variadic on real host, unlike the strictly-binary
      ;; comparison ops above -- confirmed via `javap -c` that `(+ a b c)`
      ;; compiles to LEFT-FOLDED nested `Numbers.add` calls
      ;; (`Numbers.add(Numbers.add(a,b),c)`), so N>2 args are desugared
      ;; here at analyze time into nested `:binary` nodes, reusing the
      ;; existing emitter unchanged. 0-arg `(+)`/`(*)` and 1-arg
      ;; `(+ a)`/`(* a)` match real host's own identity-element/passthrough
      ;; values (confirmed live: `(+)` => 0, `(*)` => 1, `(+ a)`/`(* a)` =>
      ;; `a` itself, no-op). `(- a)` (unary negation, confirmed via
      ;; `javap -c` to be `Numbers.minus(Object)`, a genuinely different
      ;; single-arg overload, not a 2-arg subtraction) is handled by the
      ;; `unary` case below instead, alongside `inc`/`dec`/etc.
      (+ - *)
      (cond
        (and (= '- op) (empty? args))
        (throw (ex-info "tiny analyzer: - requires at least 1 arg" {:form form}))

        (empty? args)
        {:op :const :value (if (= '+ op) 0 1)}

        (= 1 (count args))
        (if (= '- op)
          {:op :unary :fn '- :arg (analyze-expr env (first args))}
          (analyze-expr env (first args)))

        :else
        (reduce (fn [lhs-node arg-form]
                  {:op :binary :fn op :lhs lhs-node :rhs (analyze-expr env arg-form)})
                (analyze-expr env (first args))
                (rest args)))
      (inc dec zero? pos? neg? first next count)
      (do
        (when-not (= 1 (count args))
          (throw (ex-info "tiny analyzer: unary op arity"
                          {:form form})))
        {:op :unary
         :fn op
         :arg (analyze-expr env (first args))})
      throw
      (do
        (when-not (= 1 (count args))
          (throw (ex-info "tiny analyzer: throw arity" {:form form})))
        {:op :throw
         :expr (analyze-expr env (first args))})
      set!
      (do
        (when-not (= 2 (count args))
          (throw (ex-info "tiny analyzer: set! arity" {:form form})))
        (let [[target-form value-form] args]
          (when-not (and (seq? target-form)
                         (= 2 (count target-form))
                         (field-access-name (first target-form)))
            (throw (ex-info "tiny analyzer: set! target must be a field access, (set! (.-field expr) value)"
                            {:form form})))
          {:op :field-set
           :field (field-access-name (first target-form))
           :receiver (analyze-expr env (second target-form))
           :value (analyze-expr env value-form)}))
      locking
      (do
        ;; Deliberately narrow scope: a lock expression plus exactly one
        ;; body expression (no multi-form `locking` body), matching this
        ;; file's established minimal-scope pattern.
        (when-not (= 2 (count args))
          (throw (ex-info "tiny analyzer: locking requires a lock expression and one body expression"
                          {:form form})))
        {:op :locking
         :lock (analyze-expr env (first args))
         :body (analyze-expr env (second args))})
      binding
      (do
        ;; Deliberately narrow scope, matching `locking`'s: exactly one
        ;; [var value] binding pair (no multi-binding `binding` vector),
        ;; and the target var must be in the small `known-dynamic-vars`
        ;; allowlist (this tiny language has no `def`, see there for why).
        (when-not (and (= 2 (count args))
                       (vector? (first args))
                       (= 2 (count (first args))))
          (throw (ex-info "tiny analyzer: binding requires exactly one [var value] pair and one body expression"
                          {:form form})))
        (let [[var-sym value-form] (first args)
              body-form (second args)]
          (when-not (dynamic-var-target var-sym)
            (throw (ex-info "tiny analyzer: binding target must be a known dynamic var"
                            {:form form})))
          (let [[var-ns var-name] (dynamic-var-target var-sym)]
            {:op :binding
             :var-ns var-ns
             :var-name var-name
             :value (analyze-expr env value-form)
             :body (analyze-expr env body-form)})))
      fn
      (analyze-nested-fn env args)
      reify
      (analyze-reify env args)
      (cond
        ;; A CALL HEAD that is itself an expression, not a bare symbol,
        ;; e.g. `((constantly x) 99)` -- confirmed via `javap -c`: real
        ;; host just emits whatever bytecode the head expression itself
        ;; produces, `checkcast`s the result straight to `IFn`, and
        ;; invokes it with the args -- no Var lookup, no local slot,
        ;; exactly the same "cast and invoke" tail `local-fn-call`/
        ;; `core-fn-call` below already share, just with a general
        ;; expression (not a symbol lookup) supplying the callee.
        (not (symbol? op))
        {:op :computed-fn-call
         :fn-expr (analyze-expr env op)
         :args (mapv #(analyze-expr env %) args)}

        ;; A local (a fn parameter, or a `let` binding) used as a call
        ;; head, e.g. `(f x)` where `f` is itself a parameter -- confirmed
        ;; via `javap -c` on a host-AOT-compiled `(fn [f x] (f x))`: real
        ;; host emits exactly `aload f; checkcast IFn; aload x;
        ;; invokeinterface IFn.invoke`, the local's already-loaded value
        ;; cast straight to `IFn` and invoked, no Var lookup at all (it
        ;; isn't a Var reference to begin with). Checked BEFORE the
        ;; `core-var-exists?` fallback below since a local binding always
        ;; shadows any same-named `clojure.core` fn, matching real host's
        ;; own lexical-scope-first resolution order.
        (and (symbol? op) (contains? env op))
        {:op :local-fn-call
         :name op
         :args (mapv #(analyze-expr env %) args)}

        ;; General fallback for any `clojure.core` function not otherwise
        ;; special-cased above (`str` was the first fixture-tested case;
        ;; this generalizes that same mechanism to the rest of
        ;; `clojure.core`'s public surface -- `map`/`filter`/`reduce`/
        ;; `apply`/`conj`/`assoc`/... all resolve identically on real host,
        ;; confirmed via `javap -c`: no special form is compiler-special
        ;; here, every one of them is Var-lookup-then-invoke). Gated on
        ;; `core-var-exists?` so a genuine typo/unknown symbol still fails
        ;; at analyze time with a clear error, matching real host's own
        ;; compile-time resolution failure, rather than silently compiling
        ;; to a call on a freshly auto-interned unbound Var (what
        ;; `RT.var(ns,name)` alone would do with no such check).
        (and (symbol? op) (core-var-exists? (name op)))
        {:op :core-fn-call
         :fn-name (name op)
         :args (mapv #(analyze-expr env %) args)}

        :else
        (throw (ex-info "tiny analyzer: unsupported call"
                        {:form form :op op})))))))

(defn- analyze-expr
  [env form]
  (cond
    (nil? form)
    {:op :const :value nil}

    (boolean? form)
    {:op :const :value form}

    (string? form)
    {:op :const :value form}

    (keyword? form)
    {:op :const :value form}

    ;; Checked before the generic `integer?` branch below: `(long form)`
    ;; there would silently truncate a `BigInt` past Long/MAX_VALUE.
    ;; `BigDecimal` isn't `integer?` at all, so it needs its own branch
    ;; regardless.
    (or (instance? BigInt form) (instance? BigDecimal form)
        (instance? Pattern form) (instance? Ratio form))
    {:op :const :value form}

    (integer? form)
    {:op :const :value (long form)}

    (vector? form)
    {:op :vector
     :items (mapv #(analyze-expr env %) form)}

    (map? form)
    {:op :map
     :entries (mapv (fn [[k v]]
                      {:key (analyze-expr env k)
                       :val (analyze-expr env v)})
                    form)}

    (set? form)
    {:op :set
     :items (mapv #(analyze-expr env %) form)}

    (symbol? form)
    (cond
      (contains? env form)
      {:op :local :name form}

      (dynamic-var-target form)
      (let [[var-ns var-name] (dynamic-var-target form)]
        {:op :var-deref :var-ns var-ns :var-name var-name})

      (core-var-exists? (name form))
      {:op :core-fn-value :fn-name (name form)}

      :else
      (throw (ex-info "tiny analyzer: unknown local" {:symbol form})))

    (seq? form)
    (analyze-call env form)

    :else
    (throw (ex-info "tiny analyzer: unsupported form" {:form form}))))

(defn- analyze-quoted
  [form]
  (cond
    (or (nil? form)
        (boolean? form)
        (string? form)
        (keyword? form)
        (symbol? form)
        (integer? form))
    {:op :const :value form}

    (vector? form)
    {:op :vector
     :items (mapv analyze-quoted form)}

    (map? form)
    {:op :map
     :entries (mapv (fn [[k v]]
                      {:key (analyze-quoted k)
                       :val (analyze-quoted v)})
                    form)}

    (set? form)
    {:op :set
     :items (mapv analyze-quoted form)}

    (seq? form)
    {:op :list
     :items (mapv analyze-quoted form)}

    :else
    (throw (ex-info "tiny analyzer: unsupported quoted form"
                    {:form form}))))

(defn- split-variadic-params
  "`[a b & r]` -> `[[a b] r]`; `[a b]` -> `[[a b] nil]`. `&` must be
  second-to-last (exactly one name follows it), matching the one shape
  `clojure.lang.RestFn` itself supports."
  [params]
  (let [amp-idx (first (keep-indexed (fn [i p] (when (= p '&) i)) params))]
    (if (nil? amp-idx)
      [params nil]
      (do
        (when-not (= amp-idx (- (count params) 2))
          (throw (ex-info "tiny analyzer: `&` must be second-to-last in params"
                          {:params params})))
        [(subvec params 0 amp-idx) (nth params (inc amp-idx))]))))

(defn- analyze-fn-clause
  "A single arity clause `(params-vector body-form...)`, shared by both the
  single-arity `(fn [x] ...)` shape (where the whole tail IS one clause) and
  each `([x] ...)` entry of a multi-arity `(fn ([x] ...) ([x y] ...))` form.
  `params-vector` may end in `& rest-name` for a variadic clause. `fn-name`,
  when non-nil (an optional self-reference name, `(fn name [x] ...)`), is
  added to the clause's own env so calls to it inside the body analyze as an
  ordinary local-fn-call -- a param of the same name shadows it, matching
  real host (params are bound after/inside the name's own scope). `base-env`
  is `{}` for a top-level `(compile-source ...)` fn, or the ENCLOSING fn's
  own env for a nested closure literal -- letting names free in the nested
  body still resolve as bound (`analyze-nested-fn` computes which of them
  were actually reached afterward, to decide what the closure must capture)."
  [clause fn-name base-env]
  (let [raw-params (first clause)
        body (rest clause)]
    (when-not (and (vector? raw-params) (seq body))
      (throw (ex-info "tiny analyzer: malformed fn clause" {:clause clause})))
    (let [[params rest-param] (split-variadic-params raw-params)
          all-names (cond-> params rest-param (conj rest-param))]
      (when-not (and (every? symbol? all-names)
                     (= (count all-names) (count (distinct all-names))))
        (throw (ex-info "tiny analyzer: malformed fn clause" {:clause clause})))
      (let [env (cond-> (into base-env (zipmap all-names (repeat true)))
                  fn-name (assoc fn-name true)
                  ;; `recur` at the fn body's own tail position (no
                  ;; enclosing `loop`) targets THIS clause's own params --
                  ;; confirmed via `javap -c` on a host-AOT-compiled
                  ;; `(fn [n] (if (= n 0) 0 (recur (- n 1))))`: just a plain
                  ;; `astore`-into-the-arg-slot + `goto` back to the
                  ;; method's own top, the exact same GOTO-loop shape
                  ;; `loop`/`recur` already implements here, just targeting
                  ;; the method's argument slots instead of fresh `loop`
                  ;; locals. A nested `loop` shadows this via the same env
                  ;; key, matching real host's own "nearest enclosing
                  ;; loop/fn" `recur` target rule for free.
                  true (assoc recur-arity-key (count all-names)))]
        {:params params
         :rest-param rest-param
         :body (analyze-body env body)}))))

(def ^:private closure-depth-key ::closure-depth)

;; Generic AST walk collecting every `:name` referenced via a `:local` or
;; `:local-fn-call` node anywhere in `node` -- used by `analyze-nested-fn` to
;; find a closure's free variables AFTER analysis, since the analyze-time
;; env carries no `:kind` distinction between "this fn's own binding" and
;; "resolved from an enclosing scope" (see `analyze-fn-clause`). Walks any
;; map/vector shape generically (no per-op knowledge needed): a `core-fn-value`/
;; `var-deref`/etc. node is correctly NOT collected, since those resolve via
;; a Var lookup at emit time regardless of lexical scope -- only genuine
;; lexical references need capturing.
(defn- ast-referenced-names
  [node]
  (cond
    (and (map? node) (contains? #{:local :local-fn-call} (:op node)))
    (into #{(:name node)} (mapcat ast-referenced-names (vals (dissoc node :op :name))))

    ;; A NESTED closure's own `:captures` (already computed as free
    ;; relative to IT) must also count as "referenced" by whatever
    ;; encloses it -- confirmed via `javap -c` on a host-AOT-compiled
    ;; `(fn [x] (fn [y] (fn [z] (+ x (+ y z)))))`: the MIDDLE closure
    ;; captures `x` as one of its OWN fields even though its OWN body
    ;; never references `x` directly -- it only needs `x` to construct
    ;; the INNERMOST closure. Deliberately do NOT recurse into the nested
    ;; closure's `:body` here: its OWN params/self-name are already
    ;; excluded from ITS OWN captures (by `analyze-nested-fn`), so walking
    ;; into `:body` would only surface names meaningless to THIS
    ;; (enclosing) scope's own capture computation.
    (and (map? node) (= :closure (:op node)))
    (set (:captures node))

    (map? node)
    (into #{} (mapcat ast-referenced-names (vals node)))

    (sequential? node)
    (into #{} (mapcat ast-referenced-names node))

    :else
    #{}))

(defn- analyze-nested-fn
  "A `fn` literal appearing WITHIN another fn's body (not the top-level form
  `compile-source` itself compiles) -- a genuine closure, unlike the
  top-level case. Deliberately narrow scope: a single arity clause only (no
  `([x] ..) ([x y] ..)` multi-arity nested closures) -- a separate,
  sizeable extension, not attempted here. Nesting depth itself is
  UNBOUNDED (a closure inside a closure inside a closure...): confirmed
  via `javap -c` on a host-AOT-compiled `(fn [x] (fn [y] (fn [z] (+ x (+ y
  z)))))` that a MIDDLE closure captures a name purely to pass it through
  to an INNER closure's constructor, even when the middle closure's own
  body never references that name directly -- `ast-referenced-names`
  handles this by treating a nested `:closure` node's own `:captures` as
  \"referenced\" by whatever encloses it, so this transitive threading
  falls out of the SAME free-variable computation used for one level,
  with no depth-specific logic needed at all."
  [env args]
  (let [fn-name (when (symbol? (first args)) (first args))
        rest-args (if fn-name (rest args) args)]
    (when (and (seq rest-args) (seq? (first rest-args)))
      (throw (ex-info "tiny analyzer: a nested (non-top-level) fn literal supports a single arity clause only"
                      {:form args})))
    (let [{:keys [params rest-param body]}
          (analyze-fn-clause rest-args fn-name (assoc env closure-depth-key true))
          own-names (cond-> (set params)
                      rest-param (conj rest-param)
                      fn-name (conj fn-name))
          referenced (ast-referenced-names body)
          outer-names (disj (set (keys env)) closure-depth-key)
          captures (vec (filter #(and (outer-names %) (not (own-names %))) referenced))]
      {:op :closure
       :fn-name fn-name
       :params params
       :rest-param rest-param
       :body body
       :captures captures})))

;; `reify` -- confirmed via `javap -c` on a host-AOT-compiled `(reify
;; Comparator (compare [this a b] ...))`: a fresh class `implements` the
;; named interface(s) directly (NOT extending `AFunction`/`RestFn` --
;; `reify` doesn't make the result callable as a plain fn, only as the
;; reified interface(s)), captures free variables as instance fields
;; exactly like a closure, and always ALSO implements `clojure.lang.IObj`
;; (`meta`/`withMeta`) for metadata support. That `IObj` boilerplate is
;; deliberately NOT reproduced here -- it's orthogonal to the reified
;; interface's own observable behavior (nothing this witness's own fixtures
;; exercise ever calls `.meta` on a reified instance), matching the
;; established "behavior equivalence, not bytecode-shape equivalence" bar
;; used throughout this file. Deliberately narrow scope beyond that:
;; exactly ONE fully-qualified interface (no multi-interface `reify`, no
;; protocols specifically), and every implemented method's parameters must
;; be reference types (a primitive parameter, e.g. `int`, would need
;; auto-boxing on method entry before this witness's uniformly-`Object`
;; local-resolution machinery could use it -- not attempted here; a
;; PRIMITIVE RETURN type, e.g. `Comparator/compare`'s `int`, IS supported,
;; since coercing this witness's always-boxed body result down to a
;; primitive at the `return` site is the much more common/needed case and
;; doesn't touch the local-resolution machinery at all).
(defn- reflect-interface-method
  ^java.lang.reflect.Method [^Class iface method-name arg-count]
  (first (filter (fn [^java.lang.reflect.Method m]
                    (and (= method-name (.getName m))
                         (= arg-count (.getParameterCount m))))
                  (.getMethods iface))))

(defn- analyze-reify-method
  [env ^Class iface method-form]
  (when-not (and (seq? method-form) (symbol? (first method-form))
                 (vector? (second method-form)) (seq (second method-form)))
    (throw (ex-info "tiny analyzer: malformed reify method (name [this args...] body...)"
                    {:form method-form})))
  (let [[mname params & body] method-form
        this-sym (first params)
        arg-syms (vec (rest params))
        rmethod (reflect-interface-method iface (name mname) (count arg-syms))]
    (when-not rmethod
      (throw (ex-info "tiny analyzer: reify method does not match any method on the reified interface"
                      {:method mname :arg-count (count arg-syms) :interface iface})))
    (when (some #(.isPrimitive ^Class %) (.getParameterTypes rmethod))
      (throw (ex-info "tiny analyzer: reify methods with primitive parameters are not yet supported"
                      {:method mname})))
    (let [method-env (into (assoc env this-sym true) (zipmap arg-syms (repeat true)))]
      {:name (name mname)
       :this-sym this-sym
       :arg-syms arg-syms
       :reflected rmethod
       :body (analyze-body method-env body)})))

(defn- analyze-reify
  [env args]
  (when (contains? env closure-depth-key)
    (throw (ex-info "tiny analyzer: reify nested inside a closure is not yet supported" {:form args})))
  (let [[iface-sym & method-forms] args]
    (when-not (and (symbol? iface-sym) (not (namespace iface-sym)))
      (throw (ex-info "tiny analyzer: reify requires a single fully-qualified interface name"
                      {:form args})))
    (let [iface (or (get *known-protocol-interfaces* iface-sym)
                    (try (Class/forName (name iface-sym)) (catch Throwable _ nil)))]
      (when-not (and iface (.isInterface ^Class iface))
        (throw (ex-info "tiny analyzer: reify interface not found (must be a known protocol name or a fully-qualified interface name)"
                        {:interface iface-sym})))
      (let [inner-env (assoc env closure-depth-key true)
            methods (mapv #(analyze-reify-method inner-env iface %) method-forms)
            own-names (fn [{:keys [this-sym arg-syms]}] (conj (set arg-syms) this-sym))
            referenced (reduce into #{} (map (comp ast-referenced-names :body) methods))
            all-own (reduce into #{} (map own-names methods))
            outer-names (disj (set (keys env)) closure-depth-key)
            captures (vec (filter #(and (outer-names %) (not (all-own %))) referenced))]
        {:op :reify
         :interface iface
         :methods methods
         :captures captures}))))

;; `deftype` -- confirmed via `javap -p -c` on a host-AOT-compiled
;; `(deftype Point [x y])` with NO protocol/interface implementations: a
;; class with one PUBLIC FINAL field per declared field name and a
;; constructor storing each constructor arg into its field, nothing else
;; essential (real host also implements the marker interface
;; `clojure.lang.IType` and a static `getBasis` reflection helper, neither
;; of which affects observable field/construction behavior -- not
;; reproduced here, matching this file's established behavior-equivalence
;; bar). Deliberately narrow scope: field declarations ONLY, no protocol or
;; interface method implementations (that's a separate, larger extension
;; layering `reify`'s interface-implementing machinery onto a NAMED,
;; multi-field-constructor class) -- and, since `deftype` generates a
;; NAMED top-level class tied to a compilation unit rather than an
;; expression usable inline, it can only appear as one of the leading forms
;; of a top-level `(do (deftype ...)... (fn ...))` program -- see
;; `compile-source` -- never nested inside a `fn` body.
;; Splits the forms AFTER `deftype`'s field vector into interface/protocol
;; groups: every SYMBOL starts a new group, and every LIST immediately
;; following it (until the next symbol) is one of its method
;; implementations -- the same alternating shape real host's own `deftype`
;; syntax uses.
(defn- parse-deftype-impl-groups
  [forms]
  (loop [forms forms acc []]
    (if (empty? forms)
      acc
      (let [iface-sym (first forms)]
        (when-not (symbol? iface-sym)
          (throw (ex-info "tiny analyzer: expected an interface/protocol name in deftype"
                          {:form iface-sym})))
        (let [[methods rest-forms] (split-with seq? (rest forms))]
          (recur rest-forms (conj acc {:interface-sym iface-sym :method-forms (vec methods)})))))))

;; `deftype` may ALSO implement interfaces/protocols with method bodies --
;; confirmed via `javap -p -c` on a host-AOT-compiled `(deftype Rect [w h]
;; Shape (area [this] (* w h)))`: `Rect implements Shape, IType`, and
;; INSIDE `area()`, the declared fields `w`/`h` are read via a plain
;; `aload_0 (this); getfield w` -- i.e. exactly the same `this.fieldName`
;; shape this file's own closure/reify captures already use (`emit-local`'s
;; `:capture` case), just with the fields being EXPLICITLY DECLARED
;; (`[w h]`) rather than computed via free-variable analysis: a `deftype`
;; is a top-level definition with no enclosing lexical scope to capture
;; from at all (unlike `reify`, which is nested inside a `fn` and DOES
;; capture free variables), so its "captures" are simply always its own
;; field list, unconditionally, in every method. `analyze-reify-method` is
;; reused UNCHANGED for each method body (method/interface matching,
;; primitive-param rejection, body analysis are identical); only the
;; `env` passed to it differs -- deftype's own field names instead of an
;; enclosing scope.
(defn- analyze-deftype-form
  [form]
  (when-not (and (seq? form) (< 2 (count form))
                 (= 'deftype (first form))
                 (symbol? (second form))
                 (vector? (nth form 2))
                 (seq (nth form 2))
                 (every? symbol? (nth form 2))
                 (= (count (nth form 2)) (count (distinct (nth form 2)))))
    (throw (ex-info "tiny analyzer: malformed deftype -- expected (deftype Name [field...] Interface? (method [this args...] body...)?...)"
                    {:form form})))
  (let [fields (vec (nth form 2))
        field-env (zipmap fields (repeat true))
        impl-groups (parse-deftype-impl-groups (drop 3 form))
        impls (mapv (fn [{:keys [interface-sym method-forms]}]
                      (let [iface (or (get *known-protocol-interfaces* interface-sym)
                                      (when (not (namespace interface-sym))
                                        (try (Class/forName (name interface-sym)) (catch Throwable _ nil))))]
                        (when-not (and iface (.isInterface ^Class iface))
                          (throw (ex-info "tiny analyzer: deftype interface not found (must be a known protocol name or a fully-qualified interface name)"
                                          {:interface interface-sym})))
                        {:interface iface
                         :methods (mapv #(analyze-reify-method field-env iface %) method-forms)}))
                    impl-groups)]
    {:name (second form)
     :fields fields
     :impls impls}))

;; `defprotocol` -- confirmed via `javap -p` on a host-AOT-compiled
;; `(defprotocol Greet (greet [this]))`: real host generates a public
;; interface with one abstract method per protocol method (`this`
;; excluded from the interface signature -- it's the receiver, not an
;; explicit param), always Object-typed since a protocol has no
;; pre-existing Java type to reflect against (unlike `reify`'s target
;; interface). Also real host generates per-method dispatch FUNCTIONS
;; (Var-bound, so protocol methods work as ordinary first-class functions)
;; whose FULL mechanism falls back to `clojure.core/-cache-protocol-fn`
;; for values that don't directly implement the generated interface -- a
;; real, separately-sizeable reimplementation of a chunk of Clojure's
;; protocol runtime (`MethodImplCache`, `extend-protocol` registration),
;; not attempted here. This witness instead treats a protocol method call
;; as a fixed SPECIAL FORM at each call site (`known-protocol-method`)
;; compiling to the exact fast-path shape real host itself uses when it
;; CAN prove the value satisfies the interface directly (`checkcast
;; Interface; invokeinterface`) -- not a first-class Var-bound function
;; value. Like `deftype`, this generates a NAMED top-level interface tied
;; to the whole compile unit, so `defprotocol` can only appear as one of
;; the leading forms of a top-level `(do (deftype/defprotocol ...)...
;; (fn ...))` program -- see `compile-source`.
(defn- analyze-defprotocol-form
  [form]
  (when-not (and (seq? form) (= 'defprotocol (first form))
                 (< 2 (count form))
                 (symbol? (second form)))
    (throw (ex-info "tiny analyzer: malformed defprotocol -- expected (defprotocol Name (method [this args...]) ...)"
                    {:form form})))
  (let [pname (second form)
        method-forms (drop 2 form)]
    (when-not (every? #(and (seq? %) (symbol? (first %))
                            (vector? (second %)) (seq (second %)))
                      method-forms)
      (throw (ex-info "tiny analyzer: malformed defprotocol method signature -- expected (method [this args...])"
                      {:form form})))
    (let [method-names (map (comp name first) method-forms)]
      (when-not (= (count method-names) (count (distinct method-names)))
        (throw (ex-info "tiny analyzer: duplicate defprotocol method name" {:form form}))))
    {:name pname
     :methods (mapv (fn [[mname params]] {:name (name mname) :arity (dec (count params))})
                    method-forms)}))

(defn- analyze-fn
  [form]
  (when-not (and (seq? form) (= 'fn (first form)))
    (throw (ex-info "tiny analyzer: expected fn form" {:form form})))
  (let [rest-form (rest form)
        ;; Optional self-reference name: `(fn name [x] ...)` or
        ;; `(fn name ([x] ..) ([x y] ..))` -- confirmed via `javap -c` that
        ;; real host compiles a reference to this name WITHIN the body as a
        ;; plain `this` load (`aload_0`) checkCast to `IFn` and invoked,
        ;; the exact same shape `emit-local-fn-call` already uses for a
        ;; fn-valued local (unsurprising: the compiled class always
        ;; implements `IFn` via `AFunction`/`RestFn` regardless of naming).
        ;; So this only needs a new `:self` local-env kind, no new
        ;; bytecode mechanism.
        fn-name (when (symbol? (first rest-form)) (first rest-form))
        rest-form (if fn-name (rest rest-form) rest-form)
        ;; `(fn ([x] ..) ([x y] ..))`: every clause after `fn` is itself a
        ;; list. `(fn [x] ..)`: the single clause IS `rest-form` (its head is
        ;; a vector, not a list), so it is wrapped as the sole clause below.
        multi-arity? (and (seq rest-form) (seq? (first rest-form)))
        clauses (if multi-arity? rest-form [rest-form])
        arities (mapv #(analyze-fn-clause % fn-name {}) clauses)
        ;; Only compared among FIXED clauses: a fixed clause sharing its
        ;; param count with the (separately checked, at-most-one) variadic
        ;; clause is not a duplicate at all -- confirmed live, `(fn ([a b]
        ;; :fixed) ([a b & r] :variadic))` compiles fine on real host, and
        ;; the fixed clause's own `invoke(N)` override simply takes
        ;; precedence for that exact arity (see below).
        param-counts (map (comp count :params) (remove :rest-param arities))]
    (when-not (seq arities)
      (throw (ex-info "tiny analyzer: fn needs at least one arity" {:form form})))
    (when-not (= (count param-counts) (count (distinct param-counts)))
      (throw (ex-info "tiny analyzer: duplicate fn arity" {:form form})))
    ;; At most one variadic clause per `fn` -- `clojure.lang.RestFn` only
    ;; supports one `doInvoke`/one `getRequiredArity` ceiling, and real host
    ;; itself rejects a second `&`-clause the same way (a plain duplicate
    ;; arity, since two variadic clauses would both need the same fixed
    ;; param count to even be distinguishable).
    (let [variadic-arities (filter :rest-param arities)]
      (when (< 1 (count variadic-arities))
        (throw (ex-info "tiny analyzer: fn cannot have more than one variadic clause"
                        {:form form})))
      ;; Real host rejects a fixed clause with MORE params than the variadic
      ;; clause's own fixed-param count -- confirmed live: `(fn ([a b c] 1)
      ;; ([a & r] 2))` throws "Can't have fixed arity function with more
      ;; params than variadic function". A fixed clause with EQUAL param
      ;; count is fine and takes precedence over the variadic clause for
      ;; that exact arity (confirmed live: `(f 1 2)` picks the `([a b] ...)`
      ;; clause, not `([a b & r] ...)`, when both exist) -- this falls out
      ;; for free from emitting a direct `invoke(N)` override for every
      ;; fixed clause, same as the pure-fixed-multi-arity path already does:
      ;; the JVM dispatches to the most-derived override, no extra runtime
      ;; logic needed.
      (when-let [{variadic-params :params} (first variadic-arities)]
        (when (some #(> (count (:params %)) (count variadic-params))
                    (remove :rest-param arities))
          (throw (ex-info "tiny analyzer: fixed arity cannot have more params than the variadic arity"
                          {:form form})))))
    {:op :fn
     :fn-name fn-name
     :arities arities}))

(declare emit-expr)
(declare emit-closure)
(declare emit-reify)
(declare emit-deftype-new)
(declare emit-protocol-call)

(defn- emit-nil
  [^GeneratorAdapter ga]
  (.visitInsn ga Opcodes/ACONST_NULL))

(defn- emit-const
  [^GeneratorAdapter ga value]
  (cond
    (nil? value)
    (emit-nil ga)

    ;; Checked BEFORE the generic `integer?` branch below: `BigInt`
    ;; satisfies `integer?` too, and truncating one through that branch's
    ;; `(long value)` would silently lose precision for anything past
    ;; Long/MAX_VALUE. Real host does NOT construct these via
    ;; `BigInt.fromBigInteger(new BigInteger(String))` at all -- confirmed
    ;; via `javap -c` that it instead stores the literal's own source text
    ;; as a string constant and calls `clojure.lang.RT.readString` on it
    ;; once, at class-init time, caching the result in a static field. That
    ;; mechanism is deliberately NOT reproduced here: it would route this
    ;; witness's own constant construction through the real reader,
    ;; undermining the entire point of an independent DDC witness. Building
    ;; the value directly via the standard-library APIs (same VALUE,
    ;; genuinely different construction path) is the right choice for what
    ;; this backend is for, even though it diverges from real host's own
    ;; bytecode shape here. Also worth recording: a first draft built a
    ;; plain `java.math.BigInteger` instead of `clojure.lang.BigInt` --
    ;; matched by VALUE (Clojure's `=` compares across numeric types) but
    ;; NOT by class (`(class 5N)` is `clojure.lang.BigInt`, confirmed
    ;; live), a real gap caught by explicitly comparing classes, not just
    ;; values, before trusting the fixture.
    (instance? BigInt value)
    (do
      (.newInstance ga java-biginteger-type)
      (.dup ga)
      (.push ga (.toString (.toBigInteger ^BigInt value)))
      (.invokeConstructor ga java-biginteger-type string-arg-ctor-method)
      (.invokeStatic ga clj-bigint-type bigint-frombiginteger-method))

    (instance? BigDecimal value)
    (do
      (.newInstance ga bigdec-type)
      (.dup ga)
      (.push ga (.toString ^BigDecimal value))
      (.invokeConstructor ga bigdec-type string-arg-ctor-method))

    ;; Regex literals: real host's own bytecode (confirmed via `javap -c`)
    ;; is a direct `ldc "pattern"; invokestatic Pattern.compile(String)` --
    ;; no reader dependency at all, so this is reproduced exactly as-is.
    (instance? Pattern value)
    (do
      (.push ga (.pattern ^Pattern value))
      (.invokeStatic ga pattern-type pattern-compile-method))

    ;; Ratios are ALREADY reduced by the time they reach here (`parse-atom`
    ;; reduces via `Numbers/divide` at parse time, collapsing evenly
    ;; divisible cases to plain `Long`, exactly mirroring real host reader
    ;; semantics -- confirmed live). So construction here is a direct,
    ;; non-reducing `new Ratio(BigInteger, BigInteger)` from the
    ;; already-reduced numerator/denominator, deliberately NOT via
    ;; `RT/readString` (real host's actual bytecode mechanism for these),
    ;; matching the same independence principle as the BigInt/BigDecimal
    ;; branches above.
    (instance? Ratio value)
    (do
      (.newInstance ga ratio-type)
      (.dup ga)
      (.newInstance ga java-biginteger-type)
      (.dup ga)
      (.push ga (.toString ^BigInteger (.numerator ^Ratio value)))
      (.invokeConstructor ga java-biginteger-type string-arg-ctor-method)
      (.newInstance ga java-biginteger-type)
      (.dup ga)
      (.push ga (.toString ^BigInteger (.denominator ^Ratio value)))
      (.invokeConstructor ga java-biginteger-type string-arg-ctor-method)
      (.invokeConstructor ga ratio-type ratio-ctor-method))

    (integer? value)
    (do
      (.push ga (long value))
      (.box ga Type/LONG_TYPE))

    (boolean? value)
    ;; `GeneratorAdapter/box` for a boolean emits `new Boolean(z)` (the
    ;; deprecated constructor), NOT `Boolean.valueOf(z)` -- confirmed by
    ;; disassembling this witness's own output before this fix: `new
    ;; java/lang/Boolean; ...; invokespecial <init>:(Z)V`. That produces a
    ;; FRESH, non-singleton Boolean instance every time, which is NOT
    ;; `identical?` to `Boolean.FALSE`/`Boolean.TRUE`. This witness's own
    ;; `if` (`RT.booleanCast`, a real `instanceof`+`.booleanValue()`
    ;; conversion) tolerates that fine -- but real host's OWN compiled
    ;; `if` (confirmed via `javap -c` on a host-AOT-compiled `(if x ..)`)
    ;; does NOT call `RT.booleanCast` at all: it's `dup; ifnull ...;
    ;; getstatic Boolean.FALSE; if_acmpeq ...` -- a raw REFERENCE-IDENTITY
    ;; check against the singleton, for speed. A boolean built via `new
    ;; Boolean(false)` is therefore silently TRUTHY to any real-host code
    ;; that consumes it (found via `clojure.core/filter`, whose own
    ;; compiled `(if (pred f) ..)` kept every element regardless of what
    ;; a witness-emitted `<`/`>`/`=`/`zero?`/etc. predicate actually
    ;; returned -- `map`/direct calls never surfaced it, since neither
    ;; does an identity-based truthiness check). Fixed everywhere this
    ;; witness produces a boolean by using `GeneratorAdapter/valueOf`
    ;; instead of `box`, confirmed via a raw ASM probe to emit
    ;; `invokestatic Boolean.valueOf:(Z)Ljava/lang/Boolean;` -- the exact
    ;; singleton-returning call real host itself uses.
    (do
      (.push ga (boolean value))
      (.valueOf ga Type/BOOLEAN_TYPE))

    (string? value)
    (.push ga ^String value)

    (keyword? value)
    (do
      (.push ga (subs (str value) 1))
      (.invokeStatic ga keyword-type keyword-intern-method))

    (symbol? value)
    (do
      (.push ga (str value))
      (.invokeStatic ga symbol-type symbol-intern-method))

    :else
    (throw (ex-info "tiny emitter: unsupported const" {:value value}))))

(defn- emit-binary
  [^GeneratorAdapter ga env {:keys [fn lhs rhs]}]
  (emit-expr ga env lhs)
  (emit-expr ga env rhs)
  (case fn
    + (.invokeStatic ga numbers-type numbers-add-method)
    - (.invokeStatic ga numbers-type numbers-minus-method)
    * (.invokeStatic ga numbers-type numbers-multiply-method)
    quot (.invokeStatic ga numbers-type numbers-quotient-method)
    rem (.invokeStatic ga numbers-type numbers-remainder-method)
    get (.invokeStatic ga rt-type rt-get-method)
    < (do
        (.invokeStatic ga numbers-type numbers-lt-method)
        (.valueOf ga Type/BOOLEAN_TYPE))
    > (do
        (.invokeStatic ga numbers-type numbers-gt-method)
        (.valueOf ga Type/BOOLEAN_TYPE))
    >= (do
         (.invokeStatic ga numbers-type numbers-gte-method)
         (.valueOf ga Type/BOOLEAN_TYPE))
    <= (do
         (.invokeStatic ga numbers-type numbers-lte-method)
         (.valueOf ga Type/BOOLEAN_TYPE))
    = (do
        (.invokeStatic ga util-type util-equiv-method)
        (.valueOf ga Type/BOOLEAN_TYPE))))

(defn- emit-get3
  [^GeneratorAdapter ga env {:keys [map key default]}]
  (emit-expr ga env map)
  (emit-expr ga env key)
  (emit-expr ga env default)
  (.invokeStatic ga rt-type rt-get3-method))

(defn- emit-unary
  [^GeneratorAdapter ga env {:keys [fn arg]}]
  (emit-expr ga env arg)
  (case fn
    - (.invokeStatic ga numbers-type numbers-minus-unary-method)
    inc (.invokeStatic ga numbers-type numbers-inc-method)
    dec (.invokeStatic ga numbers-type numbers-dec-method)
    first (.invokeStatic ga rt-type rt-first-method)
    next (.invokeStatic ga rt-type rt-next-method)
    zero? (do
            (.invokeStatic ga numbers-type numbers-iszero-method)
            (.valueOf ga Type/BOOLEAN_TYPE))
    pos? (do
           (.invokeStatic ga numbers-type numbers-ispos-method)
           (.valueOf ga Type/BOOLEAN_TYPE))
    neg? (do
           (.invokeStatic ga numbers-type numbers-isneg-method)
           (.valueOf ga Type/BOOLEAN_TYPE))
    ;; RT.count returns primitive int; real host boxes this via
    ;; Integer/valueOf (confirmed by disassembling a real host-compiled
    ;; `(count coll)` -- Clojure's `count` returns a boxed Integer, not
    ;; Long, unlike every other numeric op in this file). GeneratorAdapter's
    ;; `.box` picks the matching valueOf automatically from the primitive
    ;; Type, same mechanism already used for the boolean ops above.
    count (do
            (.invokeStatic ga rt-type rt-count-method)
            (.box ga Type/INT_TYPE))))

(defn- emit-local
  [^GeneratorAdapter ga env name]
  (let [entry (get env name)]
    (case (:kind entry)
      :arg (.loadArg ga (int (:index entry)))
      :let (.loadLocal ga (int (:slot entry)))
      ;; A `(fn name [...] ...)` self-reference -- confirmed via `javap -c`
      ;; that real host compiles this as a plain `this` load, since the
      ;; compiled class already implements `IFn` via `AFunction`/`RestFn`
      ;; regardless of naming.
      :self (.loadThis ga)
      ;; A closure's captured free variable -- confirmed via `javap -c` on
      ;; a host-AOT-compiled `(fn [x] (fn [y] (+ x y)))`: the inner class
      ;; stores each captured name as an instance field, read via a plain
      ;; `this`-load + `getfield` (no different from any other field
      ;; access this witness already does for `.-fieldName`, just always
      ;; targeting the compiled class's OWN field rather than an arbitrary
      ;; receiver expression's).
      :capture (do (.loadThis ga)
                   (.getField ga ^Type (:owner entry) ^String (:field-name entry) obj-type))
      (throw (ex-info "tiny emitter: unknown local" {:name name})))))

(defn- emit-do
  [^GeneratorAdapter ga env exprs]
  (if (seq exprs)
    (do
      (doseq [expr (butlast exprs)]
        (emit-expr ga env expr)
        (.pop ga))
      (emit-expr ga env (last exprs)))
    (emit-nil ga)))

(defn- emit-let
  [^GeneratorAdapter ga env {:keys [bindings body]}]
  (loop [bindings bindings
         env env]
    (if (seq bindings)
      (let [{:keys [name init]} (first bindings)
            slot (.newLocal ga obj-type)]
        (emit-expr ga env init)
        (.storeLocal ga slot)
        (recur (rest bindings)
               (assoc env name {:kind :let :slot slot})))
      (emit-expr ga env body))))

(defn- emit-try
  "`visitTryCatchBlock` is called LAST, after `body` (and any try/catch or
  try/finally nested inside it) has been fully emitted -- not first. The JVM
  searches a method's exception table in call order and uses the first
  matching entry; registering this handler before emitting `body` would put
  this (outer) entry ahead of any nested handler's own entry in that table,
  so a nested `try` protecting an overlapping PC range would never be
  reached for a matching exception type (confirmed as a real, live bug:
  nesting a `try/finally` inside this construct's `body` silently skipped
  the inner `finally` before this ordering fix -- caught by testing the
  nested shape against real host `eval`, which does run the inner finally,
  not by inspection alone). Calling `visitTryCatchBlock` after emission is
  valid ASM usage: the call only needs to precede `endMethod`, and Label
  positions are already fixed by their `mark` calls regardless of when the
  referencing `visitTryCatchBlock` call happens."
  [^GeneratorAdapter ga env {:keys [body catch-class catch-name catch-body]}]
  (let [start (Label.)
        end (Label.)
        handler (Label.)
        after (Label.)
        catch-slot (.newLocal ga obj-type)]
    (.mark ga start)
    (emit-expr ga env body)
    (.mark ga end)
    (.goTo ga after)
    (.mark ga handler)
    (.storeLocal ga catch-slot)
    (emit-expr ga (assoc env catch-name {:kind :let :slot catch-slot}) catch-body)
    (.mark ga after)
    (.visitTryCatchBlock ga start end handler (Type/getInternalName catch-class))))

(defn- emit-try-finally
  "The real host duplicates the finally block on both exit paths (confirmed
  via `javap -c` on a host-AOT-compiled `(try a (finally (.incrementAndGet
  side)))`) rather than using a single shared subroutine -- this matches
  that exact shape, not an approximation: normal completion runs the body,
  stores its result, then runs `finally` and discards its value (`finally`
  never contributes to the `try`'s own result, confirmed live: `(try 42
  (finally 99))` -> `42`); a catch-all handler (`visitTryCatchBlock` with a
  `nil` type, matching real host's own `type=any` exception-table entry)
  runs `finally` again then re-throws the original exception unchanged.
  `visitTryCatchBlock` is called LAST for the same reason `emit-try` now
  does: the JVM exception table is search-order-sensitive, so an outer
  handler registered before a nested one would shadow it for any
  overlapping, matching PC range -- see `emit-try`'s docstring for the full
  explanation (this was a real bug, found by testing a `try/catch` nested
  around this construct against real host `eval`)."
  [^GeneratorAdapter ga env {:keys [body finally-body]}]
  (let [start (Label.)
        end (Label.)
        handler (Label.)
        after (Label.)
        result-slot (.newLocal ga obj-type)
        exception-slot (.newLocal ga obj-type)]
    (.mark ga start)
    (emit-expr ga env body)
    (.storeLocal ga result-slot)
    (.mark ga end)
    (emit-expr ga env finally-body)
    (.pop ga)
    (.goTo ga after)
    (.mark ga handler)
    (.storeLocal ga exception-slot)
    (emit-expr ga env finally-body)
    (.pop ga)
    (.loadLocal ga exception-slot)
    (.throwException ga)
    (.mark ga after)
    (.loadLocal ga result-slot)
    (.visitTryCatchBlock ga start end handler nil)))

(defn- emit-try-catch-finally
  "Combines the two shapes above: `finally` is duplicated on THREE exit
  paths -- normal body completion, successful `catch` completion, and any
  exception not fully handled by either (including one thrown from inside
  `catch-body` itself) -- matching the real host's own shape exactly,
  reverse-engineered via `javap -c -v` on a host-AOT-compiled `(try (quot 10
  x) (catch ArithmeticException e :divzero) (finally (.incrementAndGet
  a)))` before writing any code. The exception table needs THREE entries:
  [try-start,try-end) -> catch-start for `catch-class` specifically,
  [try-start,try-end) -> any-handler as a catch-all (so an exception NOT
  matching `catch-class` still runs `finally` before propagating), and
  [catch-start,catch-end) -> any-handler as a catch-all (so an exception
  thrown from `catch-body` itself still runs `finally` before propagating).
  Registration order matters twice over here (see `emit-try`'s docstring for
  the general reason): the two [try-start,try-end) entries must be
  registered with the specific `catch-class` BEFORE the any-handler (else
  `catch-class` would never be reached, matching the confirmed real bug
  pattern from the try/catch widening), and -- like every try* emitter in
  this file -- registration overall happens last, after body/catch-body/
  finally-body are fully emitted, so anything nested inside THIS construct
  registers first."
  [^GeneratorAdapter ga env {:keys [body catch-class catch-name catch-body finally-body]}]
  (let [try-start (Label.)
        try-end (Label.)
        catch-start (Label.)
        catch-end (Label.)
        any-handler (Label.)
        after (Label.)
        result-slot (.newLocal ga obj-type)
        catch-exception-slot (.newLocal ga obj-type)
        any-exception-slot (.newLocal ga obj-type)]
    (.mark ga try-start)
    (emit-expr ga env body)
    (.storeLocal ga result-slot)
    (.mark ga try-end)
    (emit-expr ga env finally-body)
    (.pop ga)
    (.goTo ga after)

    (.mark ga catch-start)
    (.storeLocal ga catch-exception-slot)
    (emit-expr ga (assoc env catch-name {:kind :let :slot catch-exception-slot}) catch-body)
    (.storeLocal ga result-slot)
    (.mark ga catch-end)
    (emit-expr ga env finally-body)
    (.pop ga)
    (.goTo ga after)

    (.mark ga any-handler)
    (.storeLocal ga any-exception-slot)
    (emit-expr ga env finally-body)
    (.pop ga)
    (.loadLocal ga any-exception-slot)
    (.throwException ga)

    (.mark ga after)
    (.loadLocal ga result-slot)

    (.visitTryCatchBlock ga try-start try-end catch-start (Type/getInternalName catch-class))
    (.visitTryCatchBlock ga try-start try-end any-handler nil)
    (.visitTryCatchBlock ga catch-start catch-end any-handler nil)))

(defn- emit-locking
  "Structurally identical to `emit-try-finally` -- reverse-engineered via
  `javap -c -v` on a host-AOT-compiled `(locking sb (.append sb \"x\"))`
  before writing any code -- except the `finally`-equivalent is always
  exactly `MONITOREXIT` on the lock object rather than an arbitrary
  expression, and `MONITORENTER` runs once, right after the lock
  expression is evaluated and stored, before the protected region begins.
  Real host also pushes and immediately pops an `ACONST_NULL` after each
  `MONITORENTER`/`MONITOREXIT` (representing `monitor-enter`/`monitor-exit`
  as statement-position expressions that evaluate to `nil` in Clojure's own
  general expression-oriented compiler) -- that push/pop pair is a
  cosmetic artifact of that general mechanism with no observable effect
  here, so it is not reproduced."
  [^GeneratorAdapter ga env {:keys [lock body]}]
  (let [start (Label.)
        end (Label.)
        handler (Label.)
        after (Label.)
        lock-slot (.newLocal ga obj-type)
        result-slot (.newLocal ga obj-type)
        exception-slot (.newLocal ga obj-type)]
    (emit-expr ga env lock)
    (.storeLocal ga lock-slot)
    (.loadLocal ga lock-slot)
    (.monitorEnter ga)

    (.mark ga start)
    (emit-expr ga env body)
    (.storeLocal ga result-slot)
    (.mark ga end)
    (.loadLocal ga lock-slot)
    (.monitorExit ga)
    (.goTo ga after)

    (.mark ga handler)
    (.storeLocal ga exception-slot)
    (.loadLocal ga lock-slot)
    (.monitorExit ga)
    (.loadLocal ga exception-slot)
    (.throwException ga)

    (.mark ga after)
    (.loadLocal ga result-slot)
    (.visitTryCatchBlock ga start end handler nil)))

(defn- invoke-method
  [arity]
  (Method. "invoke"
           obj-type
           (into-array Type (repeat arity obj-type))))

;; Pushes `RT.var(ns, name).getRawRoot()` cast to `IFn` -- the same
;; Var-lookup-then-invoke prelude `emit-core-fn-call` already uses for
;; `str`, generalized here to an arbitrary ns (not just `clojure.core`) so
;; `emit-binding` can reuse it for `push-thread-bindings`/
;; `pop-thread-bindings`/`hash-map` without duplicating `emit-core-fn-call`
;; itself.
(defn- emit-var-ifn
  [^GeneratorAdapter ga ^String var-ns ^String var-name]
  (.push ga var-ns)
  (.push ga var-name)
  (.invokeStatic ga rt-type rt-var-method)
  (.invokeVirtual ga var-type var-getrawroot-method)
  (.checkCast ga ifn-type))

(defn- emit-pop-thread-bindings
  [^GeneratorAdapter ga]
  (emit-var-ifn ga "clojure.core" "pop-thread-bindings")
  (.invokeInterface ga ifn-type (invoke-method 0))
  (.pop ga))

;; A bare `clojure.core` fn symbol used as a VALUE (not called), e.g. `inc`
;; in `(map inc coll)` -- confirmed via `javap -c` that real host resolves
;; this identically to a call head, `RT.var(ns,name).getRawRoot()` cast to
;; `IFn`, just WITHOUT the trailing `.invoke(...)` a call site adds.
(defn- emit-core-fn-value
  [^GeneratorAdapter ga fn-name]
  (emit-var-ifn ga "clojure.core" fn-name))

;; `binding` -- confirmed via `javap -c -v` on a host-AOT-compiled
;; `(binding [*x* 42] *x*)`: real host builds a one-entry map
;; (`clojure.core/hash-map` called via the same generic Var-call mechanism
;; as `str`) from the target Var (via `RT.var(ns,name)`, NOT `.getRawRoot`
;; -- the map key is the Var object itself) and the new value, calls
;; `clojure.core/push-thread-bindings` on that map, then runs the body in a
;; try/finally structurally identical to `emit-locking`'s lock-acquire/
;; monitor-exit shape except the \"lock\"/\"unlock\" pair is
;; `push-thread-bindings`/`pop-thread-bindings` rather than
;; MONITORENTER/MONITOREXIT. Reproduced exactly, not just behaviorally --
;; this one happens to match real host's bytecode shape as well, since
;; nothing here needed the reflective-fallback substitutions earlier
;; interop slices required.
(defn- emit-binding
  [^GeneratorAdapter ga env {:keys [var-ns var-name value body]}]
  (let [start (Label.)
        end (Label.)
        handler (Label.)
        after (Label.)
        result-slot (.newLocal ga obj-type)
        exception-slot (.newLocal ga obj-type)]
    (emit-var-ifn ga "clojure.core" "push-thread-bindings")
    (emit-var-ifn ga "clojure.core" "hash-map")
    (.push ga ^String var-ns)
    (.push ga ^String var-name)
    (.invokeStatic ga rt-type rt-var-method)
    (emit-expr ga env value)
    (.invokeInterface ga ifn-type (invoke-method 2))
    (.invokeInterface ga ifn-type (invoke-method 1))
    (.pop ga)

    (.mark ga start)
    (emit-expr ga env body)
    (.storeLocal ga result-slot)
    (.mark ga end)
    (emit-pop-thread-bindings ga)
    (.goTo ga after)

    (.mark ga handler)
    (.storeLocal ga exception-slot)
    (emit-pop-thread-bindings ga)
    (.loadLocal ga exception-slot)
    (.throwException ga)

    (.mark ga after)
    (.loadLocal ga result-slot)
    (.visitTryCatchBlock ga start end handler nil)))

(defn- emit-var-deref
  [^GeneratorAdapter ga {:keys [var-ns var-name]}]
  (.push ga ^String var-ns)
  (.push ga ^String var-name)
  (.invokeStatic ga rt-type rt-var-method)
  (.invokeVirtual ga var-type var-get-method))

(defn- emit-try-multi-catch
  "N catch clauses, no `finally` (combining multi-catch with `finally` is a
  separate, further slice). Reverse-engineered via `javap -c -v` on a
  host-AOT-compiled `(try (quot 10 x) (catch ArithmeticException e
  :divzero) (catch IllegalArgumentException e :bad-arg))` before writing
  any code: unlike `finally`'s catch-all `nil`-type entry, each catch
  clause here gets its own handler AND its own exception-table entry, all
  covering the SAME [try-start,try-end) range but with different
  (handler, specific class) pairs -- registered in source order, matching
  real host exactly. No catch-all entry is needed since there is no
  `finally` to guarantee here."
  [^GeneratorAdapter ga env {:keys [body catches]}]
  (let [try-start (Label.)
        try-end (Label.)
        after (Label.)
        result-slot (.newLocal ga obj-type)
        handlers (mapv (fn [_] (Label.)) catches)]
    (.mark ga try-start)
    (emit-expr ga env body)
    (.storeLocal ga result-slot)
    (.mark ga try-end)
    (.goTo ga after)
    (doseq [[{:keys [catch-name catch-body]} handler] (map vector catches handlers)]
      (.mark ga handler)
      (let [exception-slot (.newLocal ga obj-type)]
        (.storeLocal ga exception-slot)
        (emit-expr ga (assoc env catch-name {:kind :let :slot exception-slot}) catch-body))
      (.storeLocal ga result-slot)
      (.goTo ga after))
    (.mark ga after)
    (.loadLocal ga result-slot)
    (doseq [[{:keys [catch-class]} handler] (map vector catches handlers)]
      (.visitTryCatchBlock ga try-start try-end handler (Type/getInternalName catch-class)))))

(defn- emit-try-multi-catch-finally
  "The N-catch generalization of `emit-try-catch-finally` -- reverse-
  engineered via `javap -c -v` on a host-AOT-compiled `(try (quot 10 x)
  (catch ArithmeticException e :divzero) (catch IllegalArgumentException e
  :bad-arg) (finally (.incrementAndGet a)))` before writing any code. Each
  catch clause keeps its own specific-class exception-table entry over the
  try-body range (as in `emit-try-multi-catch`, source order), PLUS the
  try-body range gets ONE shared catch-all `finally` entry, PLUS each
  catch-body's OWN range independently gets a catch-all entry pointing to
  that SAME `finally` handler (so an exception thrown from inside any
  catch-body still runs `finally` before propagating). Registration order:
  each catch's specific entry before the shared any-handler entry for the
  SAME [try-start,try-end) range (the established ordering rule), then the
  per-catch-body any-handler entries (order among those doesn't matter,
  none of their ranges overlap each other or the try range)."
  [^GeneratorAdapter ga env {:keys [body catches finally-body]}]
  (let [try-start (Label.)
        try-end (Label.)
        any-handler (Label.)
        after (Label.)
        result-slot (.newLocal ga obj-type)
        any-exception-slot (.newLocal ga obj-type)
        catch-handlers (mapv (fn [_] (Label.)) catches)
        catch-ends (mapv (fn [_] (Label.)) catches)]
    (.mark ga try-start)
    (emit-expr ga env body)
    (.storeLocal ga result-slot)
    (.mark ga try-end)
    (emit-expr ga env finally-body)
    (.pop ga)
    (.goTo ga after)

    (doseq [[{:keys [catch-name catch-body]} handler end]
            (map vector catches catch-handlers catch-ends)]
      (.mark ga handler)
      (let [exception-slot (.newLocal ga obj-type)]
        (.storeLocal ga exception-slot)
        (emit-expr ga (assoc env catch-name {:kind :let :slot exception-slot}) catch-body))
      (.storeLocal ga result-slot)
      (.mark ga end)
      (emit-expr ga env finally-body)
      (.pop ga)
      (.goTo ga after))

    (.mark ga any-handler)
    (.storeLocal ga any-exception-slot)
    (emit-expr ga env finally-body)
    (.pop ga)
    (.loadLocal ga any-exception-slot)
    (.throwException ga)

    (.mark ga after)
    (.loadLocal ga result-slot)

    (doseq [[{:keys [catch-class]} handler] (map vector catches catch-handlers)]
      (.visitTryCatchBlock ga try-start try-end handler (Type/getInternalName catch-class)))
    (.visitTryCatchBlock ga try-start try-end any-handler nil)
    (doseq [[handler end] (map vector catch-handlers catch-ends)]
      (.visitTryCatchBlock ga handler end any-handler nil))))

(defn- emit-new
  [^GeneratorAdapter ga env {:keys [class arg]}]
  (let [ctype (Type/getType ^Class class)]
    (.newInstance ga ctype)
    (.dup ga)
    (if arg
      (do
        (emit-expr ga env arg)
        (.checkCast ga string-type)
        (.invokeConstructor ga ctype (Method. "<init>" Type/VOID_TYPE (into-array Type [string-type]))))
      (.invokeConstructor ga ctype init-method))))

(defn- emit-throw
  [^GeneratorAdapter ga env {:keys [expr]}]
  (emit-expr ga env expr)
  (.checkCast ga throwable-type)
  (.throwException ga))

(defn- emit-recur
  [^GeneratorAdapter ga env exprs]
  (let [{:keys [label slots]} (get env recur-target-key)]
    (when-not label
      (throw (ex-info "tiny emitter: recur outside loop" {})))
    (when-not (= (count slots) (count exprs))
      (throw (ex-info "tiny emitter: recur arity"
                      {:expected (count slots) :actual (count exprs)})))
    (let [temps (mapv (fn [expr]
                        (let [slot (.newLocal ga obj-type)]
                          (emit-expr ga env expr)
                          (.storeLocal ga slot)
                          slot))
                      exprs)]
      (doseq [[temp slot] (map vector temps slots)]
        (.loadLocal ga (int temp))
        ;; `loop` recur targets are fresh locals; a bare `recur` at a `fn`
        ;; body's own tail position (no enclosing `loop`) targets the
        ;; method's own argument slots instead -- confirmed via `javap -c`
        ;; that real host does exactly the same `astore`-into-arg-slot,
        ;; `GeneratorAdapter/storeArg` handles the "this" slot offset for
        ;; instance methods automatically.
        (case (:kind slot)
          :local (.storeLocal ga (int (:slot slot)))
          :arg (.storeArg ga (int (:index slot)))))
      (.goTo ga ^Label label))))

(defn- emit-loop
  [^GeneratorAdapter ga env {:keys [bindings body]}]
  (loop [bindings bindings
         env env
         slots []]
    (if (seq bindings)
      (let [{:keys [name init]} (first bindings)
            slot (.newLocal ga obj-type)]
        (emit-expr ga env init)
        (.storeLocal ga slot)
        (recur (rest bindings)
               (assoc env name {:kind :let :slot slot})
               (conj slots {:kind :local :slot slot})))
      (let [label (Label.)]
        (.mark ga label)
        (emit-expr ga
                   (assoc env recur-target-key {:label label :slots slots})
                   body)))))

(defn- emit-object-array
  [^GeneratorAdapter ga env items]
  (.push ga (int (count items)))
  (.newArray ga obj-type)
  (doseq [[i item] (map-indexed vector items)]
    (.dup ga)
    (.push ga (int i))
    (emit-expr ga env item)
    (.arrayStore ga obj-type)))

(defn- emit-interop-call
  [^GeneratorAdapter ga env {:keys [receiver method args]}]
  (emit-expr ga env receiver)
  (.push ga ^String method)
  (emit-object-array ga env args)
  (.invokeStatic ga reflector-type reflector-invoke-instance-method-method))

(defn- emit-static-interop-call
  [^GeneratorAdapter ga env {:keys [class method args]}]
  (.push ga (Type/getType ^Class class))
  (.push ga ^String method)
  (emit-object-array ga env args)
  (.invokeStatic ga reflector-type reflector-invoke-static-method-method))

(defn- emit-general-static-interop-call
  [^GeneratorAdapter ga env {:keys [class-name method args]}]
  (.push ga ^String class-name)
  (.invokeStatic ga rt-type rt-classfor-name-method)
  (.push ga ^String method)
  (emit-object-array ga env args)
  (.invokeStatic ga reflector-type reflector-invoke-static-method-method))

(defn- emit-general-new
  [^GeneratorAdapter ga env {:keys [class-name args]}]
  (.push ga ^String class-name)
  (.invokeStatic ga rt-type rt-classfor-name-method)
  (emit-object-array ga env args)
  (.invokeStatic ga reflector-type reflector-invoke-constructor-method))

;; `str` (and, in principle, any other `clojure.core` function -- this
;; mechanism generalizes, though only `str` is wired up as a fixture-tested
;; call head this pass) is NOT compiler-special on the real host at all:
;; confirmed via `javap -c` that `(str a b)` resolves the Var
;; `clojure.core/str` via `RT.var("clojure.core", "str")`, reads its
;; `getRawRoot()` (the live function value), casts to `IFn`, and calls
;; `.invoke(...)` -- the exact same mechanism ordinary user-defined function
;; calls use. This emitter matches that shape exactly, doing the Var lookup
;; inline on every call rather than caching it in a static field the way
;; real host does (a real host `const__N` field) -- a performance
;; difference only, not a behavior difference, since `Var.getRawRoot()`
;; returns the same live value either way.
(defn- emit-core-fn-call
  [^GeneratorAdapter ga env {:keys [fn-name args]}]
  (.push ga "clojure.core")
  (.push ga ^String fn-name)
  (.invokeStatic ga rt-type rt-var-method)
  (.invokeVirtual ga var-type var-getrawroot-method)
  (.checkCast ga ifn-type)
  (doseq [arg args] (emit-expr ga env arg))
  (.invokeInterface ga ifn-type (invoke-method (count args))))

;; A local (fn parameter/`let` binding) called as a fn, e.g. `(f x)` where
;; `f` is itself a parameter -- confirmed via `javap -c`: just the local's
;; value cast straight to `IFn` and invoked, no Var lookup at all.
(defn- emit-local-fn-call
  [^GeneratorAdapter ga env {:keys [name args]}]
  (emit-local ga env name)
  (.checkCast ga ifn-type)
  (doseq [arg args] (emit-expr ga env arg))
  (.invokeInterface ga ifn-type (invoke-method (count args))))

;; A call head that is itself an expression, not a bare symbol, e.g.
;; `((constantly x) 99)` -- confirmed via `javap -c`: whatever bytecode the
;; head expression produces, cast straight to `IFn` and invoked.
(defn- emit-computed-fn-call
  [^GeneratorAdapter ga env {:keys [fn-expr args]}]
  (emit-expr ga env fn-expr)
  (.checkCast ga ifn-type)
  (doseq [arg args] (emit-expr ga env arg))
  (.invokeInterface ga ifn-type (invoke-method (count args))))

;; `.-fieldName`/`set!` -- confirmed via `javap -c` on host-AOT-compiled
;; `(.-x p)` and `(set! (.-x p) v)` for an untyped `p`: field GET goes
;; through `Reflector.invokeNoArgInstanceMember(Object, String, boolean)`
;; with the boolean literal `true` (distinguishing "field access" from the
;; `false` a bare `.methodName` no-arg call uses, which also tries a
;; zero-arg method), and `set!` goes through
;; `Reflector.setInstanceField(Object, String, Object)`, which itself
;; returns the assigned value -- matching Clojure's own `set!` semantics of
;; evaluating to the value that was set.
(defn- emit-field-get
  [^GeneratorAdapter ga env {:keys [receiver field]}]
  (emit-expr ga env receiver)
  (.push ga ^String field)
  (.push ga true)
  (.invokeStatic ga reflector-type reflector-invoke-noarg-instance-member-method))

(defn- emit-field-set
  [^GeneratorAdapter ga env {:keys [receiver field value]}]
  (emit-expr ga env receiver)
  (.push ga ^String field)
  (emit-expr ga env value)
  (.invokeStatic ga reflector-type reflector-set-instance-field-method))

(defn- emit-vector
  [^GeneratorAdapter ga env items]
  (emit-object-array ga env items)
  (.invokeStatic ga rt-type rt-vector-method))

(defn- emit-map
  [^GeneratorAdapter ga env entries]
  (emit-object-array ga env
                     (mapcat (fn [{:keys [key val]}]
                               [key val])
                             entries))
  (.invokeStatic ga rt-type rt-map-method))

(defn- emit-set
  [^GeneratorAdapter ga env items]
  (emit-object-array ga env items)
  (.invokeStatic ga rt-type rt-set-method))

(defn- rt-list-method
  [arity]
  (reflect-asm-method RT "list" (repeat arity Object)))

(defn- emit-list
  [^GeneratorAdapter ga env items]
  (when (< 5 (count items))
    (throw (ex-info "tiny emitter: quoted list arity"
                    {:arity (count items)
                     :max-arity 5})))
  (doseq [item items]
    (emit-expr ga env item))
  (.invokeStatic ga rt-type (rt-list-method (count items))))

(defn- emit-expr
  [^GeneratorAdapter ga env node]
  (case (:op node)
    :nil (emit-nil ga)
    :const (emit-const ga (:value node))
    :local (emit-local ga env (:name node))
    :binary (emit-binary ga env node)
    :get3 (emit-get3 ga env node)
    :unary (emit-unary ga env node)
    :vector (emit-vector ga env (:items node))
    :map (emit-map ga env (:entries node))
    :set (emit-set ga env (:items node))
    :list (emit-list ga env (:items node))
    :do (emit-do ga env (:exprs node))
    :let (emit-let ga env node)
    :try (emit-try ga env node)
    :try-finally (emit-try-finally ga env node)
    :try-catch-finally (emit-try-catch-finally ga env node)
    :locking (emit-locking ga env node)
    :binding (emit-binding ga env node)
    :var-deref (emit-var-deref ga node)
    :try-multi-catch (emit-try-multi-catch ga env node)
    :try-multi-catch-finally (emit-try-multi-catch-finally ga env node)
    :new (emit-new ga env node)
    :general-new (emit-general-new ga env node)
    :throw (emit-throw ga env node)
    :interop-call (emit-interop-call ga env node)
    :static-interop-call (emit-static-interop-call ga env node)
    :general-static-interop-call (emit-general-static-interop-call ga env node)
    :core-fn-call (emit-core-fn-call ga env node)
    :local-fn-call (emit-local-fn-call ga env node)
    :closure (emit-closure ga env node)
    :reify (emit-reify ga env node)
    :deftype-new (emit-deftype-new ga env node)
    :protocol-call (emit-protocol-call ga env node)
    :computed-fn-call (emit-computed-fn-call ga env node)
    :core-fn-value (emit-core-fn-value ga (:fn-name node))
    :field-get (emit-field-get ga env node)
    :field-set (emit-field-set ga env node)
    :loop (emit-loop ga env node)
    :recur (emit-recur ga env (:exprs node))
    :if (let [else-label (Label.)
              end-label (Label.)]
          (emit-expr ga env (:test node))
          (.invokeStatic ga rt-type rt-booleancast-method)
          (.ifZCmp ga GeneratorAdapter/EQ else-label)
          (emit-expr ga env (:then node))
          (.goTo ga end-label)
          (.mark ga else-label)
          (emit-expr ga env (:else node))
          (.mark ga end-label))))

(defn- do-invoke-method
  [arity]
  (Method. "doInvoke"
           obj-type
           (into-array Type (repeat arity obj-type))))

(def ^:private get-required-arity-method
  (Method. "getRequiredArity" Type/INT_TYPE (into-array Type [])))

(defn- emit-class
  "`captures` (empty for a top-level `compile-source` fn) names a closure's
  free variables -- confirmed via `javap -c` on a host-AOT-compiled
  `(fn [x] (fn [y] (+ x y)))`: the inner class gets one instance field per
  captured name, a constructor taking their values (in `captures` order)
  instead of the usual no-arg one, and every reference to a captured name
  inside the class's own methods reads the field instead of an arg/local
  slot -- see `emit-local`'s `:capture` case."
  [{:keys [arities fn-name captures]}]
  (let [variadic? (boolean (some :rest-param arities))
        class-name (next-class-name)
        internal (.replace class-name \. \/)
        ctype (Type/getObjectType internal)
        base-type (if variadic? restfn-type afn-type)
        cw (ClassWriter. (bit-or ClassWriter/COMPUTE_FRAMES
                                 ClassWriter/COMPUTE_MAXS))
        capture-env (into {}
                          (map (fn [cap]
                                 [cap {:kind :capture :owner ctype :field-name (name cap)}]))
                          captures)
        self-env (cond-> capture-env
                   fn-name (assoc fn-name {:kind :self}))]
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC internal nil
            (.getInternalName base-type) nil)
    (doseq [cap captures]
      (.visitEnd (.visitField cw Opcodes/ACC_FINAL (name cap) (.getDescriptor obj-type) nil nil)))
    (if (seq captures)
      (let [ctor-method (Method. "<init>" Type/VOID_TYPE (into-array Type (repeat (count captures) obj-type)))
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC ctor-method nil nil cw)]
        (.loadThis ga)
        (.invokeConstructor ga base-type init-method)
        (doseq [[i cap] (map-indexed vector captures)]
          (.loadThis ga)
          (.loadArg ga (int i))
          (.putField ga ctype (name cap) obj-type))
        (.returnValue ga)
        (.endMethod ga))
      (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC init-method nil nil cw)]
        (.loadThis ga)
        (.invokeConstructor ga base-type init-method)
        (.returnValue ga)
        (.endMethod ga)))
    ;; One `invoke` method per FIXED (non-variadic) arity clause, same
    ;; class -- exactly how real host compiler emits fixed multi-arity
    ;; `fn`, whether or not a variadic clause is ALSO present in the same
    ;; `fn` (confirmed via `javap -c` on a mixed `(fn ([a] a) ([a b] (+ a
    ;; b)) ([a b & r] ...))`): AFunction/RestFn already expose
    ;; independently-overridable invoke0..invoke20, and IFn dispatch on the
    ;; *call side* picks the right one by argument count. When a fixed
    ;; clause's param count equals the variadic clause's own fixed-param
    ;; count, this override simply wins over RestFn's inherited
    ;; doInvoke-routing default for that exact arity -- confirmed live, see
    ;; `analyze-fn`'s validation above -- no extra runtime logic needed.
    (doseq [{:keys [params body]} (remove :rest-param arities)]
        (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC
                                    (invoke-method (count params))
                                    nil nil cw)
              recur-label (Label.)
              arg-env (into (or self-env {})
                            (map-indexed (fn [i p]
                                           [p {:kind :arg :index i}]))
                            params)
              recur-slots (mapv (fn [i] {:kind :arg :index i})
                                 (range (count params)))]
          (.mark ga recur-label)
          (emit-expr ga
                     (assoc arg-env recur-target-key
                            {:label recur-label :slots recur-slots})
                     body)
          (.returnValue ga)
          (.endMethod ga)))
    (when variadic?
      ;; The ONE variadic clause: RestFn already implements EVERY public
      ;; invoke(...) overload concretely (confirmed via `javap` on the real
      ;; clojure.jar) for arities >= getRequiredArity(), routing through
      ;; doInvoke -- a subclass only supplies the two overridable pieces:
      ;; getRequiredArity() and the ONE doInvoke overload whose param count
      ;; is (fixed-params + 1), the last slot being the collected rest
      ;; sequence (or nil if none were passed). This exact shape -- not
      ;; merely a similar one -- was reverse-engineered from real
      ;; host-AOT-compiled `(fn [a & r] r)`, `(fn [a b & r] r)`, and the
      ;; mixed fixed+variadic case above via `javap -c` before being
      ;; written here.
      (let [{:keys [params rest-param body]} (first (filter :rest-param arities))
            all-params (conj params rest-param)
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC
                                  (do-invoke-method (count all-params))
                                  nil nil cw)
            recur-label (Label.)
            arg-env (into (or self-env {})
                          (map-indexed (fn [i p]
                                         [p {:kind :arg :index i}]))
                          all-params)
            recur-slots (mapv (fn [i] {:kind :arg :index i})
                               (range (count all-params)))]
        (.mark ga recur-label)
        (emit-expr ga
                   (assoc arg-env recur-target-key
                          {:label recur-label :slots recur-slots})
                   body)
        (.returnValue ga)
        (.endMethod ga)
        (let [arity-ga (GeneratorAdapter. Opcodes/ACC_PUBLIC
                                          get-required-arity-method
                                          nil nil cw)]
          (.push arity-ga (int (count params)))
          (.returnValue arity-ga)
          (.endMethod arity-ga))))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)
          loader (DynamicClassLoader.
                  (.getContextClassLoader (Thread/currentThread)))
          klass (.defineClass loader class-name bytes nil)]
      {:class klass
       :class-name class-name
       :bytes bytes
       :digest (sha256-string (seq bytes))})))

;; A nested closure literal at its DEFINITION site, e.g. the `(fn [y] ...)`
;; inside `(fn [x] (fn [y] (+ x y)))` -- confirmed via `javap -c`: real host
;; just does `NEW innerClass; DUP; <push each captured var's CURRENT value>;
;; INVOKESPECIAL <init>`. The inner class itself is built by recursively
;; calling `emit-class` (a genuinely separate class, defined/loaded eagerly
;; here, before the outer method's own bytecode finishes -- the JVM has no
;; ordering requirement stronger than "loadable by the time it's actually
;; `NEW`-ed at runtime", which eager definition trivially satisfies).
;; Each captured value is pushed via the OUTER method's own `emit-local`
;; (its env already carries the right `:arg`/`:let`/`:self`/`:capture` kind
;; for each captured name, since captures are by construction names already
;; bound in the outer scope) -- reusing existing local-resolution machinery
;; rather than inventing a parallel one.
(defn- emit-closure
  [^GeneratorAdapter ga env {:keys [fn-name params rest-param body captures]}]
  (let [artifact (emit-class {:arities [{:params params :rest-param rest-param :body body}]
                              :fn-name fn-name
                              :captures captures})
        inner-type (Type/getObjectType (.replace ^String (:class-name artifact) \. \/))
        ctor-method (Method. "<init>" Type/VOID_TYPE (into-array Type (repeat (count captures) obj-type)))]
    (.newInstance ga inner-type)
    (.dup ga)
    (doseq [cap captures]
      (emit-local ga env cap))
    (.invokeConstructor ga inner-type ctor-method)))

;; After emitting a reify method body (which always produces a boxed
;; Object, exactly like every other emitted expression in this file), coerce
;; it to match the reified interface method's OWN declared return type --
;; `void` pops the unused value, a primitive return type unboxes it (safe:
;; params with primitive types are already rejected at analyze time, so
;; only the return side ever needs this), and a reference return type
;; (including plain `Object`) needs no coercion at all.
(defn- coerce-reify-return!
  [^GeneratorAdapter ga ^Class return-type]
  (cond
    (= Void/TYPE return-type) (.pop ga)
    (.isPrimitive return-type) (.unbox ga (Type/getType return-type))
    :else nil))

(defn- emit-reify-class
  [^Class interface methods captures]
  (let [class-name (next-class-name)
        internal (.replace class-name \. \/)
        ctype (Type/getObjectType internal)
        iface-type (Type/getType interface)
        cw (ClassWriter. (bit-or ClassWriter/COMPUTE_FRAMES
                                 ClassWriter/COMPUTE_MAXS))
        capture-env (into {}
                          (map (fn [cap]
                                 [cap {:kind :capture :owner ctype :field-name (name cap)}]))
                          captures)]
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC internal nil
            (.getInternalName obj-type) (into-array String [(.getInternalName iface-type)]))
    (doseq [cap captures]
      (.visitEnd (.visitField cw Opcodes/ACC_FINAL (name cap) (.getDescriptor obj-type) nil nil)))
    (if (seq captures)
      (let [ctor-method (Method. "<init>" Type/VOID_TYPE (into-array Type (repeat (count captures) obj-type)))
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC ctor-method nil nil cw)]
        (.loadThis ga)
        (.invokeConstructor ga obj-type init-method)
        (doseq [[i cap] (map-indexed vector captures)]
          (.loadThis ga)
          (.loadArg ga (int i))
          (.putField ga ctype (name cap) obj-type))
        (.returnValue ga)
        (.endMethod ga))
      (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC init-method nil nil cw)]
        (.loadThis ga)
        (.invokeConstructor ga obj-type init-method)
        (.returnValue ga)
        (.endMethod ga)))
    (doseq [{:keys [this-sym arg-syms reflected body]} methods]
      (let [^java.lang.reflect.Method rmethod reflected
            ret-type (Type/getType (.getReturnType rmethod))
            param-types (into-array Type (map #(Type/getType ^Class %) (.getParameterTypes rmethod)))
            asm-method (Method. (.getName rmethod) ret-type param-types)
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC asm-method nil nil cw)
            arg-env (into (assoc capture-env this-sym {:kind :self})
                          (map-indexed (fn [i p] [p {:kind :arg :index i}]))
                          arg-syms)]
        (emit-expr ga arg-env body)
        (coerce-reify-return! ga (.getReturnType rmethod))
        (.returnValue ga)
        (.endMethod ga)))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)
          loader (DynamicClassLoader.
                  (.getContextClassLoader (Thread/currentThread)))
          klass (.defineClass loader class-name bytes nil)]
      {:class klass
       :class-name class-name
       :bytes bytes
       :digest (sha256-string (seq bytes))})))

;; A `reify` expression at its DEFINITION site -- structurally identical to
;; `emit-closure`'s NEW/DUP/<captures>/INVOKESPECIAL, just building the
;; class via `emit-reify-class` instead of `emit-class`.
(defn- emit-reify
  [^GeneratorAdapter ga env {:keys [interface methods captures]}]
  (let [artifact (emit-reify-class interface methods captures)
        inner-type (Type/getObjectType (.replace ^String (:class-name artifact) \. \/))
        ctor-method (Method. "<init>" Type/VOID_TYPE (into-array Type (repeat (count captures) obj-type)))]
    (.newInstance ga inner-type)
    (.dup ga)
    (doseq [cap captures]
      (emit-local ga env cap))
    (.invokeConstructor ga inner-type ctor-method)))

;; The `deftype` class itself: a named-purpose class defined ONCE, up front,
;; by `compile-source`'s multi-form program path -- NOT recursively emitted
;; from within another method's body the way `emit-closure`/`emit-reify`
;; are, since `deftype` is a top-level program element, not an expression.
;; `:impls` (empty for a fields-only `deftype`) is a vector of `{:interface
;; :methods}` -- each method emitted exactly like `emit-reify-class`'s
;; (Method descriptor built from the REFLECTED param/return types, body
;; coerced to match a primitive return via `coerce-reify-return!`), just
;; with `field-env` (the deftype's own declared fields, ALWAYS present, no
;; free-variable computation needed -- see `analyze-deftype-form`) standing
;; in for what closures/`reify` call "captures".
(defn- emit-deftype-class
  [{:keys [fields impls]}]
  (let [class-name (next-class-name)
        internal (.replace class-name \. \/)
        ctype (Type/getObjectType internal)
        interfaces (into-array String (map #(.getInternalName (Type/getType ^Class (:interface %))) impls))
        cw (ClassWriter. (bit-or ClassWriter/COMPUTE_FRAMES
                                 ClassWriter/COMPUTE_MAXS))
        field-env (into {}
                        (map (fn [f]
                               [f {:kind :capture :owner ctype :field-name (name f)}]))
                        fields)]
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC internal nil
            (.getInternalName obj-type) interfaces)
    (doseq [f fields]
      (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_FINAL)
                              (name f) (.getDescriptor obj-type) nil nil)))
    (let [ctor-method (Method. "<init>" Type/VOID_TYPE (into-array Type (repeat (count fields) obj-type)))
          ga (GeneratorAdapter. Opcodes/ACC_PUBLIC ctor-method nil nil cw)]
      (.loadThis ga)
      (.invokeConstructor ga obj-type init-method)
      (doseq [[i f] (map-indexed vector fields)]
        (.loadThis ga)
        (.loadArg ga (int i))
        (.putField ga ctype (name f) obj-type))
      (.returnValue ga)
      (.endMethod ga))
    (doseq [{:keys [methods]} impls
            {:keys [this-sym arg-syms reflected body]} methods]
      (let [^java.lang.reflect.Method rmethod reflected
            ret-type (Type/getType (.getReturnType rmethod))
            param-types (into-array Type (map #(Type/getType ^Class %) (.getParameterTypes rmethod)))
            asm-method (Method. (.getName rmethod) ret-type param-types)
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC asm-method nil nil cw)
            arg-env (into (assoc field-env this-sym {:kind :self})
                          (map-indexed (fn [i p] [p {:kind :arg :index i}]))
                          arg-syms)]
        (emit-expr ga arg-env body)
        (coerce-reify-return! ga (.getReturnType rmethod))
        (.returnValue ga)
        (.endMethod ga)))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)
          loader (DynamicClassLoader.
                  (.getContextClassLoader (Thread/currentThread)))
          klass (.defineClass loader class-name bytes nil)]
      {:class klass
       :class-name class-name
       :bytes bytes
       :digest (sha256-string (seq bytes))})))

;; `(Name. args...)` for a `deftype` defined earlier in the SAME top-level
;; program -- direct `NEW/DUP/<args>/INVOKESPECIAL`, matching real host's
;; own bytecode shape for a compile-time-known type exactly (unlike the
;; small-allowlist/general-construction paths above, which both exist
;; specifically because THEY don't have a compile-time-resolved Class to
;; construct against).
(defn- emit-deftype-new
  [^GeneratorAdapter ga env {:keys [class args]}]
  (let [ctype (Type/getType ^Class class)
        ctor-method (Method. "<init>" Type/VOID_TYPE (into-array Type (repeat (count args) obj-type)))]
    (.newInstance ga ctype)
    (.dup ga)
    (doseq [arg args] (emit-expr ga env arg))
    (.invokeConstructor ga ctype ctor-method)))

;; The `defprotocol`-generated interface itself -- a plain abstract public
;; interface, one abstract method per protocol method, all Object-typed
;; (no pre-existing type to reflect against here, unlike `reify`'s
;; target). No method BODIES: `ACC_ABSTRACT` methods only declare a
;; descriptor, `visitMethod` returns a `MethodVisitor` that's simply ended
;; immediately with no code emitted.
(defn- emit-protocol-interface
  [{:keys [methods]}]
  (let [class-name (next-class-name)
        internal (.replace class-name \. \/)
        cw (ClassWriter. (bit-or ClassWriter/COMPUTE_FRAMES
                                 ClassWriter/COMPUTE_MAXS))]
    (.visit cw Opcodes/V1_8
            (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_INTERFACE Opcodes/ACC_ABSTRACT)
            internal nil (.getInternalName obj-type) nil)
    (doseq [{:keys [name arity]} methods]
      (let [asm-method (Method. name obj-type (into-array Type (repeat arity obj-type)))]
        (.visitEnd (.visitMethod cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_ABSTRACT)
                                 (.getName asm-method) (.getDescriptor asm-method) nil nil))))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)
          loader (DynamicClassLoader.
                  (.getContextClassLoader (Thread/currentThread)))
          klass (.defineClass loader class-name bytes nil)]
      {:class klass
       :class-name class-name
       :bytes bytes
       :digest (sha256-string (seq bytes))})))

;; `(methodName instance args...)` -- confirmed via `javap -c` on real
;; host's own FAST PATH for a protocol method call: `checkcast Interface;
;; invokeinterface method`, exactly this.
(defn- emit-protocol-call
  [^GeneratorAdapter ga env {:keys [interface method instance args]}]
  (let [iface-type (Type/getType ^Class interface)
        asm-method (Method. method obj-type (into-array Type (repeat (count args) obj-type)))]
    (emit-expr ga env instance)
    (.checkCast ga iface-type)
    (doseq [arg args] (emit-expr ga env arg))
    (.invokeInterface ga iface-type asm-method)))

;; `(do (deftype/defprotocol ...)... (fn ...))` -- the ONLY shape allowing
;; `deftype`/`defprotocol` at all, since both define a NAMED top-level
;; class/interface rather than an inline expression (see
;; `analyze-deftype-form`/`analyze-defprotocol-form`). Every form but the
;; last must be one of the two; the last must be the `fn` this compile
;; unit still ultimately returns -- deliberately narrow (no other
;; top-level forms, e.g. no top-level `def`, mixed in) to keep this a
;; small, explicit addition to `compile-source` rather than a general
;; multi-form program model.
(defn- top-level-program-form?
  [form]
  (and (seq? form) (= 'do (first form)) (< 1 (count form))
       (every? #(and (seq? %) (contains? #{'deftype 'defprotocol} (first %)))
               (butlast (rest form)))
       (seq? (last form)) (= 'fn (first (last form)))))

;; Emits each leading `deftype`/`defprotocol` form IN ORDER, folding up
;; the three registries `compile-source` binds around the trailing `fn`'s
;; own compilation (`*known-deftype-classes*`, `*known-protocol-methods*`,
;; `*known-protocol-interfaces*`) -- a plain sequential `reduce`, since a
;; later `deftype`/`defprotocol` in the same program never needs to see an
;; earlier one (no forward/mutual reference between top-level definitions
;; attempted here, matching this file's other narrow-scope-on-purpose
;; boundaries).
;; Plain (non-tail) recursion, NOT `reduce`/`loop`+`recur`: each leading
;; form is analyzed/emitted with the registries built from every form
;; BEFORE it already bound (so `(deftype Rect [w h] Shape ...)` can see a
;; `Shape` protocol declared earlier in the same program) -- `binding`
;; expands through a `try`/`finally`, and `recur` cannot cross a `try`
;; boundary, so a `loop`+`recur` version of this rebind-per-step pattern
;; would not compile; ordinary recursive calls have no such restriction
;; and this list is always small (a handful of leading declarations).
(defn- emit-leading-program-forms
  ([forms]
   (emit-leading-program-forms
    forms
    {:artifacts [] :deftype-classes {} :protocol-interfaces {} :protocol-methods {}}))
  ([forms acc]
   (if (empty? forms)
     acc
     (binding [*known-deftype-classes* (:deftype-classes acc)
               *known-protocol-interfaces* (:protocol-interfaces acc)
               *known-protocol-methods* (:protocol-methods acc)]
       (let [form (first forms)]
         (if (= 'deftype (first form))
           (let [spec (analyze-deftype-form form)
                 artifact (emit-deftype-class spec)]
             (emit-leading-program-forms
              (rest forms)
              (-> acc
                  (update :artifacts conj artifact)
                  (update :deftype-classes assoc (:name spec) (:class artifact)))))
           (let [spec (analyze-defprotocol-form form)
                 artifact (emit-protocol-interface spec)
                 iface (:class artifact)]
             (emit-leading-program-forms
              (rest forms)
              (-> acc
                  (update :artifacts conj artifact)
                  (update :protocol-interfaces assoc (:name spec) iface)
                  (update :protocol-methods into
                          (map (fn [m] [(symbol (:name m))
                                        {:interface iface :method (:name m) :arity (:arity m)}])
                               (:methods spec))))))))))))

(defn compile-source
  [source]
  (let [form (tiny-read source)
        expanded-form (tiny-expand form)]
    (if (top-level-program-form? expanded-form)
      (let [leading-forms (butlast (rest expanded-form))
            fn-form (last expanded-form)
            {:keys [artifacts deftype-classes protocol-interfaces protocol-methods]}
            (emit-leading-program-forms leading-forms)]
        (binding [*known-deftype-classes* deftype-classes
                  *known-protocol-interfaces* protocol-interfaces
                  *known-protocol-methods* protocol-methods]
          (let [ast (analyze-fn fn-form)
                artifact (emit-class ast)
                f (.newInstance ^Class (:class artifact))]
            {:source source
             :form form
             :expanded-form expanded-form
             :leading-artifacts artifacts
             :ast ast
             :artifact artifact
             :fn f})))
      (let [ast (analyze-fn expanded-form)
            artifact (emit-class ast)
            f (.newInstance ^Class (:class artifact))]
        {:source source
         :form form
         :expanded-form expanded-form
         :ast ast
         :artifact artifact
         :fn f}))))

(defn- case-row
  [{:keys [id source args expected]}]
  (try
    (let [{f :fn artifact :artifact ast :ast expanded-form :expanded-form}
          (compile-source source)
          got (apply f args)
          ok? (= expected got)]
      {:id id
       :kind :tiny-frontend-direct-emit
       :source source
       :expanded-form (pr-str expanded-form)
       :args args
       :expected expected
       :got got
       :ast ast
       :artifact (select-keys artifact [:class-name :digest])
       :frontend {:reader :pnix-tiny-reader
                  :analyzer :pnix-tiny-analyzer
                  :macroexpander :pnix-tiny-macroexpander
                  :macro-rules tiny-macro-rules
                  :uses-host-macroexpand false
                  :uses-tools-analyzer-jvm false
                  :uses-clojure-reader false}
       :gate/verdict (if ok? :accepted :rejected)
       :ok ok?})
    (catch Throwable t
      {:id id
       :kind :tiny-frontend-direct-emit
       :source source
       :gate/verdict :rejected
       :error {:class (.getName (class t))
               :message (.getMessage t)
               :data (ex-data t)}
       :ok false})))

(defn- specs
  []
  [{:id :tiny-const-arithmetic
    :source "(fn [] (* (+ 6 1) 6))"
    :args []
    :expected 42}
   {:id :tiny-one-arg
    :source "(fn [x] (+ x 1))"
    :args [41]
    :expected 42}
   {:id :tiny-branch-two-arg
    :source "(fn [x y] (if (< x y) (* (+ x 1) y) (- x y)))"
    :args [5 7]
    :expected 42}
   {:id :tiny-do-body
    :source "(fn [] (do (+ 1 2) (* 6 7)))"
    :args []
    :expected 42}
   {:id :tiny-let-sequential
    :source "(fn [x] (let [a (+ x 1) b (* a 2)] b))"
    :args [20]
    :expected 42}
   {:id :tiny-let-shadowing-branch
    :source "(fn [x] (let [x (+ x 1)] (if (< x 10) (* x 6) (- x 1))))"
    :args [6]
    :expected 42}
   {:id :tiny-loop-recur
    :source "(fn [] (loop [i 0 acc 0] (if (< i 6) (recur (+ i 1) (+ acc 7)) acc)))"
    :args []
    :expected 42}
   {:id :tiny-boolean-const-if
    :source "(fn [] (if true 42 0))"
    :args []
    :expected 42}
   {:id :tiny-nil-falsey
    :source "(fn [] (if nil 0 42))"
    :args []
    :expected 42}
   {:id :tiny-equality-branch
    :source "(fn [x] (if (= x 7) 42 0))"
    :args [7]
    :expected 42}
   {:id :tiny-vector-literal
    :source "(fn [x] [x (+ x 1) true nil])"
    :args [40]
    :expected [40 41 true nil]}
   {:id :tiny-string-keyword-vector
    :source "(fn [] [\"ok\" :answer])"
    :args []
    :expected ["ok" :answer]}
   {:id :tiny-map-literal
    :source "(fn [x] {:answer x :label \"ok\" :flag true})"
    :args [42]
    :expected {:answer 42 :label "ok" :flag true}}
   {:id :tiny-set-literal
    :source "(fn [x] #{x 7 42})"
    :args [41]
    :expected #{41 7 42}}
   {:id :tiny-quoted-symbol
    :source "(fn [] (quote answer))"
    :args []
    :expected 'answer}
   {:id :tiny-quoted-list
    :source "(fn [] (quote (+ 1 2)))"
    :args []
    :expected '(+ 1 2)}
   {:id :tiny-quoted-nested-data
    :source "(fn [] (quote {:op answer :xs [1 2] :call (+ 1 2)}))"
    :args []
    :expected '{:op answer :xs [1 2] :call (+ 1 2)}}
   {:id :tiny-macro-when
    :source "(fn [x] (when (< x 50) (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-and
    :source "(fn [x] (and true (< x 50) (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-or
    :source "(fn [x] (or nil false (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-thread-first
    :source "(fn [x] (-> x (+ 1) (* 2)))"
    :args [20]
    :expected 42}
   {:id :tiny-macro-thread-last
    :source "(fn [x] (->> x (+ 1) (* 2)))"
    :args [20]
    :expected 42}
   {:id :tiny-macro-cond
    :source "(fn [x] (cond (< x 0) :neg (< x 10) :small :else :big))"
    :args [5]
    :expected :small}
   {:id :tiny-macro-when-not
    :source "(fn [x] (when-not (< x 0) (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-if-not
    :source "(fn [x] (if-not (< x 0) (+ x 1) 0))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-if-let
    :source "(fn [x] (if-let [y (+ x 1)] y 0))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-when-let
    :source "(fn [x] (when-let [y (+ x 1)] y))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-not
    :source "(fn [x] (if (not (< x 0)) (+ x 1) 0))"
    :args [41]
    :expected 42}
   {:id :tiny-op-gt
    :source "(fn [x] (if (> x 40) 42 0))"
    :args [41]
    :expected 42}
   {:id :tiny-op-gte
    :source "(fn [x] (if (>= x 41) 42 0))"
    :args [41]
    :expected 42}
   {:id :tiny-op-lte
    :source "(fn [x] (if (<= x 41) 42 0))"
    :args [41]
    :expected 42}
   {:id :tiny-op-quot
    :source "(fn [] (quot 85 2))"
    :args []
    :expected 42}
   {:id :tiny-op-rem
    :source "(fn [] (rem 142 100))"
    :args []
    :expected 42}
   {:id :tiny-op-inc
    :source "(fn [x] (inc x))"
    :args [41]
    :expected 42}
   {:id :tiny-op-dec
    :source "(fn [x] (dec x))"
    :args [43]
    :expected 42}
   {:id :tiny-op-zero?
    :source "(fn [x] (if (zero? x) 42 0))"
    :args [0]
    :expected 42}
   {:id :tiny-op-pos?
    :source "(fn [x] (if (pos? x) 42 0))"
    :args [5]
    :expected 42}
   {:id :tiny-op-neg?
    :source "(fn [x] (if (neg? x) 0 42))"
    :args [5]
    :expected 42}
   {:id :tiny-macro-as->
    :source "(fn [x] (as-> x v (+ v 1) (* v 2)))"
    :args [20]
    :expected 42}
   {:id :tiny-macro-cond->
    :source "(fn [x] (cond-> x (< x 50) (+ 1) (> x 100) (* 2)))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-cond->>
    :source "(fn [x] (cond->> x (< x 50) (+ 1)))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-some->
    :source "(fn [x] (some-> x (+ 1) (* 2)))"
    :args [20]
    :expected 42}
   {:id :tiny-macro-some->-nil
    :source "(fn [x] (some-> x (+ 1)))"
    :args [nil]
    :expected nil}
   {:id :tiny-macro-some->>
    :source "(fn [x] (some->> x (+ 1)))"
    :args [41]
    :expected 42}
   {:id :tiny-macro-nil?
    :source "(fn [x] (if (nil? x) 0 (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :tiny-op-first
    :source "(fn [v] (first v))"
    :args [[42 1 2]]
    :expected 42}
   {:id :tiny-op-next-first
    :source "(fn [v] (first (next v)))"
    :args [[1 42 3]]
    :expected 42}
   {:id :tiny-op-get
    :source "(fn [m] (get m :answer))"
    :args [{:answer 42}]
    :expected 42}
   {:id :tiny-destructure-vector
    :source "(fn [v] (let [[a b] v] (+ a b)))"
    :args [[20 22]]
    :expected 42}
   {:id :tiny-destructure-nested
    :source "(fn [v] (let [[a [b c]] v] (+ a (+ b c))))"
    :args [[10 [20 12]]]
    :expected 42}
   {:id :tiny-destructure-with-rest-positions
    :source "(fn [v] (let [[a b c] v] (if (nil? c) (+ a b) (+ a (+ b c)))))"
    :args [[20 22]]
    :expected 42}
   {:id :tiny-multi-arity-one-arg
    :source "(fn ([x] x) ([x y] (+ x y)))"
    :args [42]
    :expected 42}
   {:id :tiny-multi-arity-two-arg
    :source "(fn ([x] x) ([x y] (+ x y)))"
    :args [20 22]
    :expected 42}
   {:id :tiny-multi-arity-three-way-zero-arg
    :source "(fn ([] 42) ([x] x) ([x y] (+ x y)))"
    :args []
    :expected 42}
   {:id :tiny-multi-arity-three-way-two-arg
    :source "(fn ([] 42) ([x] x) ([x y] (+ x y)))"
    :args [20 22]
    :expected 42}
   {:id :tiny-variadic-rest-only
    :source "(fn [& r] r)"
    :args [1 2 3]
    :expected '(1 2 3)}
   {:id :tiny-variadic-rest-empty
    :source "(fn [a & r] r)"
    :args [1]
    :expected nil}
   {:id :tiny-variadic-one-fixed
    :source "(fn [a & r] r)"
    :args [1 2 3]
    :expected '(2 3)}
   {:id :tiny-variadic-two-fixed
    :source "(fn [a b & r] [a b r])"
    :args [1 2 3 4]
    :expected [1 2 '(3 4)]}
   {:id :tiny-mixed-arity-fixed-one-clause
    :source "(fn ([a] a) ([a b] (+ a b)) ([a b & r] (+ (+ a b) (count r))))"
    :args [1]
    :expected 1}
   {:id :tiny-mixed-arity-fixed-clause-wins-over-variadic
    :source "(fn ([a] a) ([a b] (+ a b)) ([a b & r] (+ (+ a b) (count r))))"
    :args [1 2]
    :expected 3}
   {:id :tiny-mixed-arity-variadic-clause-empty-rest
    :source "(fn ([a] a) ([a b] (+ a b)) ([a b & r] (+ (+ a b) (count r))))"
    :args [1 2 3]
    :expected 4}
   {:id :tiny-mixed-arity-variadic-clause-multi-rest
    :source "(fn ([a] a) ([a b] (+ a b)) ([a b & r] (+ (+ a b) (count r))))"
    :args [1 2 3 4]
    :expected 5}
   {:id :tiny-op-count-variadic
    :source "(fn [& r] (count r))"
    :args [1 2 3]
    :expected 3}
   {:id :tiny-op-count-vector
    :source "(fn [v] (count v))"
    :args [[1 2 3 4]]
    :expected 4}
   {:id :tiny-macro-case-int
    :source "(fn [n] (case n 1 :one 2 :two :other))"
    :args [2]
    :expected :two}
   {:id :tiny-macro-case-int-default
    :source "(fn [n] (case n 1 :one 2 :two :other))"
    :args [99]
    :expected :other}
   {:id :tiny-macro-case-keyword
    :source "(fn [k] (case k :a 1 :b 2 :c 3 :d 4 :e 5 :f 6 :g 7 :h 8 0))"
    :args [:g]
    :expected 7}
   {:id :tiny-macro-case-string
    :source "(fn [s] (case s \"Aa\" 1 \"BB\" 2 :other))"
    :args ["BB"]
    :expected 2}
   {:id :tiny-try-catch-no-throw
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero)))"
    :args [2]
    :expected 5}
   {:id :tiny-try-catch-caught
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero)))"
    :args [0]
    :expected :divzero}
   {:id :tiny-try-catch-exception-not-nil
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e (nil? e))))"
    :args [0]
    :expected false}
   {:id :tiny-try-catch-composes-with-let
    :source "(fn [x] (let [r (try (quot 10 x) (catch ArithmeticException e -1))] (+ r 1)))"
    :args [2]
    :expected 6}
   {:id :tiny-throw-one-arg-constructor
    :source "(fn [] (try (throw (IllegalArgumentException. \"boom\")) (catch IllegalArgumentException e (nil? e))))"
    :args []
    :expected false}
   {:id :tiny-throw-no-arg-constructor
    :source "(fn [] (try (throw (IllegalArgumentException.)) (catch IllegalArgumentException e :caught)))"
    :args []
    :expected :caught}
   {:id :tiny-throw-rethrow-caught-exception
    :source "(fn [x] (try (try (quot 10 x) (catch ArithmeticException e (throw e))) (catch ArithmeticException e2 :outer-caught)))"
    :args [0]
    :expected :outer-caught}
   {:id :tiny-macro-case-no-default-throws
    :source "(fn [n] (try (case n 1 :one 2 :two) (catch IllegalArgumentException e :no-match)))"
    :args [99]
    :expected :no-match}
   {:id :tiny-macro-case-no-default-matches
    :source "(fn [n] (try (case n 1 :one 2 :two) (catch IllegalArgumentException e :no-match)))"
    :args [1]
    :expected :one}
   {:id :tiny-interop-get-message
    :source "(fn [] (try (throw (IllegalArgumentException. \"boom\")) (catch IllegalArgumentException e (.getMessage e))))"
    :args []
    :expected "boom"}
   {:id :tiny-interop-string-length
    :source "(fn [s] (.length s))"
    :args ["hello"]
    :expected 5}
   {:id :tiny-interop-string-uppercase
    :source "(fn [s] (.toUpperCase s))"
    :args ["hi"]
    :expected "HI"}
   {:id :tiny-interop-equals-true
    :source "(fn [a b] (.equals a b))"
    :args [1 1]
    :expected true}
   {:id :tiny-interop-equals-false
    :source "(fn [a b] (.equals a b))"
    :args [1 2]
    :expected false}
   {:id :tiny-static-interop-math-sqrt
    :source "(fn [x] (Math/sqrt x))"
    :args [16.0]
    :expected 4.0}
   {:id :tiny-static-interop-integer-tostring
    :source "(fn [x] (Integer/toString x))"
    :args [42]
    :expected "42"}
   {:id :tiny-static-interop-string-valueof
    :source "(fn [x] (String/valueOf x))"
    :args [42]
    :expected "42"}
   {:id :tiny-static-interop-ambiguous-overload-rejected
    :source "(fn [a b] (try (Math/max a b) (catch IllegalArgumentException e :ambiguous)))"
    :args [1 2.0]
    :expected :ambiguous}
   {:id :tiny-try-finally-normal-path-value
    :source "(fn [a] (try a (finally 99)))"
    :args [42]
    :expected 42}
   {:id :tiny-try-finally-side-effect-after-normal-path
    :source "(fn [a] (do (try a (finally (.incrementAndGet a))) (.get a)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0)]
    :expected 1}
   {:id :tiny-try-finally-nested-in-try-catch-normal
    :source "(fn [x] (try (try (quot 10 x) (finally :ignored)) (catch ArithmeticException e :caught)))"
    :args [2]
    :expected 5}
   {:id :tiny-try-finally-nested-in-try-catch-exceptional
    :source "(fn [x] (try (try (quot 10 x) (finally :ignored)) (catch ArithmeticException e :caught)))"
    :args [0]
    :expected :caught}
   {:id :tiny-try-finally-runs-on-exceptional-path-nested
    :source "(fn [a x] (do (try (try (quot 10 x) (finally (.incrementAndGet a))) (catch ArithmeticException e :caught)) (.get a)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 0]
    :expected 1}
   {:id :tiny-try-catch-finally-normal-path-value
    :source "(fn [a x] (try (quot 10 x) (catch ArithmeticException e :divzero) (finally (.incrementAndGet a))))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 2]
    :expected 5}
   {:id :tiny-try-catch-finally-normal-path-counter
    :source "(fn [a x] (do (try (quot 10 x) (catch ArithmeticException e :divzero) (finally (.incrementAndGet a))) (.get a)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 2]
    :expected 1}
   {:id :tiny-try-catch-finally-caught-path-value
    :source "(fn [a x] (try (quot 10 x) (catch ArithmeticException e :divzero) (finally (.incrementAndGet a))))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 0]
    :expected :divzero}
   {:id :tiny-try-catch-finally-caught-path-counter
    :source "(fn [a x] (do (try (quot 10 x) (catch ArithmeticException e :divzero) (finally (.incrementAndGet a))) (.get a)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 0]
    :expected 1}
   {:id :tiny-try-catch-finally-unmatched-type-still-runs-finally
    :source "(fn [a] (try (try (throw (IllegalArgumentException. \"boom\")) (catch ArithmeticException e :wrong-type) (finally (.incrementAndGet a))) (catch IllegalArgumentException e2 :outer-caught)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0)]
    :expected :outer-caught}
   {:id :tiny-try-catch-finally-unmatched-type-counter
    :source "(fn [a] (do (try (try (throw (IllegalArgumentException. \"boom\")) (catch ArithmeticException e :wrong-type) (finally (.incrementAndGet a))) (catch IllegalArgumentException e2 :outer-caught)) (.get a)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0)]
    :expected 1}
   {:id :tiny-str-two-args
    :source "(fn [a b] (str a b))"
    :args ["hello" "world"]
    :expected "helloworld"}
   {:id :tiny-str-numeric-args
    :source "(fn [a b] (str a b))"
    :args [1 2]
    :expected "12"}
   {:id :tiny-str-one-arg
    :source "(fn [a] (str a))"
    :args ["x"]
    :expected "x"}
   {:id :tiny-str-zero-args
    :source "(fn [] (str))"
    :args []
    :expected ""}
   {:id :tiny-str-nil-arg-is-empty
    :source "(fn [a b] (str a b))"
    :args [nil "x"]
    :expected "x"}
   {:id :tiny-str-with-string-literal
    :source "(fn [k] (str \"value: \" k))"
    :args [42]
    :expected "value: 42"}
   {:id :tiny-general-core-fn-call-map-with-value-arg
    :source "(fn [coll] (map inc coll))"
    :args [[1 2 3]]
    :expected '(2 3 4)}
   {:id :tiny-general-core-fn-call-filter-with-value-arg
    :source "(fn [coll] (filter pos? coll))"
    :args [[-1 2 -3 4]]
    :expected '(2 4)}
   {:id :tiny-general-core-fn-call-reduce-with-value-arg
    :source "(fn [coll] (reduce + coll))"
    :args [[1 2 3 4]]
    :expected 10}
   {:id :tiny-general-core-fn-call-apply
    :source "(fn [coll] (apply + coll))"
    :args [[1 2 3 4]]
    :expected 10}
   {:id :tiny-general-core-fn-call-conj
    :source "(fn [coll x] (conj coll x))"
    :args [[1 2] 3]
    :expected [1 2 3]}
   {:id :tiny-general-core-fn-call-assoc
    :source "(fn [m k v] (assoc m k v))"
    :args [{} :a 1]
    :expected {:a 1}}
   {:id :tiny-general-core-fn-call-vec
    :source "(fn [coll] (vec coll))"
    :args [(list 1 2 3)]
    :expected [1 2 3]}
   {:id :tiny-general-core-fn-call-into
    :source "(fn [a b] (into a b))"
    :args [[] [1 2 3]]
    :expected [1 2 3]}
   {:id :tiny-local-fn-call-param
    :source "(fn [f x] (f x))"
    :args [inc 5]
    :expected 6}
   {:id :tiny-local-fn-call-passed-to-core-fn
    :source "(fn [coll f] (map f coll))"
    :args [[1 2 3] inc]
    :expected '(2 3 4)}
   {:id :tiny-local-fn-call-let-bound
    :source "(fn [x] (let [f inc] (f x)))"
    :args [5]
    :expected 6}
   {:id :tiny-variadic-plus-three-args
    :source "(fn [a b c] (+ a b c))"
    :args [1 2 3]
    :expected 6}
   {:id :tiny-variadic-plus-four-args
    :source "(fn [a b c d] (+ a b c d))"
    :args [1 2 3 4]
    :expected 10}
   {:id :tiny-variadic-plus-zero-args
    :source "(fn [] (+))"
    :args []
    :expected 0}
   {:id :tiny-variadic-plus-one-arg
    :source "(fn [a] (+ a))"
    :args [5]
    :expected 5}
   {:id :tiny-variadic-minus-three-args
    :source "(fn [a b c] (- a b c))"
    :args [10 2 3]
    :expected 5}
   {:id :tiny-unary-minus
    :source "(fn [a] (- a))"
    :args [5]
    :expected -5}
   {:id :tiny-variadic-times-three-args
    :source "(fn [a b c] (* a b c))"
    :args [2 3 4]
    :expected 24}
   {:id :tiny-variadic-times-zero-args
    :source "(fn [] (*))"
    :args []
    :expected 1}
   {:id :tiny-variadic-times-one-arg
    :source "(fn [a] (* a))"
    :args [7]
    :expected 7}
   {:id :tiny-chained-lt-three-args-true
    :source "(fn [a b c] (< a b c))"
    :args [1 2 3]
    :expected true}
   {:id :tiny-chained-lt-three-args-false
    :source "(fn [a b c] (< a b c))"
    :args [1 3 2]
    :expected false}
   {:id :tiny-chained-lt-one-arg
    :source "(fn [a] (< a))"
    :args [5]
    :expected true}
   {:id :tiny-chained-eq-three-args-true
    :source "(fn [a b c] (= a b c))"
    :args [1 1 1]
    :expected true}
   {:id :tiny-chained-eq-three-args-false
    :source "(fn [a b c] (= a b c))"
    :args [1 1 2]
    :expected false}
   {:id :tiny-chained-gt-three-args
    :source "(fn [a b c] (> a b c))"
    :args [3 2 1]
    :expected true}
   {:id :tiny-chained-lte-three-args
    :source "(fn [a b c] (<= a b c))"
    :args [1 1 2]
    :expected true}
   {:id :tiny-chained-gte-three-args
    :source "(fn [a b c] (>= a b c))"
    :args [3 3 2]
    :expected true}
   {:id :tiny-get3-key-present
    :source "(fn [m k d] (get m k d))"
    :args [{:a 1} :a 99]
    :expected 1}
   {:id :tiny-get3-key-missing
    :source "(fn [m k d] (get m k d))"
    :args [{:a 1} :b 99]
    :expected 99}
   {:id :tiny-named-fn-self-recur
    :source "(fn foo [n] (if (= n 0) 0 (foo (- n 1))))"
    :args [5]
    :expected 0}
   {:id :tiny-named-fn-self-recur-return-value
    :source "(fn foo [n] (if (= n 0) :done (foo (- n 1))))"
    :args [3]
    :expected :done}
   {:id :tiny-named-fn-param-shadows-self-name
    :source "(fn foo [foo] foo)"
    :args [42]
    :expected 42}
   {:id :tiny-named-fn-mixed-arity-self-recur
    :source "(fn count-down ([n] (count-down n 0)) ([n acc] (if (= n 0) acc (count-down (- n 1) (+ acc 1)))))"
    :args [5]
    :expected 5}
   {:id :tiny-computed-fn-call-head
    :source "(fn [x] ((constantly x) 99))"
    :args [7]
    :expected 7}
   {:id :tiny-keyword-as-fn-key-present
    :source "(fn [m] (:a m))"
    :args [{:a 42}]
    :expected 42}
   {:id :tiny-keyword-as-fn-key-missing
    :source "(fn [m] (:z m))"
    :args [{:a 42}]
    :expected nil}
   {:id :tiny-fn-tail-recur
    :source "(fn [n] (if (= n 0) 0 (recur (- n 1))))"
    :args [100000]
    :expected 0}
   {:id :tiny-fn-tail-recur-variadic
    :source "(fn [n & r] (if (= n 0) r (recur (- n 1) (cons n r))))"
    :args [3]
    :expected '(1 2 3)}
   {:id :tiny-fn-tail-recur-with-self-name
    :source "(fn foo [n] (if (= n 0) 0 (recur (- n 1))))"
    :args [5]
    :expected 0}
   {:id :tiny-nested-loop-recur-shadows-fn-recur
    :source "(fn [n] (+ n (loop [i 0 acc 0] (if (< i 3) (recur (+ i 1) (+ acc 10)) acc))))"
    :args [1]
    :expected 31}
   {:id :tiny-general-catch-class-ex-info-data
    :source "(fn [] (try (throw (ex-info \"boom\" {:a 1})) (catch clojure.lang.ExceptionInfo e (ex-data e))))"
    :args []
    :expected {:a 1}}
   {:id :tiny-general-catch-class-ex-info-message
    :source "(fn [] (try (throw (ex-info \"boom\" {:a 1})) (catch clojure.lang.ExceptionInfo e (.getMessage e))))"
    :args []
    :expected "boom"}
   {:id :tiny-general-catch-class-npe
    :source "(fn [x] (try (.length x) (catch java.lang.NullPointerException e :caught-npe)))"
    :args [nil]
    :expected :caught-npe}
   ;; Regression fixtures for a real bug this slice found and fixed:
   ;; `GeneratorAdapter/box` on a boolean built `new Boolean(z)` (a
   ;; non-singleton instance), not `Boolean.valueOf(z)` -- invisible to
   ;; every fixture using this witness's OWN `if` (`RT.booleanCast`
   ;; tolerates non-singleton Booleans fine), but real host's OWN compiled
   ;; `if` (confirmed via `javap -c`) does raw REFERENCE-IDENTITY
   ;; comparison against `Boolean.FALSE`, so a witness-emitted `<`/`>`/`=`/
   ;; `zero?`/etc. result (or a literal `true`/`false` constant) crossing
   ;; into real host code doing that check -- `clojure.core/filter` is
   ;; exactly such code -- was silently always truthy. `identical?`
   ;; against the real singleton is the direct regression check;
   ;; `filter` below is the real-world case that surfaced it.
   {:id :tiny-boolean-identity-comparison-result
    :source "(fn [a b] (identical? (> a b) true))"
    :args [5 3]
    :expected true}
   {:id :tiny-boolean-identity-literal-constant
    :source "(fn [] (identical? true true))"
    :args []
    :expected true}
   {:id :tiny-closure-filter-with-capture
    :source "(fn [coll threshold] (filter (fn [x] (> x threshold)) coll))"
    :args [[1 5 10] 3]
    :expected '(5 10)}
   {:id :tiny-closure-map-with-capture
    :source "(fn [coll y] (map (fn [x] (+ x y)) coll))"
    :args [[1 2 3] 10]
    :expected '(11 12 13)}
   {:id :tiny-closure-no-capture
    :source "(fn [coll] (map (fn [x] (* x x)) coll))"
    :args [[1 2 3]]
    :expected '(1 4 9)}
   {:id :tiny-closure-immediate-invoke
    :source "(fn [x] ((fn [y] (+ x y)) 4))"
    :args [3]
    :expected 7}
   {:id :tiny-closure-multiple-captures
    :source "(fn [x y] ((fn [z] (+ x (+ y z))) 3))"
    :args [1 2]
    :expected 6}
   {:id :tiny-closure-self-recur
    :source "(fn [n] ((fn count-down [i] (if (= i 0) 0 (count-down (- i 1)))) n))"
    :args [5]
    :expected 0}
   {:id :tiny-closure-variadic
    :source "(fn [x] ((fn [& r] (cons x r)) 2 3))"
    :args [1]
    :expected '(1 2 3)}
   {:id :tiny-closure-double-nested-transitive-capture
    :source "(fn [x] (((fn [y] (fn [z] (+ x (+ y z)))) 2) 3))"
    :args [1]
    :expected 6}
   {:id :tiny-closure-quadruple-nested
    :source "(fn [a] ((((fn [b] (fn [c] (fn [d] (+ a (+ b (+ c d)))))) 2) 3) 4))"
    :args [1]
    :expected 10}
   {:id :tiny-letfn-single-binding-captures-outer
    :source "(fn [x] (letfn [(add-x [y] (+ x y))] (add-x 10)))"
    :args [3]
    :expected 13}
   {:id :tiny-letfn-self-recursive-binding
    :source "(fn [n] (letfn [(fact [i] (if (= i 0) 1 (* i (fact (- i 1)))))] (fact n)))"
    :args [5]
    :expected 120}
   {:id :tiny-letfn-two-independent-bindings
    :source "(fn [x] (letfn [(add-x [y] (+ x y)) (double-x [] (* x 2))] (+ (add-x 10) (double-x))))"
    :args [3]
    :expected 19}
   {:id :tiny-reify-function-apply
    :source "(fn [x a] (.apply (reify java.util.function.Function (apply [this y] (+ x y))) a))"
    :args [10 5]
    :expected 15}
   {:id :tiny-reify-comparator-primitive-return
    :source "(fn [x a b] (.compare (reify java.util.Comparator (compare [this p q] (- p (+ q x)))) a b))"
    :args [0 10 3]
    :expected 7}
   {:id :tiny-reify-comparator-used-by-sort
    :source "(fn [coll] (sort (reify java.util.Comparator (compare [this a b] (- a b))) coll))"
    :args [[5 1 9 3]]
    :expected '(1 3 5 9)}
   {:id :tiny-reify-runnable-void-return-side-effect
    :source "(fn [] (let [counter (java.util.concurrent.atomic.AtomicInteger. 0)] (.run (reify java.lang.Runnable (run [this] (.set counter 99)))) (.get counter)))"
    :args []
    :expected 99}
   {:id :tiny-reify-supplier-no-capture
    :source "(fn [] (.get (reify java.util.function.Supplier (get [this] 42))))"
    :args []
    :expected 42}
   {:id :tiny-deftype-construct-and-access
    :source "(do (deftype Point [x y]) (fn [a b] (let [p (Point. a b)] (+ (.-x p) (.-y p)))))"
    :args [3 4]
    :expected 7}
   {:id :tiny-deftype-field-directly
    :source "(do (deftype Point [x y]) (fn [a b] (.-x (Point. a b))))"
    :args [10 20]
    :expected 10}
   {:id :tiny-deftype-three-fields
    :source "(do (deftype Vec3 [x y z]) (fn [] (let [v (Vec3. 1 2 3)] (+ (.-x v) (+ (.-y v) (.-z v))))))"
    :args []
    :expected 6}
   {:id :tiny-deftype-two-independent-types
    :source "(do (deftype A [n]) (deftype B [n]) (fn [] (+ (.-n (A. 10)) (.-n (B. 20)))))"
    :args []
    :expected 30}
   {:id :tiny-protocol-basic-dispatch
    :source "(do (defprotocol Greet (greet [this])) (fn [] (greet (reify Greet (greet [this] :hello)))))"
    :args []
    :expected :hello}
   {:id :tiny-protocol-method-with-arg-and-capture
    :source "(do (defprotocol Adder (add-to [this x])) (fn [n] (add-to (reify Adder (add-to [this x] (+ x n))) 10)))"
    :args [5]
    :expected 15}
   {:id :tiny-protocol-two-methods
    :source "(do (defprotocol Shape (area [this]) (perimeter [this])) (fn [w h] (let [r (reify Shape (area [this] (* w h)) (perimeter [this] (* 2 (+ w h))))] (+ (area r) (perimeter r)))))"
    :args [3 4]
    :expected 26}
   {:id :tiny-protocol-and-deftype-mixed-program
    :source "(do (defprotocol Greet (greet [this])) (deftype Person [name]) (fn [] (greet (reify Greet (greet [this] :hi)))))"
    :args []
    :expected :hi}
   {:id :tiny-deftype-implements-protocol
    :source "(do (defprotocol Shape (area [this])) (deftype Rect [w h] Shape (area [this] (* w h))) (fn [a b] (area (Rect. a b))))"
    :args [3 4]
    :expected 12}
   {:id :tiny-deftype-implements-protocol-two-methods
    :source "(do (defprotocol Shape (area [this]) (perimeter [this])) (deftype Rect [w h] Shape (area [this] (* w h)) (perimeter [this] (* 2 (+ w h)))) (fn [a b] (+ (area (Rect. a b)) (perimeter (Rect. a b)))))"
    :args [3 4]
    :expected 26}
   {:id :tiny-field-get
    :source "(fn [p] (.-x p))"
    :args [(java.awt.Point. 7 9)]
    :expected 7}
   {:id :tiny-field-set-returns-assigned-value
    :source "(fn [p v] (set! (.-x p) v))"
    :args [(java.awt.Point.) 8]
    :expected 8}
   {:id :tiny-field-set-mutates-then-readback
    :source "(fn [p] (do (set! (.-x p) 7) (.-x p)))"
    :args [(java.awt.Point.)]
    :expected 7}
   {:id :tiny-locking-normal-path
    :source "(fn [sb] (do (locking sb (.append sb \"x\")) (.toString sb)))"
    :args [(StringBuilder.)]
    :expected "x"}
   {:id :tiny-locking-exceptional-path-propagates
    :source "(fn [lock x] (try (locking lock (quot 10 x)) (catch ArithmeticException e :caught)))"
    :args [(Object.) 0]
    :expected :caught}
   {:id :tiny-try-bare-no-clauses
    :source "(fn [] (try 42))"
    :args []
    :expected 42}
   {:id :tiny-try-multi-catch-first-matches
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg)))"
    :args [2]
    :expected 5}
   {:id :tiny-try-multi-catch-first-clause-triggered
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg)))"
    :args [0]
    :expected :divzero}
   {:id :tiny-try-multi-catch-second-clause-triggered
    :source "(fn [] (try (throw (IllegalArgumentException. \"bad\")) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg)))"
    :args []
    :expected :bad-arg}
   {:id :tiny-try-multi-catch-three-clauses-third-triggered
    :source "(fn [] (try (throw (RuntimeException. \"boom\")) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg) (catch RuntimeException e :runtime)))"
    :args []
    :expected :runtime}
   {:id :tiny-try-multi-catch-finally-normal-path
    :source "(fn [a x] (try (quot 10 x) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg) (finally (.incrementAndGet a))))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 2]
    :expected 5}
   {:id :tiny-try-multi-catch-finally-first-clause-value
    :source "(fn [a x] (try (quot 10 x) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg) (finally (.incrementAndGet a))))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 0]
    :expected :divzero}
   {:id :tiny-try-multi-catch-finally-first-clause-counter
    :source "(fn [a x] (do (try (quot 10 x) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg) (finally (.incrementAndGet a))) (.get a)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0) 0]
    :expected 1}
   {:id :tiny-try-multi-catch-finally-second-clause-value
    :source "(fn [a] (try (throw (IllegalArgumentException. \"bad\")) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg) (finally (.incrementAndGet a))))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0)]
    :expected :bad-arg}
   {:id :tiny-try-multi-catch-finally-exception-in-catch-body-still-runs-finally
    :source "(fn [a] (try (try (throw (IllegalArgumentException. \"bad\")) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e (throw (RuntimeException. \"from-catch\"))) (finally (.incrementAndGet a))) (catch RuntimeException e2 :outer-caught)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0)]
    :expected :outer-caught}
   {:id :tiny-try-multi-catch-finally-exception-in-catch-body-counter
    :source "(fn [a] (do (try (try (throw (IllegalArgumentException. \"bad\")) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e (throw (RuntimeException. \"from-catch\"))) (finally (.incrementAndGet a))) (catch RuntimeException e2 :outer-caught)) (.get a)))"
    :args [(java.util.concurrent.atomic.AtomicInteger. 0)]
    :expected 1}
   {:id :tiny-general-new-unique-arity
    :source "(fn [x y] (.-x (java.awt.Point. x y)))"
    :args [7 9]
    :expected 7}
   {:id :tiny-general-new-ambiguous-arity-int-overload
    :source "(fn [n] (.size (java.util.ArrayList. n)))"
    :args [4]
    :expected 0}
   {:id :tiny-general-new-ambiguous-arity-collection-overload
    :source "(fn [coll] (.size (java.util.ArrayList. coll)))"
    :args [(java.util.Arrays/asList (into-array [1 2 3]))]
    :expected 3}
   {:id :tiny-general-new-no-arg
    :source "(fn [] (.size (java.util.ArrayList.)))"
    :args []
    :expected 0}
   {:id :tiny-general-static-interop-digit-true
    :source "(fn [c] (java.lang.Character/isDigit c))"
    :args [\5]
    :expected true}
   {:id :tiny-general-static-interop-digit-false
    :source "(fn [c] (java.lang.Character/isDigit c))"
    :args [\a]
    :expected false}
   {:id :tiny-general-static-interop-no-arg
    :source "(fn [] (java.util.Collections/emptyList))"
    :args []
    :expected []}
   {:id :tiny-bigint-literal
    :source "(fn [] 5N)"
    :args []
    :expected 5N}
   {:id :tiny-bigint-literal-beyond-long-range
    :source "(fn [] 10000000000000000000N)"
    :args []
    :expected 10000000000000000000N}
   {:id :tiny-bigdec-literal
    :source "(fn [] 1.5M)"
    :args []
    :expected 1.5M}
   {:id :tiny-bigint-arithmetic-beyond-long-overflow
    :source "(fn [] (+ 9223372036854775807N 1N))"
    :args []
    :expected 9223372036854775808N}
   {:id :tiny-bigdec-arithmetic
    :source "(fn [] (* 1.5M 2))"
    :args []
    :expected 3.0M}
   {:id :tiny-bigint-equals-long
    :source "(fn [] (= 5N 5))"
    :args []
    :expected true}
   ;; `java.util.regex.Pattern` does not override `.equals` (identity-based),
   ;; confirmed live: `(= #"a+" #"a+")` is `false` for two distinct
   ;; instances -- so this fixture compares the pattern SOURCE STRING via
   ;; `.pattern`, not the `Pattern` object itself, which the already-general
   ;; Reflector-based `.methodName` interop call handles for free.
   {:id :tiny-regex-literal-pattern-source
    :source "(fn [] (.pattern #\"a+\"))"
    :args []
    :expected "a+"}
   {:id :tiny-ratio-literal
    :source "(fn [] 1/3)"
    :args []
    :expected 1/3}
   {:id :tiny-ratio-literal-collapses-to-long
    :source "(fn [] 4/2)"
    :args []
    :expected 2}
   {:id :tiny-ratio-arithmetic
    :source "(fn [] (+ 1/3 1/3))"
    :args []
    :expected 2/3}
   {:id :tiny-binding-value-inside
    :source "(fn [] (binding [*tiny-dynamic-var* 42] *tiny-dynamic-var*))"
    :args []
    :expected 42}
   {:id :tiny-binding-reverts-after-normal-exit
    :source "(fn [] (do (binding [*tiny-dynamic-var* 42] nil) *tiny-dynamic-var*))"
    :args []
    :expected :tiny-dynamic-var-root}
   {:id :tiny-binding-reverts-after-exceptional-exit
    :source "(fn [] (do (try (binding [*tiny-dynamic-var* 99] (throw (RuntimeException.))) (catch RuntimeException e nil)) *tiny-dynamic-var*))"
    :args []
    :expected :tiny-dynamic-var-root}])

(defn run
  []
  (reset-compiler-state!)
  (let [rows (mapv case-row (specs))
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        held-row {:id :production-frontend-replacement-boundary
                  :kind :frontier-boundary
                  :gate/verdict :accepted
                  :boundary/status :declared
                  :boundary/reason :tiny-frontend-covers-only-expression-let-do-loop-data-literal-quote-macro-and-core-op-subset
                  :not-claimed [:full-reader
                                :production-clojure-core-macroexpander
                                :tools-analyzer-replacement
                                :full-special-form-surface]
                  :ok true}
        all-rows (conj rows held-row)
        canonical (mapv #(select-keys %
                                      [:id :kind :source :expanded-form
                                       :args :expected :got
                                       :artifact :frontend :gate/verdict
                                       :boundary/status :boundary/reason
                                       :not-claimed :ok])
                        all-rows)
        invariants (sorted-map
                    :tiny-frontend-accepted (empty? rejected)
                    :tools-analyzer-not-used
                    (every? #(false? (get-in % [:frontend :uses-tools-analyzer-jvm]))
                            rows)
                    :clojure-reader-not-used
                    (every? #(false? (get-in % [:frontend :uses-clojure-reader]))
                            rows)
                    :host-macroexpand-not-used
                    (every? #(false? (get-in % [:frontend :uses-host-macroexpand]))
                            rows)
                    :macro-subset-accepted
                    (every? #(= :accepted (:gate/verdict %))
                            (filter (comp #{:tiny-macro-when
                                            :tiny-macro-and
                                            :tiny-macro-or
                                            :tiny-macro-not
                                            :tiny-macro-thread-first
                                            :tiny-macro-thread-last
                                            :tiny-macro-cond
                                            :tiny-macro-when-not
                                            :tiny-macro-if-not
                                            :tiny-macro-if-let
                                            :tiny-macro-when-let
                                            :tiny-macro-as->
                                            :tiny-macro-cond->
                                            :tiny-macro-cond->>
                                            :tiny-macro-some->
                                            :tiny-macro-some->-nil
                                            :tiny-macro-some->>
                                            :tiny-macro-nil?
                                            :tiny-macro-case-int
                                            :tiny-macro-case-int-default
                                            :tiny-macro-case-keyword
                                            :tiny-macro-case-string}
                                          :id)
                                    rows))
                    :op-subset-accepted
                    (every? #(= :accepted (:gate/verdict %))
                            (filter (comp #{:tiny-op-gt :tiny-op-gte :tiny-op-lte
                                            :tiny-op-quot :tiny-op-rem
                                            :tiny-op-inc :tiny-op-dec
                                            :tiny-op-zero? :tiny-op-pos? :tiny-op-neg?
                                            :tiny-op-first :tiny-op-next-first :tiny-op-get
                                            :tiny-op-count-variadic :tiny-op-count-vector}
                                          :id)
                                    rows))
                    :destructure-subset-accepted
                    (every? #(= :accepted (:gate/verdict %))
                            (filter (comp #{:tiny-destructure-vector
                                            :tiny-destructure-nested
                                            :tiny-destructure-with-rest-positions}
                                          :id)
                                    rows))
                    :let-do-subset-accepted
                    (every? #(= :accepted (:gate/verdict %))
                            (filter (comp #{:tiny-do-body
                                            :tiny-let-sequential
                                            :tiny-let-shadowing-branch}
                                          :id)
                                    rows))
                    :loop-recur-subset-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :tiny-loop-recur (:id %)) rows))))
                    :literal-and-vector-subset-accepted
                    (every? #(= :accepted (:gate/verdict %))
                            (filter (comp #{:tiny-boolean-const-if
                                            :tiny-nil-falsey
                                            :tiny-equality-branch
                                            :tiny-vector-literal
                                            :tiny-string-keyword-vector
                                            :tiny-map-literal
                                            :tiny-set-literal
                                            :tiny-quoted-symbol
                                            :tiny-quoted-list
                                            :tiny-quoted-nested-data}
                                          :id)
                                    rows))
                    :production-boundary-declared
                    (and (= :accepted (:gate/verdict held-row))
                         (= :declared (:boundary/status held-row)))
                    :all-rows-ok (every? :ok all-rows))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.frontend-selfhost.receipt.v1"
     :stage [:U6 :R5 :frontend-selfhost]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "self-owned reader/macroexpander/analyzer/direct ASM emitter path for a small expression/let/do/loop/data-literal/quote/macro(when,and,or,not,nil?,->,->>,cond,when-not,if-not,if-let,when-let,as->,cond->,cond->>,some->,some->>)/core-op(< > >= <= = + - * quot rem inc dec zero? pos? neg? first next get)/vector-destructuring(self-owned, via first/next) subset with production frontend replacement held"
     :macroexpander {:kind :self-owned-rewrite-rules
                     :rules tiny-macro-rules
                     :uses-host-macroexpand false
                     :production-macroexpander :declared-boundary}
     :status-counts {:accepted (inc (count accepted))
                     :declared-boundary 1
                     :rejected (count rejected)}
     :rows all-rows
     :invariants invariants
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok ok?}))

(defn write-receipt!
  [r]
  (io/make-parents receipt-path)
  (spit receipt-path (with-out-str (pp/pprint r)))
  r)

(defn -main
  [& _]
  (let [r (write-receipt! (run))]
    (doseq [row (:rows r)]
      (println (str "  [" (if (:ok row) "OK" "FAIL") "] "
                    (name (:id row))
                    " -> "
                    (name (:gate/verdict row)))))
    (println (str "frontend selfhost: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
