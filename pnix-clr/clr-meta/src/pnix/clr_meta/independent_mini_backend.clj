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

;; ---- nested-fn desugaring (beta-reduction, no runtime closure value) ----
;;
;; This backend's whole checked-Int64 profile stays unboxed throughout --
;; there is no notion of a class (unlike the reference JVM host's
;; `emit-class`/`emit-closure` pair) and no general "call an arbitrary
;; computed value" mechanism (only the fixed operators below). Genuine
;; first-class closures (stored in a binding, applied later through an
;; unrelated path, returned across a `compile-source` boundary) would need
;; both a new runtime value representation and a general apply form -- a
;; substantially bigger step than anything else in this file, deliberately
;; NOT attempted here.
;;
;; What IS supported: a `fn` literal applied DIRECTLY, `((fn [p...] body)
;; a...)`, which is exactly the standard beta-reduction `(let [p... a...]
;; body)` -- needing no new runtime machinery at all, since it collapses
;; onto the SAME `let` analysis/emission already built and verified.
;; Arbitrary nesting/transitive capture (the shape every closure-flavored
;; fixture in this repo -- `bootstrap.clj`'s own conformance corpus,
;; `independent_mini_interpreter.clj`'s fixtures -- actually uses) falls
;; out for free by ALSO floating an application through a `let` a nested
;; reduction already produced (`((let [b...] TAIL) a...)` -> `(let [b...]
;; (TAIL a...))`, sound because a function call's ARGS are always
;; evaluated independently of the callee's own internal scope), then
;; re-running both rules until nothing more reduces. `desugar` walks
;; bottom-up (children first), so by the time either rule checks the
;; current node's operator, that operator has already been reduced as far
;; as it can be.
;;
;; Also handled: a NAMED local fn, bound via `let` and called once as the
;; let's own tail expression -- `(let [... name (fn [p...] body)] (name
;; a...))` -- the natural "define a small helper, call it right here"
;; idiom, distinct from an anonymous fn applied inline. Only that exact
;; shape (the fn-binding is the LAST binding, and the tail is a direct
;; call to that same name) is recognized: it reduces to `(let
;; [...earlier-bindings...] ((fn [p...] body) a...))`, re-desugared so the
;; inner application immediately falls to the same beta-reduction rule
;; above. A `name` called more than once, or used any other way (passed as
;; a value, etc.), is NOT inlined -- still the same first-class-closure
;; boundary noted above.
;;
;; Known, deliberately unguarded boundary: this is plain substitution, not
;; capture-avoiding substitution -- a nested `fn`'s parameter name
;; colliding with a name used in one of ITS OWN call's sibling arguments
;; (e.g. `((fn [x y] (+ x y)) 1 x)`, where the second arg `x` means the
;; OUTER `x`, not the fresh param) is not handled correctly. None of this
;; repo's actual fixtures need that shape; a real capture-avoiding pass
;; (alpha-renaming every binding to a fresh gensym first) is a well-known,
;; well-scoped follow-up if it's ever needed.
(defn- desugar [form]
  (cond
    (seq? form)
    (let [desugared (map desugar form)
          op (first desugared) args (rest desugared)]
      (cond
        (and (seq? op) (= 'fn (first op)))
        (let [[_ params & body] op]
          (when-not (and (vector? params) (every? symbol? params)
                         (= (count params) (count (distinct params)))
                         (= 1 (count body)))
            (throw (Exception. "tiny reader: malformed nested fn")))
          (when-not (= (count params) (count args))
            (throw (Exception. "tiny reader: nested fn arity mismatch")))
          (desugar (list* 'let (vec (interleave params args)) body)))

        (and (seq? op) (= 'let (first op)))
        (let [[_ bindings tail] op]
          (desugar (list 'let bindings (list* tail args))))

        (and (= op 'let) (= 2 (count args))
             (vector? (first args)) (even? (count (first args))) (seq (first args))
             (seq? (second args)))
        (let [[bindings tail] args
              pairs (partition 2 bindings)
              [last-name last-init] (last pairs)]
          (if (and (seq? last-init) (= 'fn (first last-init))
                   (= last-name (first tail)))
            (let [earlier (vec (apply concat (butlast pairs)))
                  reduced-tail (cons last-init (rest tail))]
              (desugar (if (seq earlier)
                         (list 'let earlier reduced-tail)
                         reduced-tail)))
            (apply list op args)))

        :else
        (apply list op args)))

    (vector? form) (mapv desugar form)
    :else form))

;; ---- tiny analyzer ----
;; (fn [params...] body) -> {:params [...] :body <expr>}
;; expr := int | sym | (if t then else) | (let [name val ...] body)
;;       | (loop [name val ...] body) | (recur val ...)
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
;;
;; `recur` at a `loop`'s own tail position targets that loop's bindings; a
;; bare `recur` with no enclosing `loop` targets the fn's own params
;; instead (`analyze-fn` seeds the same `recur-arity-key` env entry a
;; `loop` would) -- matching the reference JVM host's own "nearest
;; enclosing loop/fn" rule, and giving self-recursion on the top-level fn
;; for free without a separate named-fn mechanism.

(def ^:private recur-arity-key ::recur-arity)

(declare analyze-let analyze-loop analyze-recur)

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
        loop (analyze-loop env args)
        recur (analyze-recur env args)
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

(defn- analyze-loop [env args]
  (let [[bindings & body] args]
    (when-not (and (vector? bindings) (even? (count bindings)) (seq bindings)
                   (= 1 (count body)))
      (throw (Exception. "tiny analyzer: malformed loop")))
    (loop [pairs (partition 2 bindings) env env acc []]
      (if (seq pairs)
        (let [[name init] (first pairs)]
          (when-not (symbol? name)
            (throw (Exception. "tiny analyzer: loop binding name")))
          (recur (rest pairs)
                 (assoc env name true)
                 (conj acc {:name name :init (analyze-expr env init)})))
        {:op :loop
         :bindings acc
         :body (analyze-expr (assoc env recur-arity-key (count acc)) (first body))}))))

(defn- analyze-recur [env args]
  (let [arity (get env recur-arity-key)]
    (when-not arity
      (throw (Exception. "tiny analyzer: recur outside loop")))
    (when-not (= arity (count args))
      (throw (Exception. "tiny analyzer: recur arity")))
    {:op :recur :exprs (mapv #(analyze-expr env %) args)}))

(defn- analyze-fn [form]
  (when-not (and (seq? form) (= 'fn (first form)))
    (throw (Exception. "tiny analyzer: expected fn form")))
  (let [[_ params & body] form]
    (when-not (and (vector? params) (every? symbol? params) (= 1 (count body)))
      (throw (Exception. "tiny analyzer: malformed fn")))
    (let [env (assoc (zipmap params (repeat true)) recur-arity-key (count params))]
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

;; `loop` is `let` plus a `MarkLabel` right where the bindings finish and
;; the body begins, so `recur` can `Br` straight back to it. `recur-target-key`
;; carries that label plus the ordered storage targets (`loop` locals, or --
;; for a bare `recur` with no enclosing `loop` -- the fn's own arg slots,
;; set up once in `compile-source`) for `emit-recur` to reassign.
(def ^:private recur-target-key ::recur-target)

(defn- emit-loop [^System.Reflection.Emit.ILGenerator il env {:keys [bindings body]}]
  (let [[env locals]
        (loop [bindings bindings env env locals []]
          (if (seq bindings)
            (let [{:keys [name init]} (first bindings)
                  local (.DeclareLocal il Int64)]
              (emit-expr il env init)
              (.Emit il OpCodes/Stloc local)
              (recur (rest bindings)
                     (assoc env name {:kind :local :local local})
                     (conj locals {:kind :local :local local})))
            [env locals]))
        label (.DefineLabel il)]
    (.MarkLabel il label)
    (emit-expr il (assoc env recur-target-key {:label label :locals locals}) body)))

;; `recur` computes ALL new values from the OLD binding/arg values FIRST,
;; into fresh temp locals, before reassigning any of them -- matching
;; Clojure's own "recur rebinds simultaneously, not sequentially" semantics
;; (confirmed live against real host: `(loop [a 1 b 2] (if done (+ a b)
;; (recur b a)))`-style swaps must see the PRE-recur values of both, not a
;; partially-updated mix). Only after every new value is safely in a temp
;; local does it get stored into its real target (a `loop` local via
;; `Stloc`, or a fn arg slot via `Starg` -- .NET arguments are directly
;; reassignable, the same honest mechanism the reference JVM host's own
;; bare-recur-targets-the-fn's-own-arg-slots case uses).
(defn- emit-recur [^System.Reflection.Emit.ILGenerator il env exprs]
  (let [{:keys [label locals]} (get env recur-target-key)]
    (when-not label
      (throw (Exception. "tiny emitter: recur outside loop")))
    (when-not (= (count locals) (count exprs))
      (throw (Exception. "tiny emitter: recur arity")))
    (let [temps (mapv (fn [expr]
                         (let [t (.DeclareLocal il Int64)]
                           (emit-expr il env expr)
                           (.Emit il OpCodes/Stloc t)
                           t))
                       exprs)]
      (doseq [[^System.Reflection.Emit.LocalBuilder temp target] (map vector temps locals)]
        (.Emit il OpCodes/Ldloc temp)
        (case (:kind target)
          :local (.Emit il OpCodes/Stloc ^System.Reflection.Emit.LocalBuilder (:local target))
          :arg (.Emit il OpCodes/Starg (short (:index target)))))
      (.Emit il OpCodes/Br ^System.Reflection.Emit.Label label))))

(defn- emit-expr [^System.Reflection.Emit.ILGenerator il env node]
  (case (:op node)
    :const (.Emit il OpCodes/Ldc_I8 (long (:value node)))
    :local (let [{:keys [kind index local]} (get env (:name node))]
             (case kind
               :arg (.Emit il OpCodes/Ldarg (short index))
               :local (.Emit il OpCodes/Ldloc ^System.Reflection.Emit.LocalBuilder local)))
    :binary (emit-binary il env node)
    :let (emit-let il env node)
    :loop (emit-loop il env node)
    :recur (emit-recur il env (:exprs node))
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
  (let [form (desugar (tiny-read source))
        ast (analyze-fn form)
        arity (count (:params ast))
        param-types (into-array System.Type (repeat arity Int64))
        dm (DynamicMethod. "IndependentMiniBackendFn" Int64 param-types)
        il (.GetILGenerator dm)
        arg-env (into {}
                      (map-indexed (fn [i p] [p {:kind :arg :index i}]))
                      (:params ast))
        recur-label (.DefineLabel il)
        recur-locals (mapv (fn [i] {:kind :arg :index i}) (range arity))]
    (.MarkLabel il recur-label)
    (emit-expr il
               (assoc arg-env recur-target-key {:label recur-label :locals recur-locals})
               (:body ast))
    (.Emit il OpCodes/Ret)
    dm))

(defn compile-and-invoke
  "Compile `source` and invoke it with `args` (a seq of longs)."
  [source args]
  (let [dm (compile-source source)
        boxed-args (into-array Object (map #(long %) args))]
    (.Invoke dm nil boxed-args)))
