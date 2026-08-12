(ns pnix.clr-meta.independent-mini-interpreter
  "Trusting-Trust (Diverse Double-Compiling) witness, distinct from
  independent_mini_backend.clj's compiler-backend approach (a DynamicMethod
  IL emitter): a from-scratch tokenizer/reader + tree-walking INTERPRETER for
  the small, environment-driven Lisp subset the gen0-2 evaluator-generation
  lane proves (`quote`/`if`/`let`/`fn`, variadic `&` rest params, named
  recursion, symbol/environment lookup and application) -- matching
  `conformance-cases` in bootstrap.clj's own 9-case corpus. Shares zero code
  with `pnix.clr-meta.bootstrap/evaluate` or `pnix.clr-meta.main`'s reader.
  Cross-validated against `bin/clr-meta -e` (the real, textual
  evaluator-generation-2 tool-eval path), not against pre-parsed Clojure data
  -- an independently-authored reader is as much a part of this witness as
  the independently-authored evaluator is.

  It is a frontier witness, not a replacement for the production evaluator:
  it covers exactly the 9-case conformance corpus's shape, not the full
  admitted portable-form surface `pnix.clr-meta.main` accepts.")

(import System.Int64)

;; ---- tiny tokenizer / reader (no clojure.core/read-string) ----

(defn- tokenize [^String source]
  (->> (re-seq #"\s*(\(|\)|\[|\]|:[^\s\(\)\[\]]+|-?\d+|&|[^\s\(\)\[\]]+)" source)
       (mapv second)))

(declare parse-one)

(defn- parse-list [tokens close]
  (loop [tokens tokens acc []]
    (cond
      (empty? tokens)
      (throw (Exception. "tiny reader: missing closing delimiter"))

      (= close (first tokens))
      [acc (rest tokens)]

      :else
      (let [[x tokens'] (parse-one tokens)]
        (recur tokens' (conj acc x))))))

(defn- parse-one [tokens]
  (let [token (first tokens)]
    (cond
      (= token "(") (let [[xs tokens'] (parse-list (rest tokens) ")")]
                      [(apply list xs) tokens'])
      (= token "[") (let [[xs tokens'] (parse-list (rest tokens) "]")]
                      [(vec xs) tokens'])
      (#{")" "]"} token)
      (throw (Exception. (str "tiny reader: unexpected " token)))

      (= token "true") [true (rest tokens)]
      (= token "false") [false (rest tokens)]
      (= token "nil") [nil (rest tokens)]

      (and (.StartsWith token ":") (> (.Length token) 1))
      [(keyword (subs token 1)) (rest tokens)]

      (re-matches #"-?\d+" token)
      [(Int64/Parse token) (rest tokens)]

      :else
      [(symbol token) (rest tokens)])))

(defn tiny-read [source]
  (let [[form rest-tokens] (parse-one (tokenize source))]
    (when (seq rest-tokens)
      (throw (Exception. "tiny reader: trailing tokens")))
    form))

;; ---- environment ----

(defn- env-lookup [env sym]
  (if (contains? env sym)
    (get env sym)
    (throw (Exception. (str "tiny interp: unbound symbol " sym)))))

;; ---- tree-walking interpreter ----
;; A `fn` value is represented as a real ClojureCLR closure: calling it with
;; argument values evaluates the body in an environment extended by binding
;; params (and an optional `&` rest param) to those values, plus -- for a
;; named fn -- the function's own name bound to itself (for recursion).

(declare ieval)

(defn- bind-params [params args env]
  (loop [params (seq params) args args env env]
    (cond
      (empty? params) env
      (= '& (first params))
      (assoc env (second params) (apply list args))
      :else
      (if (empty? args)
        (throw (Exception. "tiny interp: too few arguments"))
        (recur (rest params) (rest args) (assoc env (first params) (first args)))))))

(defn- make-closure [params body env fn-name]
  (fn [& args]
    (let [call-env (bind-params params args env)
          call-env (if fn-name
                     (assoc call-env fn-name (make-closure params body env fn-name))
                     call-env)]
      (ieval body call-env))))

(defn ieval [form env]
  (cond
    (integer? form) form
    (boolean? form) form
    (keyword? form) form
    (nil? form) nil
    (string? form) form
    (symbol? form) (env-lookup env form)
    (vector? form) (mapv #(ieval % env) form)
    (seq? form)
    (let [head (first form) args (rest form)]
      (cond
        (= head 'quote)
        (first args)

        (= head 'if)
        (let [[test-f then-f else-f] args]
          (if (ieval test-f env) (ieval then-f env) (ieval else-f env)))

        (= head 'let)
        (let [[bindings & body] args]
          (when (odd? (count bindings))
            (throw (Exception. "tiny interp: malformed let bindings")))
          (let [new-env (reduce
                          (fn [e [k v]] (assoc e k (ieval v e)))
                          env
                          (partition 2 bindings))]
            (last (mapv #(ieval % new-env) body))))

        (= head 'fn)
        (let [[maybe-name & rest-args] args
              named? (symbol? maybe-name)
              fn-name (when named? maybe-name)
              params (if named? (first rest-args) maybe-name)
              body (if named? (second rest-args) (first rest-args))]
          (make-closure params body env fn-name))

        :else
        (apply (ieval head env) (mapv #(ieval % env) args))))
    :else
    (throw (Exception. (str "tiny interp: unsupported form " form)))))

(def ^:private default-env
  "The 9-case conformance corpus's own `evaluate` starts from a truly empty
  environment and has its test harness inject `add`/`multiply`/etc.
  per-case; the real `bin/clr-meta -e` tool-eval path this witness cross-
  validates against instead resolves ordinary arithmetic/comparison/vector
  symbols against its own default bindings (confirmed live: `-e \"(+ 40
  2)\"` already works with no injected env). To compare against THAT real
  path on the corpus's own literal syntax (`+`/`-`/`*`/`<`/`vector`, not
  the corpus's placeholder names like `add`/`multiply`), this interpreter
  seeds the same small set, using real ClojureCLR host functions as trusted
  substrate -- the same honest role the JVM classfile format and the CLR
  runtime already play elsewhere in this repo's DDC witnesses."
  {'+ + '- - '* * '< < '<= <= '> > '>= >= '= = 'vector vector})

(defn compile-and-eval
  "Read `source` (a single top-level form) and evaluate it against a small
  default environment (see `default-env`)."
  [source]
  (ieval (tiny-read source) default-env))
