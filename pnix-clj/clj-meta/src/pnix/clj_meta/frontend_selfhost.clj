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
           [clojure.lang AFunction DynamicClassLoader IFn Keyword Numbers Reflector RestFn RT Symbol Util Var]
           [java.security MessageDigest]))

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
(def ^:private object-array-class (Class/forName "[Ljava.lang.Object;"))
(def ^:private init-method
  (Method. "<init>" Type/VOID_TYPE (into-array Type [])))
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
(def ^:private reflector-invoke-noarg-instance-member-method
  (reflect-asm-method Reflector "invokeNoArgInstanceMember" [Object String Boolean/TYPE]))
(def ^:private reflector-set-instance-field-method
  (reflect-asm-method Reflector "setInstanceField" [Object String Object]))
(def ^:private rt-classfor-name-method
  (reflect-asm-method RT "classForName" [String]))
(def ^:private reflector-invoke-constructor-method
  (reflect-asm-method Reflector "invokeConstructor" [Class object-array-class]))

(defn- next-class-name
  []
  (str "pnix.clj_meta.frontend_selfhost.Fn__" (swap! class-counter inc)))

(defn reset-compiler-state!
  "Reset deterministic class naming for receipt-generating callers."
  []
  (reset! class-counter -1))

(defn- tokenize
  [source]
  (->> (re-seq #"\s*(#\{|\"[^\"]*\"|[\(\)\[\]\{\}]|[^\s\(\)\[\]\{\}]+)" source)
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

      (and (.startsWith ^String token ":")
           (< 1 (count token)))
      (keyword (subs token 1))

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

(defn- analyze-call
  [env form]
  (let [[op & args] form]
    (cond
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
                (let [[_ class-sym name-sym catch-body-form] clause-form]
                  (when-not (and (= 4 (count clause-form))
                                 (contains? known-exception-classes class-sym)
                                 (symbol? name-sym))
                    (throw (ex-info "tiny analyzer: malformed catch clause (supported classes: ArithmeticException, Exception, RuntimeException, Throwable, IllegalArgumentException)"
                                    {:form form})))
                  {:catch-class (get known-exception-classes class-sym)
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
      (+ - * < = > >= <= quot rem get)
      (do
        (when-not (= 2 (count args))
          (throw (ex-info "tiny analyzer: binary op arity"
                          {:form form})))
        {:op :binary
         :fn op
         :lhs (analyze-expr env (first args))
         :rhs (analyze-expr env (second args))})
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
      str
      {:op :core-fn-call
       :fn-name "str"
       :args (mapv #(analyze-expr env %) args)}
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
      (throw (ex-info "tiny analyzer: unsupported call"
                      {:form form :op op}))))))

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
    (if (contains? env form)
      {:op :local :name form}
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
  `params-vector` may end in `& rest-name` for a variadic clause."
  [clause]
  (let [raw-params (first clause)
        body (rest clause)]
    (when-not (and (vector? raw-params) (seq body))
      (throw (ex-info "tiny analyzer: malformed fn clause" {:clause clause})))
    (let [[params rest-param] (split-variadic-params raw-params)
          all-names (cond-> params rest-param (conj rest-param))]
      (when-not (and (every? symbol? all-names)
                     (= (count all-names) (count (distinct all-names))))
        (throw (ex-info "tiny analyzer: malformed fn clause" {:clause clause})))
      (let [env (zipmap all-names (repeat true))]
        {:params params
         :rest-param rest-param
         :body (analyze-body env body)}))))

(defn- analyze-fn
  [form]
  (when-not (and (seq? form) (= 'fn (first form)))
    (throw (ex-info "tiny analyzer: expected fn form" {:form form})))
  (let [rest-form (rest form)
        ;; `(fn ([x] ..) ([x y] ..))`: every clause after `fn` is itself a
        ;; list. `(fn [x] ..)`: the single clause IS `rest-form` (its head is
        ;; a vector, not a list), so it is wrapped as the sole clause below.
        multi-arity? (and (seq rest-form) (seq? (first rest-form)))
        clauses (if multi-arity? rest-form [rest-form])
        arities (mapv analyze-fn-clause clauses)
        param-counts (map (comp count :params) arities)]
    (when-not (seq arities)
      (throw (ex-info "tiny analyzer: fn needs at least one arity" {:form form})))
    (when-not (= (count param-counts) (count (distinct param-counts)))
      (throw (ex-info "tiny analyzer: duplicate fn arity" {:form form})))
    ;; Deliberately narrow scope: a variadic clause (`& rest`) may only
    ;; appear alone, not mixed with other fixed arities in the same `fn`.
    ;; Real `clojure.lang.RestFn` subclasses CAN mix lower fixed arities
    ;; with one variadic "ceiling" arity, but that needs additional
    ;; lower-arity `invoke` overrides beyond `doInvoke`+`getRequiredArity` --
    ;; a separate, larger slice, not attempted here.
    (when (and (some :rest-param arities) (> (count arities) 1))
      (throw (ex-info "tiny analyzer: variadic fn cannot be mixed with other arities"
                      {:form form})))
    {:op :fn
     :arities arities}))

(declare emit-expr)

(defn- emit-nil
  [^GeneratorAdapter ga]
  (.visitInsn ga Opcodes/ACONST_NULL))

(defn- emit-const
  [^GeneratorAdapter ga value]
  (cond
    (nil? value)
    (emit-nil ga)

    (integer? value)
    (do
      (.push ga (long value))
      (.box ga Type/LONG_TYPE))

    (boolean? value)
    (do
      (.push ga (boolean value))
      (.box ga Type/BOOLEAN_TYPE))

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
        (.box ga Type/BOOLEAN_TYPE))
    > (do
        (.invokeStatic ga numbers-type numbers-gt-method)
        (.box ga Type/BOOLEAN_TYPE))
    >= (do
         (.invokeStatic ga numbers-type numbers-gte-method)
         (.box ga Type/BOOLEAN_TYPE))
    <= (do
         (.invokeStatic ga numbers-type numbers-lte-method)
         (.box ga Type/BOOLEAN_TYPE))
    = (do
        (.invokeStatic ga util-type util-equiv-method)
        (.box ga Type/BOOLEAN_TYPE))))

(defn- emit-unary
  [^GeneratorAdapter ga env {:keys [fn arg]}]
  (emit-expr ga env arg)
  (case fn
    inc (.invokeStatic ga numbers-type numbers-inc-method)
    dec (.invokeStatic ga numbers-type numbers-dec-method)
    first (.invokeStatic ga rt-type rt-first-method)
    next (.invokeStatic ga rt-type rt-next-method)
    zero? (do
            (.invokeStatic ga numbers-type numbers-iszero-method)
            (.box ga Type/BOOLEAN_TYPE))
    pos? (do
           (.invokeStatic ga numbers-type numbers-ispos-method)
           (.box ga Type/BOOLEAN_TYPE))
    neg? (do
           (.invokeStatic ga numbers-type numbers-isneg-method)
           (.box ga Type/BOOLEAN_TYPE))
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
        (.storeLocal ga (int slot)))
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
               (conj slots slot)))
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

(defn- emit-general-new
  [^GeneratorAdapter ga env {:keys [class-name args]}]
  (.push ga ^String class-name)
  (.invokeStatic ga rt-type rt-classfor-name-method)
  (emit-object-array ga env args)
  (.invokeStatic ga reflector-type reflector-invoke-constructor-method))

(defn- invoke-method
  [arity]
  (Method. "invoke"
           obj-type
           (into-array Type (repeat arity obj-type))))

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
    :try-multi-catch (emit-try-multi-catch ga env node)
    :try-multi-catch-finally (emit-try-multi-catch-finally ga env node)
    :new (emit-new ga env node)
    :general-new (emit-general-new ga env node)
    :throw (emit-throw ga env node)
    :interop-call (emit-interop-call ga env node)
    :static-interop-call (emit-static-interop-call ga env node)
    :core-fn-call (emit-core-fn-call ga env node)
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
  [{:keys [arities]}]
  (let [variadic? (boolean (some :rest-param arities))
        class-name (next-class-name)
        internal (.replace class-name \. \/)
        ctype (Type/getObjectType internal)
        base-type (if variadic? restfn-type afn-type)
        cw (ClassWriter. (bit-or ClassWriter/COMPUTE_FRAMES
                                 ClassWriter/COMPUTE_MAXS))]
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC internal nil
            (.getInternalName base-type) nil)
    (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC init-method nil nil cw)]
      (.loadThis ga)
      (.invokeConstructor ga base-type init-method)
      (.returnValue ga)
      (.endMethod ga))
    (if variadic?
      ;; A single variadic clause: extends RestFn instead of AFunction.
      ;; RestFn already implements EVERY public invoke(...) overload
      ;; concretely (confirmed via `javap` on the real clojure.jar) -- it
      ;; handles argument-count matching and rest-sequence collection
      ;; entirely on its own. A subclass only supplies the two
      ;; overridable pieces: getRequiredArity() and the ONE doInvoke
      ;; overload whose param count is (fixed-params + 1), the last slot
      ;; being the collected rest sequence (or nil if none were passed).
      ;; This exact shape -- not merely a similar one -- was
      ;; reverse-engineered from real host-AOT-compiled `(fn [a & r] r)`,
      ;; `(fn [& r] r)`, and `(fn [a b & r] r)` via `javap -c` before being
      ;; written here.
      (let [{:keys [params rest-param body]} (first arities)
            all-params (if rest-param (conj params rest-param) params)
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC
                                  (do-invoke-method (count all-params))
                                  nil nil cw)]
        (emit-expr ga
                   (into {}
                         (map-indexed (fn [i p]
                                        [p {:kind :arg :index i}]))
                         all-params)
                   body)
        (.returnValue ga)
        (.endMethod ga)
        (let [arity-ga (GeneratorAdapter. Opcodes/ACC_PUBLIC
                                          get-required-arity-method
                                          nil nil cw)]
          (.push arity-ga (int (count params)))
          (.returnValue arity-ga)
          (.endMethod arity-ga)))
      ;; One `invoke` method per arity clause, same class -- this is exactly
      ;; how the real host compiler emits fixed multi-arity `fn`: AFunction
      ;; already exposes independently-overridable invoke0..invoke20, and IFn
      ;; dispatch on the *call side* (host `apply`/`invoke`) already picks the
      ;; right one by argument count.
      (doseq [{:keys [params body]} arities]
        (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC
                                    (invoke-method (count params))
                                    nil nil cw)]
          (emit-expr ga
                     (into {}
                           (map-indexed (fn [i p]
                                          [p {:kind :arg :index i}]))
                           params)
                     body)
          (.returnValue ga)
          (.endMethod ga))))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)
          loader (DynamicClassLoader.
                  (.getContextClassLoader (Thread/currentThread)))
          klass (.defineClass loader class-name bytes nil)]
      {:class klass
       :class-name class-name
       :bytes bytes
       :digest (sha256-string (seq bytes))})))

(defn compile-source
  [source]
  (let [form (tiny-read source)
        expanded-form (tiny-expand form)
        ast (analyze-fn expanded-form)
        artifact (emit-class ast)
        f (.newInstance ^Class (:class artifact))]
    {:source source
     :form form
     :expanded-form expanded-form
     :ast ast
     :artifact artifact
     :fn f}))

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
    :expected 0}])

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
