(ns pnix-clj.machine
  "M7 — pnix abstract machine, DERIVED from the definitional tree-walk
  evaluator (pnix-clj.evaluator/eval-ast*) by the functional correspondence
  (Ager/Biernacki/Danvy/Midtgaard, PPDP'03): closure conversion + CPS +
  defunctionalization of the continuations. Control is an explicit
  heap-allocated frame stack (CEK shape), so the machine runs in CONSTANT JVM
  stack; call-by-need is the Krivine + memoizing-store refinement — forcing a
  pending thunk pushes an :update frame and enters the thunk's expression in
  the SAME loop, and the update frame memoizes the result into the shared
  thunk store (the evaluator's own thunk representation, so boundary helpers
  force machine thunks transparently).

  The correspondence transforms CONTROL ONLY: every value-level semantic
  (binary operators, strict bool positions, unary ops, interpolation
  coercion) is the evaluator's own public value algebra — shared definitions,
  never copies, so the two lanes cannot drift on value semantics.

  Machine states (one flat loop, four modes):
    :eval   — enter an AST node under an environment
    :value  — return a WHNF value to the top control frame
    :force  — enter a thunk (memoizing store protocol; :update frame)
    :unwind — propagate a held result outward, memoizing pending :update
              frames; `or` default is applied in select-transition after a
              WHNF target (nested intermediate missing-attr propagates)

  Fragment (v3, honest): literals, var (incl. __curPos), list, let
  (recursive, lazy, incl. plain inherit), attrset (rec included; static keys
  only), select (attrpath + `or` default + nullary-builtin finish), has-attr,
  if, assert, with, string templates, ! and unary minus, every binary
  operator, lambda (simple param) and application — RUN UNDER THE DEFAULT ENV
  (M7c): builtins resolve as vars and via `builtins.X`, and their application
  delegates whole to the evaluator's public apply-callable (partial
  application, the D2 lazy-argument positions, finish — one definition, zero
  drift; the builtin's INTERNALS run on the evaluator's own recursion, a
  hand-placed value-algebra boundary in the weval sense, not machine
  control). tryEval is a [:try-eval] frame: the :unwind mode catches exactly
  the D3 catchable reasons (throw/assert), everything else stays uncatchable.
  An unbound name holds :unbound-var at run time exactly like the evaluator.
  Dynamic attr keys, dotted-path attr bindings with dynamic segments, param
  patterns, import, paths and everything else refuse statically as
  :machine-unsupported-op — a graceful hold, never a guess.

  Why this exists (2026-07-08 deep-research verdict, docs/REMAINING_DECISION.md):
  D1c's deep NON-TAIL shapes (binary left-spine chains, nested list literals)
  overflow any tree-walk — including real Nix's, machine-dependently (issue
  #9627) — so the conformance answer stays the graceful bound. The machine is
  PILLAR work: the derivation itself is the metacircular artifact, and it
  evaluates those exact shapes in constant JVM stack as a by-product.
  Refuted shortcuts (0-3, adversarial verification): clojure.core/trampoline
  and store-allocated-continuation framings — this is neither; it is the
  classical defunctionalized-continuation machine with a memoizing store."
  (:require [pnix-clj.error :as err]
            [pnix-clj.evaluator :as ev]
            [pnix-clj.parser :as parser]))

(def lane-classification
  {:lane :proof-only
   :scope :derived-abstract-machine
   :product-runtime :forbidden
   :semantic-authority :evaluator-shared-value-algebra-only
   :derivation :functional-correspondence-defunctionalized-continuations
   :laziness :krivine-memoizing-store
   :fragment :closed-lexical-core-static-refusal
   :mutation :forbidden
   :admission :forbidden
   :allowed-output :whnf-or-realized-eval-result})

;; ── fragment ──────────────────────────────────────────────────────────────

(def supported-ops
  "AST ops the machine executes. Everything else refuses statically."
  #{:int :float :bool :null :string :path :var :list :let :if :not :neg
    :binary :lambda :call :attrset :select :has-attr :assert :with
    :string-template :import})

(defn- static-attr-key?
  [k]
  (string? k))

(defn- dynamic-key?
  [k]
  (and (map? k) (= :dynamic-attr-key (:kind k))))

(defn- children
  [node]
  (case (:op node)
    :list (:items node)
    :let (conj (mapv :value (:bindings node)) (:body node))
    :attrset (into (mapv :value (:attrs node))
                   (comp (mapcat (fn [b] (if-let [p (:path b)]
                                           p
                                           [(:key b)])))
                         (filter dynamic-key?)
                         (map :expr))
                   (:attrs node))
    :select (cond-> [(:target node)]
              (dynamic-key? (:attr node)) (conj (:expr (:attr node)))
              (:default node) (conj (:default node)))
    :has-attr (cond-> [(:target node)]
                (dynamic-key? (:attr node)) (conj (:expr (:attr node))))
    :if [(:condition node) (:then node) (:else node)]
    :assert [(:condition node) (:body node)]
    :with [(:env-expr node) (:body node)]
    :string-template (into [] (comp (filter #(= :expr (:kind %)))
                                    (map :expr))
                           (:parts node))
    (:not :neg) [(:expr node)]
    :binary [(:left node) (:right node)]
    :lambda (into [(:body node)]
                  (keep :default)
                  (:params (:param-pattern node)))
    :call [(:fn node) (:arg node)]
    []))

(defn unsupported-node
  "First fragment violation in ast, or nil when the whole tree is inside the
  machine's fragment. Since M7c the machine environment IS the default env
  (builtins included), so variables need no static analysis at all — an
  unbound name holds :unbound-var at run time exactly like the evaluator.
  Static refusal is mainly defensive non-simple let bindings (name not a
  string or value not a map AST) — the parser no longer produces those after
  D22. :path / :import / dynamic keys / path attrs are in supported-ops.
  Iterative walk (explicit node stack)."
  [ast]
  (loop [stack [ast]]
    (if (empty? stack)
      nil
      (let [node (peek stack)
            stack (pop stack)
            op (:op node)]
        (cond
          (not (contains? supported-ops op))
          {:op op}

          (and (= :let op)
               (some (fn [b] (not (and (string? (:name b)) (map? (:value b)))))
                     (:bindings node)))
          {:op :let :detail :non-simple-binding}

          :else
          (recur (into stack (children node))))))))

;; ── the memoizing thunk store (the evaluator's own thunk shape) ───────────

(declare run-whnf)

(defn- thunk?
  [v]
  (and (map? v) (= :thunk (:kind v))))

(defn- menv
  "A machine thunk's captured environment: a plain map, or an atom for the
  `let`/`rec` knot (dereferenced at force time, after the knot is tied)."
  [t]
  (let [e (::menv t)]
    (if (instance? clojure.lang.Atom e) @e e)))

(defn- mthunk
  "A machine thunk IS an evaluator thunk (same shape, same state protocol), so
  evaluator-side helpers (nix-equal, interpolation coercion, deep force at
  foreign boundaries) force it transparently through :compute; the machine
  itself forces it in-loop via ::mast/::menv with an :update frame instead —
  no JVM recursion."
  [label ast env-or-ref]
  (let [t {:kind :thunk
           :label label
           :state (atom {:phase :pending})
           ::mast ast
           ::menv env-or-ref}]
    (assoc t :compute (fn [] (run-whnf ast (menv t))))))

;; The with-scope chain rides in the machine env under a namespaced key
;; (mirrors the evaluator's ::with-scopes; string binding names can never
;; collide with it). Closures capture it with the env, exactly like the
;; evaluator's closures do.
(def ^:private scopes-key ::with-scopes)

;; ── the machine ───────────────────────────────────────────────────────────

(defn- select-transition
  "eval-select's decision on a WHNF target and a RESOLVED string key, as a
  transition map for the machine loop (shared by the static-key and
  dynamic-key frame arms so the two cannot drift)."
  [targetv k default fenv env kont]
  (cond
    (and (map? targetv) (contains? targetv k))
    (let [slot (get targetv k)]
      {:mode (if (thunk? slot) :force :value)
       :x slot :env env :kont (conj kont [:select-finish])})

    default
    {:mode :eval :x default :env fenv :kont kont}

    (not (map? targetv))
    {:mode :unwind
     :x (err/failed :eval :select-target-not-attrset
                  {:attr k :target-value targetv})
     :env env :kont kont}

    :else
    {:mode :unwind
     :x (err/failed :eval :missing-attr
                  {:attr k :available-attrs (vec (sort (keys targetv)))})
     :env env :kont kont}))

(declare attrs-path-transition)

(defn- attrs-transition
  "Next step of frame-driven attrset construction (the mixed static/dynamic
  key path, D20 semantics): consume static-key bindings inline with the
  duplicate check; stop at the first dynamic key (its expression evaluation
  gets an [:attrs-key] frame), route a dotted-PATH binding (M7i — surviving
  paths have dynamic segments, D10 nests fully-static ones at parse) through
  attrs-path-transition, or finish with the values map. Mirrors eval-attrs'
  loop: rec threads env' + env-ref per binding."
  [bindings values env' rec? env-ref enclosing env kont]
  (loop [bindings bindings
         values values
         env' env']
    (if-let [{:keys [key value from-enclosing path] :as b} (first bindings)]
      (cond
        path
        (attrs-path-transition b (rest bindings) [] path values env'
                               rec? env-ref enclosing env kont)

        (string? key)
        (if (contains? values key)
          {:mode :unwind
           :x (err/failed :eval :duplicate-attr {:attr key})
           :env env :kont kont}
          (let [slot (if rec?
                       ;; rec static slots were pre-bound (base-env prebind)
                       (get env' key)
                       (mthunk key value (if from-enclosing enclosing env')))
                env'' (if rec? (assoc env' key slot) env')]
            (when rec? (reset! env-ref env''))
            (recur (rest bindings) (assoc values key slot) env'')))

        :else
        {:mode :eval :x (:expr key) :env env'
         :kont (conj kont [:attrs-key b (rest bindings) values env'
                           rec? env-ref enclosing])})
      {:mode :value :x values :env env :kont kont})))

(defn- attrs-path-transition
  "Resolve a dotted-path binding's key segments (eval-attrs' path branch,
  M7i): static string segments are consumed inline; a dynamic segment's
  expression evaluates under an [:attrs-path] frame (D20 string check on the
  value). When every segment is resolved, the slot merges via the shared
  merge-attr-path — an [:err k] is the :duplicate-attr held with the same
  data as the host's — and rec threads the TOP key into env'/env-ref exactly
  like eval-attrs. NOTE (D21 filed): the host evaluates path keys EAGERLY at
  construction and holds literal↔path collisions; real Nix defers dynamic
  sub-keys and merges — the machine mirrors the HOST bug-for-bug until D21
  lands on the evaluator."
  [b rest-bindings done segs values env' rec? env-ref enclosing env kont]
  (loop [done done
         segs segs]
    (if-let [seg (first segs)]
      (if (string? seg)
        (recur (conj done seg) (rest segs))
        {:mode :eval :x (:expr seg) :env env'
         :kont (conj kont [:attrs-path b rest-bindings done (rest segs)
                           values env' rec? env-ref enclosing])})
      ;; all segments resolved — mirror eval-attrs' merge
      (let [slot (mthunk [:attr-path done] (:value b) env')
            [st merged] (ev/merge-attr-path values done (:path-spans b) slot)]
        (if (= st :err)
          {:mode :unwind
           :x (err/failed :eval :duplicate-attr {:attr merged :path done})
           :env env :kont kont}
          (let [top (first done)
                env'' (if rec? (assoc env' top (get merged top)) env')]
            (when rec? (reset! env-ref env''))
            (attrs-transition rest-bindings merged env''
                              rec? env-ref enclosing env kont)))))))

(defn- consume-text-parts
  "Consume leading :text template parts into chunks; returns
  [remaining-parts chunks] with remaining starting at the next :expr part
  (or empty)."
  [parts chunks]
  (loop [remaining parts
         chunks chunks]
    (if (and (seq remaining) (= :text (:kind (first remaining))))
      (recur (rest remaining) (conj chunks (:value (first remaining))))
      [remaining chunks])))

(defn- join-template-chunks
  "The evaluator's template joiner verbatim: contents concatenate, contexts
  union (a context-free template stays a plain String)."
  [chunks]
  (ev/ctx-string (apply str (map ev/string-content chunks))
                 (into [] (mapcat ev/string-ctx) chunks)))

(defn run-whnf
  "Run the machine to weak-head normal form. One flat loop; control frames
  live in `kont` (a vector used as a stack) on the heap. Returns an eval
  result map; list/attrset values carry machine thunks exactly like the
  evaluator's."
  [ast env]
  (loop [mode :eval
         x ast
         env env
         kont []]
    (case mode
      ;; ── enter an AST node ────────────────────────────────────────────
      :eval
      (do
        ;; M7g fuel parity: one tick per node ENTRY, the same volatile and
        ;; the same tagged throw as eval-ast*, so safe-eval-style budgets
        ;; bound the machine identically (an approximate step-count match,
        ;; honestly not byte-identical — the loop has :value/:force steps
        ;; eval-ast* does not count).
        (when-let [f ev/*fuel*]
          (when (neg? (long (vswap! f unchecked-dec)))
            (throw (ex-info "pnix fuel exhausted" {:pnix-fuel-exhausted true}))))
        (case (:op x)
        (:int :float :bool :null :string)
        (recur :value (:value x) env kont)

        :path
        (recur :value (ev/path-value (:value x)) env kont)

        :import
        ;; The evaluator's :import arm verbatim — the resolver is a SHARED
        ;; dynamic seam (core binds it for in-memory module maps); resolving
        ;; runs whatever the resolver runs, a hand-placed boundary like
        ;; builtin internals. Unwired, both lanes hold identically.
        (if ev/*import-resolver*
          (let [r (ev/*import-resolver* (ev/import-resolver-context env)
                                        (:target x)
                                        nil)]
            (if (= :ok (:status r))
              (if (thunk? (:value r))
                (recur :force (:value r) env kont)
                (recur :value (:value r) env kont))
              (recur :unwind r env kont)))
          (recur :unwind
                 (err/failed :eval
                           :import-evaluation-not-wired
                           {:target (:target x)})
                 env kont))

        :var
        (let [name (:name x)]
          (cond
            ;; The evaluator's __curPos special case wins even over a lexical
            ;; shadow — mirrored bug-for-bug through the shared definition.
            (= "__curPos" name)
            (recur :value (ev/source-position (:span x)) env kont)

            (contains? env name)
            (let [v (get env name)]
              (if (thunk? v)
                (recur :force v env kont)
                (recur :value v env kont)))

            :else
            (if-let [scope (some (fn [attrs] (when (contains? attrs name) attrs))
                                 (get env scopes-key))]
              (let [v (get scope name)]
                (if (thunk? v)
                  (recur :force v env kont)
                  (recur :value v env kont)))
              (recur :unwind
                     (err/failed :eval :unbound-var {:name name})
                     env kont))))

        :list
        (recur :value
               (vec (map-indexed (fn [i item] (mthunk [:list i] item env))
                                 (:items x)))
               env kont)

        :let
        ;; The evaluator's recursive-let semantics verbatim: every binding is
        ;; a memoized thunk evaluating in the final env (knot via env-ref);
        ;; plain `inherit` resolves against the enclosing env. The body stays
        ;; in the loop with no frame — let CHAINS are constant-stack here for
        ;; free, where eval-let needed a dedicated tail loop.
        (let [enclosing env
              env-ref (atom nil)
              final-env (reduce (fn [acc {:keys [name value from-enclosing]}]
                                  (assoc acc name
                                         (if from-enclosing
                                           (mthunk name value enclosing)
                                           (mthunk name value env-ref))))
                                env
                                (:bindings x))]
          (reset! env-ref final-env)
          (recur :eval (:body x) final-env kont))

        :attrset
        ;; All-static attrsets build in one step (no duplicate is possible —
        ;; static-static collisions are parse errors since D10): rec
        ;; pre-binds every key as a thunk evaluating in the knot-tied env
        ;; where the siblings are visible as vars (eval-attrs parity); plain
        ;; inherit copies from the enclosing env. A DYNAMIC key (M7e, D20
        ;; semantics) switches to the frame-driven path: static prefix
        ;; consumed inline, each dynamic key expression evaluated under an
        ;; [:attrs-key] frame with the string check and the duplicate check.
        (let [enclosing env
              env-ref (atom nil)
              rec? (:recursive x)
              attrs (:attrs x)]
          (if (every? (fn [b] (string? (:key b))) attrs)
            (let [values (reduce (fn [acc {:keys [key value from-enclosing]}]
                                   (assoc acc key
                                          (mthunk key value
                                                  (cond
                                                    from-enclosing enclosing
                                                    rec? env-ref
                                                    :else enclosing))))
                                 {}
                                 attrs)]
              (when rec?
                (reset! env-ref (merge env values)))
              (recur :value values env kont))
            (let [base-env (if rec?
                             ;; host parity: rec pre-binds every STATIC key
                             (reduce (fn [acc {:keys [key value from-enclosing]}]
                                       (if (string? key)
                                         (assoc acc key
                                                (mthunk key value
                                                        (if from-enclosing
                                                          enclosing
                                                          env-ref)))
                                         acc))
                                     env attrs)
                             env)
                  _ (reset! env-ref base-env)
                  t (attrs-transition attrs {} base-env rec? env-ref
                                      enclosing env kont)]
              (recur (:mode t) (:x t) (:env t) (:kont t)))))

        :if
        (recur :eval (:condition x) env
               (conj kont [:if (:then x) (:else x) env (:span x)]))

        :assert
        (recur :eval (:condition x) env
               (conj kont [:assert (:body x) env (:span x)]))

        :with
        (recur :eval (:env-expr x) env
               (conj kont [:with (:body x) env]))

        :select
        (recur :eval (:target x) env
               (conj kont [:select-attr (:attr x) (:default x) env (:span x)]))

        :has-attr
        (recur :eval (:target x) env
               (conj kont [:has-attr (:attr x) env]))

        :string-template
        ;; Consume leading :text parts eagerly; stop at the first :expr part
        ;; (its evaluation gets a :template frame) or join if none remain.
        (let [[remaining chunks] (consume-text-parts (:parts x) [])]
          (if-let [part (first remaining)]
            (recur :eval (:expr part) env
                   (conj kont [:template (rest remaining) chunks env]))
            (recur :value (join-template-chunks chunks) env kont)))

        :not
        (recur :eval (:expr x) env (conj kont [:not (:span x)]))

        :neg
        (recur :eval (:expr x) env (conj kont [:neg]))

        :binary
        (recur :eval (:left x) env
               (conj kont [:bin-1 (:operator x) (:right x) env (:span x)]))

        :lambda
        (recur :value
               {:kind :closure
                :param (:param x)
                :param-pattern (:param-pattern x)
                :body (:body x)
                :env env}
               env kont)

        :call
        (recur :eval (:fn x) env
               (conj kont [:call-arg (:arg x) env (:span x)]))

        (recur :unwind
               (err/failed :eval :machine-unsupported-op {:op (:op x)})
               env kont)))

      ;; ── force a thunk (memoizing store protocol) ─────────────────────
      :force
      (let [state (:state x)]
        (case (:phase @state)
          :done
          (let [r (:result @state)]
            (if (= :ok (:status r))
              (recur :value (:value r) env kont)
              (recur :unwind r env kont)))

          :forcing
          (recur :unwind
                 (err/failed :eval :infinite-recursion {:label (:label x)})
                 env kont)

          :pending
          (if (::mast x)
            (do
              (reset! state {:phase :forcing})
              (recur :eval (::mast x) (menv x) (conj kont [:update state])))
            ;; Foreign thunk (not machine-made): force through its own
            ;; compute, memoizing via the shared protocol.
            (do
              (reset! state {:phase :forcing})
              (let [r ((:compute x))]
                (reset! state {:phase :done :result r})
                (if (= :ok (:status r))
                  (recur :value (:value r) env kont)
                  (recur :unwind r env kont)))))))

      ;; ── return a WHNF value to the top frame ─────────────────────────
      :value
      (if (empty? kont)
        {:status :ok :value x}
        (let [f (peek kont)
              kont (pop kont)]
          (case (nth f 0)
            :update
            (do
              (reset! (nth f 1) {:phase :done :result {:status :ok :value x}})
              (recur :value x env kont))

            :if
            (let [[_ then else fenv span] f]
              (if-let [violation (ev/if-condition-violation x span)]
                (recur :unwind violation env kont)
                (recur :eval (if x then else) fenv kont)))

            :assert
            (let [[_ body fenv span] f]
              (if-let [violation (ev/assert-condition-violation x span)]
                (recur :unwind violation env kont)
                (if x
                  (recur :eval body fenv kont)
                  (recur :unwind
                         (err/failed :eval :assertion-failed {:span span})
                         env kont))))

            :with
            ;; Oracle: non-attrset with is a no-op scope (body still runs).
            (let [[_ body fenv] f]
              (if (ev/attrset-value? x)
                (recur :eval body
                       (assoc fenv scopes-key
                              (cons x (get fenv scopes-key)))
                       kont)
                (recur :eval body fenv kont)))

            :select-attr
            ;; eval-select parity, including the map?/contains? shape (not
            ;; attrset-value?) and the miss-vs-default arbitration. The forced
            ;; slot value flows through a [:select-finish] frame so a selected
            ;; NULLARY builtin finishes exactly like eval-select's. A dynamic
            ;; key evaluates under a [:select-key] frame (M7e) — its D20
            ;; string-check held is NOT caught by an `or` default.
            (let [[_ attr default fenv span] f]
              (if (dynamic-key? attr)
                (recur :eval (:expr attr) fenv
                       (conj kont [:select-key x default fenv span]))
                (let [t (select-transition x attr default fenv env kont)]
                  (recur (:mode t) (:x t) (:env t) (:kont t)))))

            :select-key
            ;; x is the evaluated dynamic key — D20 string check, then the
            ;; same select decision as the static arm (shared transition).
            (let [[_ targetv default fenv _span] f
                  kr (ev/attr-key-value-result x)]
              (if (not= :ok (:status kr))
                (recur :unwind kr env kont)
                (let [t (select-transition targetv (:value kr) default fenv
                                           env kont)]
                  (recur (:mode t) (:x t) (:env t) (:kont t)))))

            :attrs-path
            ;; x is one evaluated dynamic SEGMENT of a dotted-path binding —
            ;; D20 string check, then keep resolving the remaining segments
            ;; (attrs-path-transition merges when they run out).
            (let [[_ b rest-bindings done segs values env' rec? env-ref
                   enclosing] f
                  kr (ev/attr-key-value-result x)]
              (if (not= :ok (:status kr))
                (recur :unwind kr env kont)
                (let [t (attrs-path-transition b rest-bindings
                                               (conj done (:value kr)) segs
                                               values env' rec? env-ref
                                               enclosing env kont)]
                  (recur (:mode t) (:x t) (:env t) (:kont t)))))

            :attrs-key
            ;; x is the evaluated dynamic attr key of an attrset binding —
            ;; D20: string check + duplicate check, then continue the
            ;; frame-driven construction.
            (let [[_ b rest-bindings values env' rec? env-ref enclosing] f
                  kr (ev/attr-key-value-result x)]
              (if (not= :ok (:status kr))
                (recur :unwind kr env kont)
                (let [k (:value kr)]
                  (if (contains? values k)
                    (recur :unwind
                           (err/failed :eval :duplicate-attr {:attr k})
                           env kont)
                    (let [slot (mthunk [:attr k] (:value b)
                                       (if (:from-enclosing b) enclosing env'))
                          env'' (if rec? (assoc env' k slot) env')]
                      (when rec? (reset! env-ref env''))
                      (let [t (attrs-transition rest-bindings
                                                (assoc values k slot)
                                                env'' rec? env-ref enclosing
                                                env kont)]
                        (recur (:mode t) (:x t) (:env t) (:kont t))))))))

            :select-finish
            (let [r (ev/nullary-builtin-result x)]
              (if (= :ok (:status r))
                (recur :value (:value r) env kont)
                (recur :unwind r env kont)))

            :has-attr
            ;; Nix `?` on a non-attrset is FALSE, not an error (D6). A
            ;; dynamic key evaluates under a [:hasattr-key] frame (M7e);
            ;; its D20 string-check held propagates like eval-has-attr's.
            (let [[_ attr fenv] f]
              (if (dynamic-key? attr)
                (recur :eval (:expr attr) fenv
                       (conj kont [:hasattr-key x]))
                (recur :value
                       (if (ev/attrset-value? x)
                         (contains? x attr)
                         false)
                       env kont)))

            :hasattr-key
            (let [[_ targetv] f
                  kr (ev/attr-key-value-result x)]
              (if (not= :ok (:status kr))
                (recur :unwind kr env kont)
                (recur :value
                       (if (ev/attrset-value? targetv)
                         (contains? targetv (:value kr))
                         false)
                       env kont)))

            :template
            ;; x is the evaluated :expr part — coerce via the shared
            ;; interpolation algebra, then consume following :text parts up
            ;; to the next :expr (or join).
            (let [[_ parts chunks fenv] f
                  piece (ev/interpolation-value-result x)]
              (if (not= :ok (:status piece))
                (recur :unwind piece env kont)
                (let [[remaining chunks] (consume-text-parts
                                          parts (conj chunks (:value piece)))]
                  (if-let [part (first remaining)]
                    (recur :eval (:expr part) fenv
                           (conj kont [:template (rest remaining) chunks fenv]))
                    (recur :value (join-template-chunks chunks) env kont)))))

            :not
            (let [r (ev/not-value-result x (nth f 1))]
              (if (= :ok (:status r))
                (recur :value (:value r) env kont)
                (recur :unwind r env kont)))

            :neg
            (let [r (ev/neg-value-result x)]
              (if (= :ok (:status r))
                (recur :value (:value r) env kont)
                (recur :unwind r env kont)))

            :bin-1
            (let [[_ op right fenv span] f]
              (case op
                ("&&" "||" "->")
                (let [checked (ev/logical-operand-result
                               op :left {:status :ok :value x} span)]
                  (if (not= :ok (:status checked))
                    (recur :unwind checked env kont)
                    (cond
                      (and (= op "&&") (false? x))
                      (recur :value false env kont)

                      (and (= op "||") (true? x))
                      (recur :value true env kont)

                      (and (= op "->") (false? x))
                      (recur :value true env kont)

                      :else
                      (recur :eval right fenv
                             (conj kont [:bin-logical op span])))))

                (recur :eval right fenv (conj kont [:bin-2 op x]))))

            :bin-logical
            (let [[_ op span] f
                  checked (ev/logical-operand-result
                           op :right {:status :ok :value x} span)]
              (if (= :ok (:status checked))
                (recur :value x env kont)
                (recur :unwind checked env kont)))

            :bin-2
            (let [[_ op leftv] f
                  r (ev/binary-value-result op leftv x)]
              (if (= :ok (:status r))
                (recur :value (:value r) env kont)
                (recur :unwind r env kont)))

            :call-arg
            (let [[_ arg fenv _span] f]
              (cond
                ;; eval-call's tryEval special case: evaluate the argument
                ;; under a [:try-eval] frame — the :unwind mode catches
                ;; exactly the D3 catchable reasons (throw/assert).
                (and (map? x)
                     (= :builtin (:kind x))
                     (= :tryEval (:name x)))
                (recur :eval arg fenv (conj kont [:try-eval]))

                ;; A simple closure applies NATIVELY — the body stays in this
                ;; loop (the constant-stack claim). Machine and evaluator
                ;; closures are the same shape, so both take this path. The
                ;; env-ref deref mirrors apply-callable's base-env exactly
                ;; (rec-attrset closures built by the evaluator carry one).
                (and (map? x)
                     (= :closure (:kind x))
                     (string? (:param x))
                     (nil? (:param-pattern x)))
                (recur :eval (:body x)
                       (assoc (or (some-> x :env-ref deref) (:env x))
                              (:param x)
                              (mthunk [:arg (:param x)] arg fenv))
                       kont)

                ;; A PATTERN closure applies natively too (M7d, D19
                ;; semantics): evaluate the argument to WHNF under a
                ;; [:pattern-bind] frame — the binding itself is value-level
                ;; (guards + knot), and both the defaults and the body stay
                ;; machine control.
                (and (map? x)
                     (= :closure (:kind x))
                     (:param-pattern x))
                (recur :eval arg fenv (conj kont [:pattern-bind x]))

                ;; Everything else — builtins (partial application, D2 lazy
                ;; positions, finish), pattern closures, host fns, and the
                ;; not-callable error — delegates WHOLE to the evaluator's
                ;; public apply-callable: one definition, zero drift. The
                ;; builtin's internals run on the evaluator's own recursion —
                ;; a hand-placed value-algebra boundary (weval discipline),
                ;; not machine control.
                :else
                (let [r (ev/apply-callable x (mthunk [:arg] arg fenv))]
                  (if (= :ok (:status r))
                    (if (thunk? (:value r))
                      (recur :force (:value r) env kont)
                      (recur :value (:value r) env kont))
                    (recur :unwind r env kont)))))

            :try-eval
            ;; tryEval succeeded: wrap like eval-call does (the value is NOT
            ;; deep-forced — WHNF, exactly the evaluator's shape).
            (recur :value {"success" true "value" x} env kont)

            :pattern-bind
            ;; x is the WHNF argument of a pattern closure — the D19
            ;; application semantics, value-level (apply-callable's pattern
            ;; branch verbatim): attrset guard, REQUIRED formals in pattern
            ;; order, extra keys unless `...`, then a KNOT-TIED env where a
            ;; missing formal binds a lazy machine-thunk default (forced
            ;; in-loop like every machine thunk — a cycle is the blackhole)
            ;; and @as binds the actual argument only.
            (let [closure (nth f 1)
                  pattern (:param-pattern closure)
                  params (:params pattern)
                  as-name (:as pattern)
                  base-env (or (some-> closure :env-ref deref)
                               (:env closure))]
              (cond
                (not (ev/attrset-value? x))
                (recur :unwind
                       (err/failed :eval
                                 :lambda-pattern-arg-not-attrset
                                 {:value-type (ev/strict-type x)})
                       env kont)

                :else
                (if-let [missing (some (fn [{:keys [name default]}]
                                         (when (and (not default)
                                                    (not (contains? x name)))
                                           name))
                                       params)]
                  (recur :unwind
                         (err/failed :eval
                                   :missing-lambda-pattern-arg
                                   {:param missing})
                         env kont)
                  (let [formal-names (into #{} (map :name) params)
                        extra (when-not (:ellipsis? pattern)
                                (first (sort (remove formal-names (keys x)))))]
                    (if extra
                      (recur :unwind
                             (err/failed :eval
                                       :unexpected-lambda-pattern-arg
                                       {:arg extra})
                             env kont)
                      (let [env-ref (atom nil)
                            final-env
                            (reduce (fn [acc {:keys [name default]}]
                                      (assoc acc name
                                             (if (contains? x name)
                                               (get x name) ; raw slot, lazy
                                               (mthunk name default env-ref))))
                                    (cond-> base-env
                                      as-name (assoc as-name x))
                                    params)]
                        (reset! env-ref final-env)
                        (recur :eval (:body closure) final-env kont))))))))))

      ;; ── propagate a held outward ─────────────────────────────────────
      ;; Pending :update frames memoize the held (exactly force-value's
      ;; memoize-the-held behavior). select-attr does NOT catch held targets:
      ;; `or` applies only after a WHNF target (select-transition), matching
      ;; eval-select / nix-instantiate (nested missing intermediate propagates).
      :unwind
      (if (empty? kont)
        x
        (let [f (peek kont)
              kont (pop kont)]
          (case (nth f 0)
            :update
            (do
              (reset! (nth f 1) {:phase :done :result x})
              (recur :unwind x env kont))

            :try-eval
            ;; Nix tryEval catches only throw and assert (D3 taxonomy);
            ;; every other held keeps unwinding, uncatchable.
            (if (contains? #{:throw-builtin-called :assertion-failed}
                           (:reason x))
              (recur :value {"success" false "value" false} env kont)
              (recur :unwind x env kont))

            (recur :unwind x env kont)))))))

;; ── iterative deep realize (the boundary) ─────────────────────────────────

(deftype RealizeExit [n])
(deftype RealizeMapExit [ks])

(defn- force-mthunk
  "Force one machine thunk with a FRESH machine run (itself constant-stack),
  respecting the shared state protocol. Flat: called from the realize loop,
  never nested."
  [t]
  (let [state (:state t)]
    (case (:phase @state)
      :done (:result @state)
      :forcing (err/failed :eval :infinite-recursion {:label (:label t)})
      :pending
      (do
        (reset! state {:phase :forcing})
        (let [r (if (::mast t)
                  (run-whnf (::mast t) (menv t))
                  ((:compute t)))]
          (reset! state {:phase :done :result r})
          r)))))

(defn realize-deep
  "Deep-realize a machine value into plain Clojure data, iteratively (an
  explicit work stack + an output stack) — the evaluator's recursive
  force-deep is itself a D1c recursion site, so the machine lane realizes
  without JVM recursion too. Attrsets realize in sorted-key order like
  force-deep. Propagates the first held."
  [v]
  (loop [stack [v]
         out []]
    (if (empty? stack)
      {:status :ok :value (peek out)}
      (let [item (peek stack)
            stack (pop stack)]
        (cond
          (instance? RealizeExit item)
          (let [n (.n ^RealizeExit item)
                cnt (count out)
                built (vec (subvec out (- cnt n) cnt))]
            (recur stack (conj (vec (subvec out 0 (- cnt n))) built)))

          (instance? RealizeMapExit item)
          (let [ks (.ks ^RealizeMapExit item)
                n (count ks)
                cnt (count out)
                built (zipmap ks (subvec out (- cnt n) cnt))]
            (recur stack (conj (vec (subvec out 0 (- cnt n))) built)))

          (thunk? item)
          (let [r (force-mthunk item)]
            (if (= :ok (:status r))
              (recur (conj stack (:value r)) out)
              r))

          (vector? item)
          (recur (into (conj stack (RealizeExit. (count item))) (rseq item))
                 out)

          (ev/attrset-value? item)
          (let [ks (vec (sort (keys item)))]
            (recur (into (conj stack (RealizeMapExit. ks))
                         (map #(get item %))
                         (reverse ks))
                   out))

          :else
          (recur stack (conj out item)))))))

;; ── public API ────────────────────────────────────────────────────────────

(def ^:private legacy-reason->error-class
  {:unbound-var :unknown-variable
   :missing-attr :attribute-missing
   :call-target-not-callable :not-callable
   :infinite-recursion :cycle-detected
   :machine-unsupported-op :unsupported-expression})

(defn- public-machine-result
  [result]
  (if (= :held (:status result))
    (let [error (or (:error result) {})
          candidate (or (:class error) (:reason error) (:reason result))
          error-class (if (keyword? candidate)
                        (get legacy-reason->error-class candidate candidate)
                        :machine-evaluation-failed)]
      (-> result
          (assoc :status :failed)
          (assoc :error (assoc error
                               :phase (or (:phase error) :eval)
                               :class error-class))))
    result))

(defn run-ast
  "Static fragment check → machine run (under the evaluator's default env —
  builtins included, M7c) → iterative realize. The machine's own control path
  is constant JVM stack (safe on a tiny thread); parsing and delegated
  builtin internals are the depth-bearing boundaries."
  [ast]
  (public-machine-result
   (if-let [violation (unsupported-node ast)]
     (err/failed :eval :machine-unsupported-op violation)
     (let [r (run-whnf ast ev/default-env)]
       (if (= :ok (:status r))
         (realize-deep (:value r))
         r)))))

(defn eval-source
  "Parse (recursive descent — runs on a dedicated big stack so deep probe
  sources parse, exactly like the core lane) then run the machine on the
  calling thread (which needs no depth)."
  [source]
  (let [parsed (promise)
        t (Thread. nil
                   (fn [] (deliver parsed (try {:ok (parser/parse-source source)}
                                               (catch Throwable t {:thrown t}))))
                   "pnix-machine-parse"
                   (* 2048 1024 1024))]
    (.start t)
    (.join t)
    (let [{:keys [ok thrown]} @parsed]
      (cond
        thrown {:status :failed
                :reason :machine-parse-failed
                :error {:phase :parse :class :syntax-error}}
        (not= :ok (:status ok)) ok
        :else (run-ast (:ast ok))))))

;; ── report (M7g — the :machine report-artifact capability) ───────────────

(def differential-corpus
  "The machine⇄evaluator differential rows (one source per row; every fragment
  feature incl. the D18/D19/D20 oracle matrices). Shared by the report below
  and the bootstrap gate pin, so the regression surface is one list."
  ["1 + 2 * 3" "(1 + 2) * 3" "10 / 4" "10.0 / 4" "1 / 0" "-5 + 3"
   "true && false || true" "!(1 < 2)" "1 < 2 && 2 <= 2" "1 == 1.0"
   "[1 2] == [1 2]" "\"a\" + \"b\"" "\"a\" < \"b\"" "[1 2] ++ [3]"
   "[] ++ null" "[1] ++ 2" "null ++ []"
   "null // { a = 1; }" "{ a = 1; } // null" "null // null"
   "[] // { a = 1; }" "{ a = 1; } // { b = 2; }"
   "builtins.attrValues null" "builtins.attrNames null"
   "builtins.elem 1 null" "builtins.genList (x: x) (-1)"
   "builtins.genList (x: x) 0" "builtins.genList (x: x) 3"
   "builtins.fromJSON 1" "builtins.compareVersions 1 2"
   "builtins.dirOf 1" "builtins.baseNameOf 1"
   "builtins.toJSON (x: x)" "with null; 1"
   "builtins.catAttrs \"a\" null" "builtins.listToAttrs [ 1 ]"
   "({ a = 1; }.b).c or 9" "{ a = {}; }.a.b or 7" "{ a = 1; }.b or 9"
   "builtins.hasAttr \"a\" null" "builtins.intersectAttrs null { a = 1; }"
   "builtins.mapAttrs (n: v: v) null" "builtins.groupBy (x: x) null"
   "null ? a"
   "builtins.zipAttrsWith (n: vs: vs) null" "builtins.genericClosure 1"
   "builtins.elemAt [1 2] 1.0"
   "builtins.replaceStrings [\"a\"] [\"b\" \"c\"] \"a\""
   "builtins.catAttrs null [ ]" "builtins.getAttr \"a\" null"
   "builtins.baseNameOf \"/\"" "builtins.baseNameOf \"a/b/c\""
   "builtins.dirOf \"/\""
   "builtins.split \"\" \"ab\"" "builtins.match \"\" \"a\""
   "builtins.split \".\" \"a.b.c\""
   "builtins.elemAt [1 2] 2" "builtins.elemAt [1 2] (-1)"
   "builtins.elemAt [10 20 30] 1"
   "[1 [2 [3]]]"
   "let a = 1; b = a + 1; in a + b" "let a = b + 1; b = 1; in a"
   ;; D22 dotted let (parser path->nested + machine follows evaluator)
   "let a.b = 1; in a.b" "let a.b = 1; a.c = 2; in a.b + a.c"
   "let a.b.c = 7; in a.b.c" "let a = { b = 1; }; a.c = 2; in a.b + a.c"
   "let a = 1 / 0; in 1" "let f = x: x + 1; in f 41"
   "let f = x: y: x + y; in f 1 2" "(x: x x) (x: 1)" "(x: 1) (1 / 0)"
   "let a = a; in a" "let x = 1; in let x = 2; in x"
   "if 1 < 2 then \"t\" else \"e\"" "if 1 then 1 else 2"
   "1 && true" "true && 1" "false || 2" "1 -> true"
   "false && (1 / 0 == 0)" "true || (1 / 0 == 0)" "-true" "!5" "\"a\" + 1"
   "some_unbound_name"
   "{ a = 1; b = 2; }" "{ a = 1; }.a" "rec { a = 1; b = a + 1; }.b"
   "rec { a = b; b = 2; }.a" "let x = 5; in { inherit x; }.x"
   "let a = 1; in rec { inherit a; b = a + 1; }.b" "{ a = 1 / 0; b = 2; }.b"
   "{ a = { b = 3; }; }.a.b" "{ a.b = 1; }.a.b"
   "rec { a = rec { b = c; c = 9; }; }.a.b"
   "{ a = 1; }.b or 5" "{ }.a or \"d\"" "{ a = { }; }.a.b or \"z\""
   "{ a = 1 / 0; }.a or 5" "{ }.a or unknown_free"
   "{ a = 1; } ? a" "{ } ? a" "1 ? a" "{ a.b = 1; } ? a.b" "{ a = 1; } ? a.b"
   "assert true; 42" "assert false; 42" "assert 5; 1" "assert 1 < 2; \"ok\""
   "with { a = 1; }; a" "let a = 2; in with { a = 1; }; a"
   "with { a = 1; }; with { a = 2; }; a" "with 5; 1" "with { }; zzz"
   "(with { a = 7; }; (x: a + x)) 1" "with { a = 1; b = 2; }; a + b"
   "\"x${\"y\"}z\"" "\"a${\"b${\"c\"}d\"}e\"" "\"n${1}\""
   "\"${{ __toString = self: \"S\"; }}\"" "\"pre${\"\"}post\""
   "let v = \"V\"; in \"[${v}]\""
   "let s = rec { n = 3; m = n * n; }; in if s ? m then s.m else 0"
   "with rec { a = 2; b = a + 3; }; [ a b ]" "{ f = x: x + 1; }.f 41"
   "assert { a = true; }.a; \"passed\""
   "{ a = 1; b = [ 2 3 ]; } == { a = 1; b = [ 2 3 ]; }"
   "rec { a = 1; } == { a = 1; }"
   "builtins.length [ 1 2 3 ]" "builtins.add 1 2" "(builtins.add 1) 2"
   "builtins.map (x: x * 2) [ 1 2 3 ]" "map (x: x + 1) [ 1 2 ]"
   "toString 1.5" "builtins.attrNames { b = 1; a = 2; }"
   "builtins.foldl' (a: b: a + b) 0 [ 1 2 3 ]"
   "builtins.concatStringsSep \",\" [ \"a\" \"b\" ]"
   "builtins.length (builtins.map (throw \"BOOM\") [ ])" "builtins.map 1 [ ]"
   "builtins.foldl' (a: b: b) (throw \"BOOM\") [ 1 ]"
   "builtins.foldl' (a: b: a) 0 [ (throw \"BOOM\") ]"
   "builtins.any (x: x) [ true (throw \"BOOM\") ]"
   "builtins.length [ (throw \"BOOM\") 2 ]" "builtins.any (throw \"BOOM\") [ ]"
   "builtins.foldl' (throw \"BOOM\") 0 [ ]"
   "throw \"X\"" "abort \"Y\"" "1 + (throw \"Z\")"
   "(builtins.tryEval (throw \"t\")).success" "(builtins.tryEval 42).value"
   "(builtins.tryEval (assert false; 1)).success"
   "(builtins.tryEval (1 && true)).success"
   "(builtins.tryEval (builtins.head [ ])).success"
   "builtins.tryEval (throw \"t\")"
   "with { map = 1; }; map (x: x) [ 2 ]" "let map = x: \"shadowed\"; in map 1"
   "builtins.hasAttr \"a\" { a = 1; }" "builtins.hasAttr \"a\" 1"
   "let __curPos = 1; in __curPos" "\"n=${toString (1 + 2)}\""
   "builtins.readFile \"/x\"" "zzz_unbound" "builtins.nixVersion" "1 2"
   "({ a ? throw \"x\" }: 1) { }" "({ a ? b, b ? 2 }: a) { }"
   "({ a ? b, b ? a }: a) { }" "({ a }: a) { a = 1; b = 2; }"
   "({ a, ... }: a) { a = 1; b = 2; }" "({ a }: a) 1" "({ a }: 1) { }"
   "({ a ? 5 }@args: args.a or \"absent\") { }"
   "let f = { x, y ? x * 2 }: x + y; in f { x = 3; }"
   "builtins.map ({ v }: v + 1) [ { v = 1; } { v = 2; } ]"
   "builtins.functionArgs ({ a, b ? 1, ... }: a)"
   "(builtins.tryEval (({ a }: a) { a = 1; b = 2; })).success"
   "rec { g = { n ? 5 }: n; v = g { }; }.v"
   "let k = \"x\"; in { \"${k}\" = 5; }.x" "{ ${\"x\"} = 1; ${\"y\"} = 2; }.y"
   "{ a = 1; \"${\"a\"}\" = 2; }.a" "{ ${\"a\"} = 1; a = 2; }.a"
   "{ }.${1} or \"d\"" "let s = { a = 1; ${\"a\"} = 2; }; in 1"
   "rec { a = 2; \"${\"b\"}\" = a; }.b" "rec { \"${\"b\"}\" = a; a = 2; }.b"
   "{ a = 1; }.${\"a\"}" "{ }.${\"q\"} or \"dflt\""
   "{ a = 1; } ? ${\"a\"}" "1 ? ${\"a\"}" "{ } ? ${1}"
   "{ ${\"a\"} = 1 / 0; b = 2; }.b"
   "let z = 3; in { inherit z; ${\"w\"} = z + 1; }.w"
   "rec { n = 5; ${\"m\"} = n * 2; }.m"
   "./a" "./a == ./a" "./a == ./b" "{ p = ./x; }.p" "toString ./x"
   "./a + \"/b\"" "./a + ./b" "./a + \"b\"" "\"x\" + ./a"
   "./a < ./b" "\"${./x}\"" "[ ./a ./b ]"
   "builtins.typeOf ./a" "builtins.typeOf (./a + \"/b\")" "import ./m"])

(defn- comparable-result
  [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))

(defn- run-on-stack
  [kb f]
  (let [p (promise)
        t (Thread. nil
                   (fn [] (deliver p (try {:r (f)}
                                          (catch Throwable t
                                            {:threw (str (class t))}))))
                   "machine-report-stack"
                   (long (* kb 1024)))]
    (.start t)
    (.join t)
    @p))

(defn report
  "The :machine report artifact: (1) the full differential corpus, machine ==
  evaluator EXACTLY (ok and held alike); (2) the constant-stack witness — the
  same plus-chain AST finishes on a 256KB thread under the machine while the
  recursive tree-walk overflows. :ok only when every row agrees and the
  witness holds; :failed pins the first divergence. Labels stay honest: the
  differential is generative evidence, not a proof; builtin internals and
  import resolution run on the evaluator (hand-placed boundaries)."
  []
  (let [core-eval (requiring-resolve 'pnix-clj.core/eval-source)
        rows (mapv (fn [source]
                     (let [m (comparable-result (eval-source source))
                           e (comparable-result (core-eval source))]
                       {:source source
                        :machine m
                        :evaluator e
                        :agree? (= m e)}))
                   differential-corpus)
        divergent (filterv (complement :agree?) rows)
        depth-src (str "1" (apply str (repeat 30000 " + 1")))
        parsed (:r (run-on-stack (* 2 1024 1024)
                                 #(parser/parse-source depth-src)))
        machine-small (run-on-stack 256 #(run-ast (:ast parsed)))
        treewalk-small (run-on-stack 256 #(ev/eval-ast (:ast parsed)))
        witness {:depth 30000
                 :machine-256kb (comparable-result
                                 (or (:r machine-small)
                                     {:status :failed
                                      :reason :machine-stack-run-threw}))
                 :treewalk-256kb (or (some-> (:r treewalk-small)
                                             comparable-result)
                                     [:threw (:threw treewalk-small)])
                 :ok? (boolean
                       (and (= [:ok 30001]
                               (comparable-result (or (:r machine-small) {})))
                            (or (:threw treewalk-small)
                                (not= :ok (:status (:r treewalk-small))))))}]
    {:kind :machine-report
     :schema :pnix-clj.machine-report.v0
     :status (if (and (empty? divergent) (:ok? witness)) :ok :failed)
     :row-count (count rows)
     :divergent divergent
     :constant-stack-witness witness
     :derivation {:technique :functional-correspondence
                  :laziness :krivine-memoizing-store
                  :value-algebra :shared-evaluator-seams
                  :honest-labels [:differential-not-proof
                                  :builtin-internals-on-evaluator-recursion
                                  :fuel-ticks-approximate]}}))

(defn -main
  [& _args]
  (let [{:keys [status row-count divergent constant-stack-witness]} (report)]
    (println (format "pnix-clj machine: status=%s rows=%d divergent=%d witness=%s"
                     (name status) row-count (count divergent)
                     (if (:ok? constant-stack-witness) "ok" "FAILED")))
    (when (seq divergent)
      (doseq [d (take 5 divergent)]
        (println "  DIVERGE" (pr-str d))))
    (System/exit (if (= :ok status) 0 1))))
