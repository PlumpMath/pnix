(ns pnix.clr-meta.independent-mini-backend
  "Tiny independent Int64-subset-to-CLR-IL emitter.

  Trusting-Trust (Diverse Double-Compiling) witness: a hand-written
  tokenizer/reader plus direct IL emission via
  System.Reflection.Emit.DynamicMethod, sharing zero code with the Compiler
  Stage1-7 family (compiler_stage1.clj, compiler_selfhost_*.clj), which uses
  System.Reflection.Emit.PersistedAssemblyBuilder to produce full PE
  executables. DynamicMethod JITs a method in memory and returns an
  invokable handle directly, without ever touching the assembly/PE-writing
  path the Stage1-7 family shares.

  The pinned ClojureCLR runtime (its own reader, its own JIT) and the CLR
  itself remain trusted host substrate here, the same honest role the JVM
  classfile format plays for the reference JVM host's analogous
  frontend_selfhost.clj and Python's ast/compile() play for the Python host's
  independent_mini_backend.py.

  It is a frontier witness, not a replacement for the Compiler Stage1-7
  family: it covers a bounded Int64 arithmetic/comparison/if/arg fixture
  set, not the checked-Int64 expression profile those stages formally close.")

(import System.Reflection.Emit.DynamicMethod
        System.Reflection.Emit.OpCodes)

;; ---- tiny tokenizer / reader (no clojure.core/read-string) ----

(defn- tokenize [^String source]
  (->> (re-seq #"\s*(\(|\)|\[|\]|-?\d+|[^\s\(\)\[\]]+)" source)
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

      (re-matches #"-?\d+" token)
      [(Int64/Parse token) (rest tokens)]

      :else
      [(symbol token) (rest tokens)])))

(defn tiny-read [source]
  (let [[form rest-tokens] (parse-one (tokenize source))]
    (when (seq rest-tokens)
      (throw (Exception. "tiny reader: trailing tokens")))
    form))

;; ---- tiny analyzer ----
;; (fn [params...] body) -> {:params [...] :body <expr>}
;; expr := int | sym | (if t then else) | (+ a b) | (- a b) | (* a b)
;;       | (< a b) | (> a b) | (<= a b) | (>= a b) | (= a b)

(defn- analyze-expr [env form]
  (cond
    (integer? form) {:op :const :value (long form)}

    (symbol? form)
    (if (contains? env form)
      {:op :arg :index (get env form)}
      (throw (Exception. (str "tiny analyzer: unknown local " form))))

    (seq? form)
    (let [op (first form) args (rest form)]
      (case op
        if (do (when-not (= 3 (count args))
                 (throw (Exception. "tiny analyzer: if arity")))
               {:op :if
                :test (analyze-expr env (nth args 0))
                :then (analyze-expr env (nth args 1))
                :else (analyze-expr env (nth args 2))})
        (+ - * < > <= >= =)
        (do (when-not (= 2 (count args))
              (throw (Exception. "tiny analyzer: binary op arity")))
            {:op :binary :fn op
             :lhs (analyze-expr env (first args))
             :rhs (analyze-expr env (second args))})
        (throw (Exception. (str "tiny analyzer: unsupported op " op)))))

    :else
    (throw (Exception. (str "tiny analyzer: unsupported form " form)))))

(defn- analyze-fn [form]
  (when-not (and (seq? form) (= 'fn (first form)))
    (throw (Exception. "tiny analyzer: expected fn form")))
  (let [[_ params & body] form]
    (when-not (and (vector? params) (every? symbol? params) (= 1 (count body)))
      (throw (Exception. "tiny analyzer: malformed fn")))
    (let [env (into {} (map-indexed (fn [i p] [p i]) params))]
      {:params params
       :body (analyze-expr env (first body))})))

;; ---- direct IL emission via DynamicMethod (no PersistedAssemblyBuilder) ----

(def ^:private CMP-BRANCH
  {'< OpCodes/Blt '> OpCodes/Bgt '<= OpCodes/Ble '>= OpCodes/Bge '= OpCodes/Beq})

(declare emit-expr)

(defn- emit-binary [^System.Reflection.Emit.ILGenerator il {:keys [fn lhs rhs]}]
  (cond
    (contains? CMP-BRANCH fn)
    (let [true-label (.DefineLabel il)
          end-label (.DefineLabel il)]
      (emit-expr il lhs)
      (emit-expr il rhs)
      (.Emit il (get CMP-BRANCH fn) true-label)
      (.Emit il OpCodes/Ldc_I8 (long 0))
      (.Emit il OpCodes/Br end-label)
      (.MarkLabel il true-label)
      (.Emit il OpCodes/Ldc_I8 (long 1))
      (.MarkLabel il end-label))

    :else
    (do
      (emit-expr il lhs)
      (emit-expr il rhs)
      (case fn
        + (.Emit il OpCodes/Add_Ovf)
        - (.Emit il OpCodes/Sub_Ovf)
        * (.Emit il OpCodes/Mul_Ovf)))))

(defn- emit-expr [^System.Reflection.Emit.ILGenerator il node]
  (case (:op node)
    :const (.Emit il OpCodes/Ldc_I8 (long (:value node)))
    :arg (.Emit il OpCodes/Ldarg (short (:index node)))
    :binary (emit-binary il node)
    :if (let [else-label (.DefineLabel il)
              end-label (.DefineLabel il)]
          (emit-expr il (:test node))
          (.Emit il OpCodes/Ldc_I8 (long 0))
          (.Emit il OpCodes/Beq else-label)
          (emit-expr il (:then node))
          (.Emit il OpCodes/Br end-label)
          (.MarkLabel il else-label)
          (emit-expr il (:else node))
          (.MarkLabel il end-label))))

(defn compile-source
  "Compile `(fn [params...] body)` source text to an invokable DynamicMethod."
  [source]
  (let [form (tiny-read source)
        ast (analyze-fn form)
        arity (count (:params ast))
        param-types (into-array System.Type (repeat arity Int64))
        dm (DynamicMethod. "IndependentMiniBackendFn" Int64 param-types)
        il (.GetILGenerator dm)]
    (emit-expr il (:body ast))
    (.Emit il OpCodes/Ret)
    dm))

(defn compile-and-invoke
  "Compile `source` and invoke it with `args` (a seq of longs)."
  [source args]
  (let [dm (compile-source source)
        boxed-args (into-array Object (map #(long %) args))]
    (.Invoke dm nil boxed-args)))
