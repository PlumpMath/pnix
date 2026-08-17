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
;; expr := int | sym | (if t then else) | (let [name val ...] body)
;;       | (+ a b) | (- a b) | (* a b)
;;       | (< a b) | (> a b) | (<= a b) | (>= a b) | (= a b)
;;
;; `env` tracks only NAME PRESENCE at analyze time (params and `let`
;; bindings both resolve to a plain `:op :local` node, matching the
;; reference JVM host's `frontend_selfhost.clj` split) -- the concrete
;; storage (an argument slot vs a declared local) is decided at EMIT time
;; instead, since .NET's `ILGenerator/DeclareLocal` (the `Ldloc`/`Stloc`
;; target) needs a live `ILGenerator` to call, unlike a param's fixed
;; `Ldarg` index which is already known from its position in `params`.

(declare analyze-let)

(defn- analyze-expr [env form]
  (cond
    (integer? form) {:op :const :value (long form)}

    (symbol? form)
    (if (contains? env form)
      {:op :local :name form}
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
        let (analyze-let env args)
        (+ - * < > <= >= =)
        (do (when-not (= 2 (count args))
              (throw (Exception. "tiny analyzer: binary op arity")))
            {:op :binary :fn op
             :lhs (analyze-expr env (first args))
             :rhs (analyze-expr env (second args))})
        (throw (Exception. (str "tiny analyzer: unsupported op " op)))))

    :else
    (throw (Exception. (str "tiny analyzer: unsupported form " form)))))

(defn- analyze-let [env args]
  (let [[bindings & body] args]
    (when-not (and (vector? bindings) (even? (count bindings)) (seq bindings)
                   (= 1 (count body)))
      (throw (Exception. "tiny analyzer: malformed let")))
    (loop [pairs (partition 2 bindings) env env acc []]
      (if (seq pairs)
        (let [[name init] (first pairs)]
          (when-not (symbol? name)
            (throw (Exception. "tiny analyzer: let binding name")))
          (recur (rest pairs)
                 (assoc env name true)
                 (conj acc {:name name :init (analyze-expr env init)})))
        {:op :let :bindings acc :body (analyze-expr env (first body))}))))

(defn- analyze-fn [form]
  (when-not (and (seq? form) (= 'fn (first form)))
    (throw (Exception. "tiny analyzer: expected fn form")))
  (let [[_ params & body] form]
    (when-not (and (vector? params) (every? symbol? params) (= 1 (count body)))
      (throw (Exception. "tiny analyzer: malformed fn")))
    (let [env (zipmap params (repeat true))]
      {:params params
       :body (analyze-expr env (first body))})))

;; ---- direct IL emission via DynamicMethod (no PersistedAssemblyBuilder) ----

(def ^:private CMP-BRANCH
  {'< OpCodes/Blt '> OpCodes/Bgt '<= OpCodes/Ble '>= OpCodes/Bge '= OpCodes/Beq})

(declare emit-expr)

(defn- emit-binary [^System.Reflection.Emit.ILGenerator il env {:keys [fn lhs rhs]}]
  (cond
    (contains? CMP-BRANCH fn)
    (let [true-label (.DefineLabel il)
          end-label (.DefineLabel il)]
      (emit-expr il env lhs)
      (emit-expr il env rhs)
      (.Emit il (get CMP-BRANCH fn) true-label)
      (.Emit il OpCodes/Ldc_I8 (long 0))
      (.Emit il OpCodes/Br end-label)
      (.MarkLabel il true-label)
      (.Emit il OpCodes/Ldc_I8 (long 1))
      (.MarkLabel il end-label))

    :else
    (do
      (emit-expr il env lhs)
      (emit-expr il env rhs)
      (case fn
        + (.Emit il OpCodes/Add_Ovf)
        - (.Emit il OpCodes/Sub_Ovf)
        * (.Emit il OpCodes/Mul_Ovf)))))

;; A `let` binding's storage is an honest `Int64` local declared right on
;; this method's own `ILGenerator` (`DeclareLocal` + `Stloc`/`Ldloc`) --
;; the direct .NET analogue of a JVM local variable slot, no boxing
;; involved (this backend's whole checked-Int64 profile stays unboxed
;; throughout, matching `independent_mini_backend.clj`'s own scope note).
(defn- emit-let [^System.Reflection.Emit.ILGenerator il env {:keys [bindings body]}]
  (loop [bindings bindings env env]
    (if (seq bindings)
      (let [{:keys [name init]} (first bindings)
            local (.DeclareLocal il Int64)]
        (emit-expr il env init)
        (.Emit il OpCodes/Stloc local)
        (recur (rest bindings) (assoc env name {:kind :local :local local})))
      (emit-expr il env body))))

(defn- emit-expr [^System.Reflection.Emit.ILGenerator il env node]
  (case (:op node)
    :const (.Emit il OpCodes/Ldc_I8 (long (:value node)))
    :local (let [{:keys [kind index local]} (get env (:name node))]
             (case kind
               :arg (.Emit il OpCodes/Ldarg (short index))
               :local (.Emit il OpCodes/Ldloc ^System.Reflection.Emit.LocalBuilder local)))
    :binary (emit-binary il env node)
    :let (emit-let il env node)
    :if (let [else-label (.DefineLabel il)
              end-label (.DefineLabel il)]
          (emit-expr il env (:test node))
          (.Emit il OpCodes/Ldc_I8 (long 0))
          (.Emit il OpCodes/Beq else-label)
          (emit-expr il env (:then node))
          (.Emit il OpCodes/Br end-label)
          (.MarkLabel il else-label)
          (emit-expr il env (:else node))
          (.MarkLabel il end-label))))

(defn compile-source
  "Compile `(fn [params...] body)` source text to an invokable DynamicMethod."
  [source]
  (let [form (tiny-read source)
        ast (analyze-fn form)
        arity (count (:params ast))
        param-types (into-array System.Type (repeat arity Int64))
        dm (DynamicMethod. "IndependentMiniBackendFn" Int64 param-types)
        il (.GetILGenerator dm)
        arg-env (into {}
                      (map-indexed (fn [i p] [p {:kind :arg :index i}]))
                      (:params ast))]
    (emit-expr il arg-env (:body ast))
    (.Emit il OpCodes/Ret)
    dm))

(defn compile-and-invoke
  "Compile `source` and invoke it with `args` (a seq of longs)."
  [source args]
  (let [dm (compile-source source)
        boxed-args (into-array Object (map #(long %) args))]
    (.Invoke dm nil boxed-args)))
