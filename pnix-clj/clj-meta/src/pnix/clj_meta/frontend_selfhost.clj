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
           [clojure.lang AFunction DynamicClassLoader IFn Keyword Numbers Reflector RestFn RT Symbol Util]
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
(defn- interop-method-name
  [op]
  (when (and (symbol? op) (< 1 (count (name op))) (.startsWith (name op) "."))
    (subs (name op) 1)))

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

      (interop-method-name op)
      (do
        (when (empty? args)
          (throw (ex-info "tiny analyzer: interop call needs a receiver" {:form form})))
        {:op :interop-call
         :method (interop-method-name op)
         :receiver (analyze-expr env (first args))
         :args (mapv #(analyze-expr env %) (rest args))})

      :else
      (case op
      quote (analyze-quote form args)
      do (analyze-body env args)
      let (analyze-let env form args)
      loop (analyze-loop env form args)
      recur (analyze-recur env form args)
      try
      (do
        ;; Deliberately narrow scope: exactly one body expression and one
        ;; `catch` clause (no `finally`, no multi-catch, no multi-form
        ;; try/catch bodies) -- a single-expression try/catch is enough to
        ;; cover the conformance-corpus shape this backend targets
        ;; (`(try (quot 10 x) (catch ArithmeticException e :divzero))`), and
        ;; keeps the AST/emitter change small and easy to verify.
        (when-not (= 2 (count args))
          (throw (ex-info "tiny analyzer: try requires exactly one body form and one catch clause"
                          {:form form})))
        (let [[body-form catch-form] args]
          (when-not (and (seq? catch-form) (= 'catch (first catch-form)))
            (throw (ex-info "tiny analyzer: try's second form must be a catch clause"
                            {:form form})))
          (let [[_ class-sym name-sym catch-body-form] catch-form]
            (when-not (and (= 4 (count catch-form))
                           (contains? known-exception-classes class-sym)
                           (symbol? name-sym))
              (throw (ex-info "tiny analyzer: malformed catch clause (supported classes: ArithmeticException, Exception, RuntimeException, Throwable)"
                              {:form form})))
            {:op :try
             :body (analyze-expr env body-form)
             :catch-class (get known-exception-classes class-sym)
             :catch-name name-sym
             :catch-body (analyze-expr (assoc env name-sym true) catch-body-form)})))
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
  [^GeneratorAdapter ga env {:keys [body catch-class catch-name catch-body]}]
  (let [start (Label.)
        end (Label.)
        handler (Label.)
        after (Label.)
        catch-slot (.newLocal ga obj-type)]
    (.visitTryCatchBlock ga start end handler (Type/getInternalName catch-class))
    (.mark ga start)
    (emit-expr ga env body)
    (.mark ga end)
    (.goTo ga after)
    (.mark ga handler)
    (.storeLocal ga catch-slot)
    (emit-expr ga (assoc env catch-name {:kind :let :slot catch-slot}) catch-body)
    (.mark ga after)))

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
    :new (emit-new ga env node)
    :throw (emit-throw ga env node)
    :interop-call (emit-interop-call ga env node)
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

(defn- invoke-method
  [arity]
  (Method. "invoke"
           obj-type
           (into-array Type (repeat arity obj-type))))

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
    :expected false}])

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
