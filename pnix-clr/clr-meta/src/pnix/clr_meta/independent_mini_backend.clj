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
        System.Reflection.Emit.OpCodes
        System.Reflection.Emit.AssemblyBuilder
        System.Reflection.Emit.AssemblyBuilderAccess
        System.Reflection.AssemblyName
        System.Reflection.TypeAttributes
        System.Reflection.FieldAttributes
        System.Reflection.MethodAttributes
        System.Reflection.CallingConventions)

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
;; The ONE exception: a `let` binding whose init is a `fn` literal that
;; `desugar` couldn't eliminate (see below) is CLOSURE-typed, not
;; Int64-typed -- `env` marks those with `{:closure-arity N}` instead of
;; plain `true`, since a closure-valued name can only ever be CALLED
;; (`:op :closure-call`), never used as an ordinary Int64 expression.
;;
;; `recur` at a `loop`'s own tail position targets that loop's bindings; a
;; bare `recur` with no enclosing `loop` targets the fn's own params
;; instead (`analyze-fn` seeds the same `recur-arity-key` env entry a
;; `loop` would) -- matching the reference JVM host's own "nearest
;; enclosing loop/fn" rule, and giving self-recursion on the top-level fn
;; for free without a separate named-fn mechanism.

(def ^:private recur-arity-key ::recur-arity)

(declare analyze-let analyze-loop analyze-recur analyze-closure-fn ast-contains-closure?)

(defn- closure-literal? [form]
  (and (seq? form) (= 'fn (first form))))

(defn- analyze-expr [env form]
  (cond
    (integer? form) {:op :const :value (long form)}

    (symbol? form)
    (let [entry (get env form ::not-found)]
      (cond
        (= entry ::not-found)
        (throw (Exception. (str "tiny analyzer: unknown local " form)))

        (map? entry)
        (throw (Exception. (str "tiny analyzer: closure value " form " used without being called")))

        :else {:op :local :name form}))

    (seq? form)
    (let [op (first form) args (rest form)]
      (if (and (symbol? op) (map? (get env op)))
        (let [arity (:closure-arity (get env op))]
          (when-not (= arity (count args))
            (throw (Exception. "tiny analyzer: closure call arity mismatch")))
          {:op :closure-call :name op :args (mapv #(analyze-expr env %) args)})
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
          (throw (Exception. (str "tiny analyzer: unsupported op " op))))))

    :else
    (throw (Exception. (str "tiny analyzer: unsupported form " form)))))

;; A raw `:op :local` AST-node walk (mirroring the reference JVM host's
;; `ast-referenced-names`) -- used to find a closure's free variables
;; AFTER analyzing its body, since `env` carries no distinction between
;; "this closure's own param" and "resolved from the enclosing scope" (see
;; `analyze-fn-clause`'s JVM-host analogue for the same reasoning).
(defn- ast-referenced-names [node]
  (cond
    (and (map? node) (= :local (:op node))) #{(:name node)}
    (map? node) (reduce into #{} (map ast-referenced-names (vals node)))
    (sequential? node) (reduce into #{} (map ast-referenced-names node))
    :else #{}))

;; A nested `fn` literal `desugar` couldn't eliminate (called more than
;; once, called from a non-tail position, or otherwise escaping) -- a
;; GENUINE closure, needing a real runtime value (see the emitter section
;; below for how). Deliberately narrow: exactly ONE param (no multi-arity,
;; no variadic), and every captured free variable must itself be an
;; ordinary Int64 binding -- capturing ANOTHER closure is not supported
;; (this backend never needed transitive closure-of-closure capture for
;; any actual fixture, and it would need the base closure type itself to
;; be capturable, a real further extension). Also deliberately rejected: a
;; nested closure literal INSIDE this closure's own body -- `ast-referenced-
;; names` cannot distinguish "an inner closure's own param" from "a name
;; free in the outer closure" by AST shape alone (both are plain `:op
;; :local` nodes), so allowing it would risk silently mis-capturing the
;; inner closure's param as an outer free variable. No fixture needs
;; closures nested this way; a real fix would need `ast-referenced-names`
;; to stop descending at nested `:closure` boundaries.
(defn- analyze-closure-fn [env form]
  (let [[_ params & body] form]
    (when-not (and (vector? params) (= 1 (count params)) (symbol? (first params))
                   (= 1 (count body)))
      (throw (Exception. "tiny analyzer: nested fn literal must have exactly one param (multi-arity/variadic closures not supported)")))
    (let [param (first params)
          body-ast (analyze-expr (assoc env param true) (first body))]
      (when (ast-contains-closure? body-ast)
        (throw (Exception. "tiny analyzer: a closure literal nested inside another closure's body is not supported")))
      (let [captures (vec (disj (ast-referenced-names body-ast) param))]
        (doseq [cap captures]
          (when (map? (get env cap))
            (throw (Exception. (str "tiny analyzer: capturing a closure value (" cap ") is not supported")))))
        {:op :closure :param param :body body-ast :captures captures}))))

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
          (if (closure-literal? init)
            (let [closure-ast (analyze-closure-fn env init)]
              (recur (rest pairs)
                     (assoc env name {:closure-arity 1})
                     (conj acc {:name name :kind :closure :closure closure-ast})))
            (recur (rest pairs)
                   (assoc env name true)
                   (conj acc {:name name :kind :int64 :init (analyze-expr env init)}))))
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

;; Whether `ast` contains any genuine closure (a `let`-bound `fn` literal
;; `desugar` could not beta-reduce away) -- `compile-source` uses this to
;; decide between the cheap, unconditionally-kept `DynamicMethod` path (no
;; closures: every existing fixture keeps working through the exact same
;; code as before) and the TypeBuilder-hosted static-method path a real
;; closure value requires (see the emitter section below for why).
(defn- ast-contains-closure? [node]
  (cond
    (and (map? node) (contains? #{:closure :closure-call} (:op node))) true
    (map? node) (boolean (some ast-contains-closure? (vals node)))
    (sequential? node) (boolean (some ast-contains-closure? node))
    :else false))

;; ---- direct IL emission via DynamicMethod / TypeBuilder ----
;;
;; Closures need a real runtime value (a heap object holding its captured
;; Int64s plus an `Invoke` method), which forces a SECOND, TypeBuilder-based
;; emission path alongside the original DynamicMethod one:
;; `System.Reflection.Emit.DynamicMethod`'s IL generator cannot reference
;; `TypeBuilder`-created constructors/methods on this platform ("MethodInfo/
;; ConstructorInfo must be a runtime ... object"), even though the identical
;; IL sequence works when the CALLING method is itself TypeBuilder-hosted --
;; verified end-to-end during this feature's development. So: a program
;; with no closures anywhere keeps compiling through the original,
;; unmodified `DynamicMethod` path (zero regression risk on all pre-existing
;; fixtures); a program containing a closure compiles its whole top-level fn
;; as a TypeBuilder-hosted static method instead, so its IL generator can
;; freely reference the closure classes' constructors/`Invoke` methods
;; defined alongside it in the same dynamic module. Both paths share
;; `emit-top-level-body!` for the actual body emission.

(def ^:private CMP-BRANCH
  {'< OpCodes/Blt '> OpCodes/Bgt '<= OpCodes/Ble '>= OpCodes/Bge '= OpCodes/Beq})

(declare emit-expr)

;; Bound once per `compile-source` call that contains a closure, to the
;; shared `ModuleBuilder` every closure class (and the top-level container
;; type) gets defined into -- unbound (and unused) on the plain
;; `DynamicMethod` path, since no closure class is ever created there.
(def ^:dynamic *closure-module* nil)

(def ^:private public-static-method-attrs
  (System.Enum/ToObject MethodAttributes (bit-or (long MethodAttributes/Public) (long MethodAttributes/Static))))

;; A name's current value can live in an arg slot, a declared local, or (for
;; code inside a closure's own `Invoke` body) a capture field read off `this`
;; -- this is the one place that distinction is resolved, shared by both the
;; plain `:local` AST case and closure construction's "push each capture's
;; current value" step.
(defn- emit-local-value! [^System.Reflection.Emit.ILGenerator il env name]
  (let [{:keys [kind index local field]} (get env name)]
    (case kind
      :arg (.Emit il OpCodes/Ldarg (short index))
      :local (.Emit il OpCodes/Ldloc ^System.Reflection.Emit.LocalBuilder local)
      :field (do (.Emit il OpCodes/Ldarg_0)
                 (.Emit il OpCodes/Ldfld field)))))

;; Defines ONE new TypeBuilder class per closure literal into
;; `*closure-module*`: one public Int64 field per capture, a public
;; constructor storing each capture param into its field, and a public
;; (non-virtual -- there is never more than one concrete type behind any
;; given closure-local, so no vtable dispatch is needed) `Invoke(long):long`
;; implementing the body, with the param at arg index 1 (index 0 is `this`)
;; and captures read via `emit-local-value!`'s `:field` case. `.CreateType`
;; is called immediately, before returning -- the type is always fully
;; finished before anything else can reference it.
(defn- emit-closure! [{:keys [param body captures]} env]
  (let [mod *closure-module*
        closure-type (.DefineType mod (str "IndependentMiniBackendClosure_" (gensym)) TypeAttributes/Public)
        capture-fields (mapv (fn [cap] [cap (.DefineField closure-type (str cap) Int64 FieldAttributes/Public)])
                              captures)
        ctor (.DefineConstructor closure-type MethodAttributes/Public CallingConventions/Standard
                                  (into-array System.Type (repeat (count captures) Int64)))
        ctor-il (.GetILGenerator ctor)]
    (.Emit ctor-il OpCodes/Ldarg_0)
    (.Emit ctor-il OpCodes/Call (.GetConstructor System.Object (into-array System.Type [])))
    (doseq [[i [_ field]] (map-indexed vector capture-fields)]
      (.Emit ctor-il OpCodes/Ldarg_0)
      (.Emit ctor-il OpCodes/Ldarg (short (inc i)))
      (.Emit ctor-il OpCodes/Stfld field))
    (.Emit ctor-il OpCodes/Ret)
    (let [invoke (.DefineMethod closure-type "Invoke" MethodAttributes/Public
                                 Int64 (into-array System.Type [Int64]))
          inv-il (.GetILGenerator invoke)
          field-env (into {} (map (fn [[cap field]] [cap {:kind :field :field field}])) capture-fields)
          body-env (assoc field-env param {:kind :arg :index 1})]
      (emit-expr inv-il body-env body)
      (.Emit inv-il OpCodes/Ret)
      (.CreateType closure-type)
      {:type closure-type :ctor ctor :invoke invoke :captures (mapv first capture-fields)})))

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
      (let [{:keys [name kind init closure]} (first bindings)]
        (case kind
          :int64
          (let [local (.DeclareLocal il Int64)]
            (emit-expr il env init)
            (.Emit il OpCodes/Stloc local)
            (recur (rest bindings) (assoc env name {:kind :local :local local})))

          :closure
          (let [{:keys [type ctor invoke captures]} (emit-closure! closure env)
                local (.DeclareLocal il type)]
            (doseq [cap captures] (emit-local-value! il env cap))
            (.Emit il OpCodes/Newobj ctor)
            (.Emit il OpCodes/Stloc local)
            (recur (rest bindings) (assoc env name {:kind :closure-local :local local :invoke invoke})))))
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
    :local (emit-local-value! il env (:name node))
    :binary (emit-binary il env node)
    :let (emit-let il env node)
    :loop (emit-loop il env node)
    :recur (emit-recur il env (:exprs node))
    :closure-call
    (let [{:keys [local invoke]} (get env (:name node))]
      (.Emit il OpCodes/Ldloc ^System.Reflection.Emit.LocalBuilder local)
      (doseq [arg (:args node)] (emit-expr il env arg))
      (.Emit il OpCodes/Call invoke))
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

;; Shared by both the `DynamicMethod` path and the TypeBuilder-hosted
;; closure path: mark the recur-back label, emit the body against an env
;; that already carries the arg bindings plus the recur target, and return.
(defn- emit-top-level-body! [^System.Reflection.Emit.ILGenerator il arg-env recur-label recur-locals body]
  (.MarkLabel il recur-label)
  (emit-expr il (assoc arg-env recur-target-key {:label recur-label :locals recur-locals}) body)
  (.Emit il OpCodes/Ret))

(defn- compile-source-dynamic-method [ast]
  (let [arity (count (:params ast))
        param-types (into-array System.Type (repeat arity Int64))
        dm (DynamicMethod. "IndependentMiniBackendFn" Int64 param-types)
        il (.GetILGenerator dm)
        arg-env (into {}
                      (map-indexed (fn [i p] [p {:kind :arg :index i}]))
                      (:params ast))
        recur-label (.DefineLabel il)
        recur-locals (mapv (fn [i] {:kind :arg :index i}) (range arity))]
    (emit-top-level-body! il arg-env recur-label recur-locals (:body ast))
    dm))

;; The closure-carrying path: the whole top-level fn becomes a public
;; static method on a fresh TypeBuilder-hosted container type in a fresh
;; Run-only dynamic assembly/module, so its IL generator can freely
;; reference the closure classes `emit-closure!` defines alongside it (see
;; the emitter section's opening note for why `DynamicMethod` can't do
;; this). `.Invoke(null, args)` on the resulting static `MethodInfo` behaves
;; identically to `.Invoke(null, args)` on a `DynamicMethod` from
;; `compile-and-invoke`'s point of view -- both are plain `MethodInfo`.
(defn- compile-source-with-closures [ast]
  (let [arity (count (:params ast))
        asm (AssemblyBuilder/DefineDynamicAssembly
             (AssemblyName. "IndependentMiniBackendClosureAsm") AssemblyBuilderAccess/Run)
        mod (.DefineDynamicModule asm "IndependentMiniBackendClosureMod")
        container (.DefineType mod "IndependentMiniBackendContainer" TypeAttributes/Public)
        invoke-mb (.DefineMethod container "TopLevelInvoke" public-static-method-attrs
                                  Int64 (into-array System.Type (repeat arity Int64)))
        il (.GetILGenerator invoke-mb)
        arg-env (into {}
                      (map-indexed (fn [i p] [p {:kind :arg :index i}]))
                      (:params ast))
        recur-label (.DefineLabel il)
        recur-locals (mapv (fn [i] {:kind :arg :index i}) (range arity))]
    (binding [*closure-module* mod]
      (emit-top-level-body! il arg-env recur-label recur-locals (:body ast)))
    (let [created (.CreateType container)]
      (.GetMethod created "TopLevelInvoke"))))

(defn compile-source
  "Compile `(fn [params...] body)` source text to an invokable MethodInfo:
  a plain `DynamicMethod` when the program has no closures (unchanged from
  before this feature existed), or a TypeBuilder-hosted static method when
  it does (see `compile-source-with-closures`)."
  [source]
  (let [form (desugar (tiny-read source))
        ast (analyze-fn form)]
    (if (ast-contains-closure? ast)
      (compile-source-with-closures ast)
      (compile-source-dynamic-method ast))))

(defn compile-and-invoke
  "Compile `source` and invoke it with `args` (a seq of longs)."
  [source args]
  (let [dm (compile-source source)
        boxed-args (into-array Object (map #(long %) args))]
    (.Invoke dm nil boxed-args)))
