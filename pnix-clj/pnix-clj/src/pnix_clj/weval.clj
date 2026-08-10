(ns pnix-clj.weval
  "F8: weval-shaped IR-level 1st Futamura projection on an interpreter BODY (Fallin, weval, PLDI 2025; Truffle PE, PLDI 2017) -- pc-as-context dispatch elimination, STATIC residuals only, hand-placed boundaries; an architectural proof that clj-meta hosts IR-level PE, NOT a performance program.

  What is built (bounded spike, per docs/REMAINING_DECISION.md B):
  - a REGISTER IR for the pnix arith/bool/let/if core, compiled from the real
    parser's AST (everything outside the fragment refuses honestly);
  - an IR INTERPRETER whose body has the classic shape the technique targets:
    instruction fetch + `case` dispatch at a pc loop -- the interpretive
    overhead being eliminated;
  - the SPECIALIZER: for one static program, every reachable pc becomes its
    own residual block (pc-as-specialization-context -- the known pitfall is
    constant-prop collapse at the dispatch merge, and the known fix is
    per-context splitting with MEMOIZED blocks so join points are shared, not
    exponentially unrolled). The residual is a static Clojure form: no
    instruction vector, no pc, no `case` dispatch -- only the boundary calls.
  - HAND-PLACED boundaries (weval intrinsics / @TruffleBoundary discipline --
    Truffle removed all automatic heuristics): dynamic-value semantics
    (arith, comparison, truthiness) stay behind runtime helper fns that the
    residual calls; the specializer never inlines them.

  Honest labels (deep-research 2026-07-07): correctness = construction
  argument + differential tests (residual vs IR-interpreter vs the real pnix
  evaluator, plus the clj-meta bytecode lane); performance = HEURISTIC -- the
  report's dispatch-count metric is architectural evidence (N fetch/dispatch
  steps interpreted vs 0 in the residual), not a wall-clock claim (weval's
  own ceiling was ~2.17x with a real JIT 3.86x beyond)."
  (:require [clojure.set :as set]
            [clojure.walk :as walk]
            [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.hash :as hash]
            [pnix-clj.parser :as parser]))

(def lane-classification
  {:lane :proof-only
   :scope :ir-level-partial-evaluation-spike
   :product-runtime :forbidden
   :optimizer-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :residual-policy :static-only-no-deopt
   :boundary-policy :hand-placed-runtime-helpers
   :performance-claims :heuristic-dispatch-count-only
   :allowed-output :weval-report})

;; --- runtime boundary helpers (hand-placed, shared by interpreter AND
;; residual so the two differ ONLY in dispatch) --------------------------------

(defn bin-boundary
  "Dynamic binary-op semantics behind the boundary: pnix core arithmetic and
  comparison over longs/booleans. Division by zero and non-fragment operand
  types throw a tagged error the harness reports as held -- mirroring the
  evaluator's structured holds for the same corpus."
  [operator a b]
  (case operator
    "+" (+ (long a) (long b))
    "-" (- (long a) (long b))
    "*" (* (long a) (long b))
    "/" (if (zero? (long b))
          (throw (ex-info "weval boundary: division by zero"
                          {:weval-failed :division-by-zero}))
          (quot (long a) (long b)))
    "<" (< (long a) (long b))
    ">" (> (long a) (long b))
    "<=" (<= (long a) (long b))
    ">=" (>= (long a) (long b))
    "==" (= a b)
    "!=" (not= a b)))

(defn truthy-boundary
  "Boolean-position check behind the boundary: the fragment is strictly
  typed, so a non-boolean in an if/&&/||/->/! position throws tagged."
  [v]
  (if (boolean? v)
    v
    (throw (ex-info "weval boundary: non-boolean in boolean position"
                    {:weval-failed :non-bool}))))

(defn neg-boundary
  [v]
  (- (long v)))

;; --- pnix core AST -> register IR --------------------------------------------

(defn- free-vars
  [ast]
  (cond
    (map? ast) (let [here (when (= :var (:op ast)) #{(:name ast)})]
                 (into (or here #{})
                       (mapcat free-vars (vals (dissoc ast :op :span :source-hash)))))
    (sequential? ast) (into #{} (mapcat free-vars ast))
    :else #{}))

(def ^:private fragment-binaries
  #{"+" "-" "*" "/" "<" ">" "<=" ">=" "==" "!=" "&&" "||" "->"})

(defn- compile-ast
  "Compile a fragment AST into {:instrs [...] :ret reg}. `env` maps names to
  registers; `state` is {:instrs [] :next-reg n}. Returns [state reg] or
  throws {:weval-unsupported ...} for anything outside the fragment."
  [state env ast]
  (letfn [(fresh [st] [(update st :next-reg inc) (:next-reg st)])
          (emit [st instr] (update st :instrs conj instr))
          (unsupported [what]
            (throw (ex-info "outside the weval fragment"
                            {:weval-unsupported what})))]
    (case (:op ast)
      (:int :bool)
      (let [[st r] (fresh state)]
        [(emit st {:op :const :dst r :value (:value ast)}) r])

      :var
      (if-let [r (get env (:name ast))]
        [state r]
        (let [[st r] (fresh state)]
          [(emit st {:op :input :dst r :name (:name ast)}) r]))

      :not
      (let [[st r-in] (compile-ast state env (:expr ast))
            [st r] (fresh st)]
        [(emit st {:op :not :dst r :a r-in}) r])

      :neg
      (let [[st r-in] (compile-ast state env (:expr ast))
            [st r] (fresh st)]
        [(emit st {:op :neg :dst r :a r-in}) r])

      :binary
      (let [operator (:operator ast)]
        (when-not (contains? fragment-binaries operator)
          (unsupported {:operator operator}))
        (if (contains? #{"&&" "||" "->"} operator)
          ;; short-circuit forms compile to branches, creating the join-point
          ;; (merge) shape that pc-as-context must share, not duplicate.
          (let [[st r-left] (compile-ast state env (:left ast))
                [st r-out] (fresh st)
                branch-at (count (:instrs st))
                st (emit st ::branch-placeholder)
                ;; taken path: evaluate the right side
                [st r-right] (compile-ast st env (:right ast))
                st (emit st {:op :copy :dst r-out :src r-right})
                jump-at (count (:instrs st))
                st (emit st ::jump-placeholder)
                ;; short-circuit path: constant result
                const-pc (count (:instrs st))
                st (emit st {:op :const :dst r-out
                             :value (case operator "&&" false "||" true "->" true)})
                join-pc (count (:instrs st))
                take-right-pc (inc branch-at)
                st (-> st
                       (assoc-in [:instrs branch-at]
                                 (case operator
                                   ;; && : true -> right, false -> const false
                                   "&&" {:op :branch :cond r-left
                                         :then take-right-pc :else const-pc}
                                   ;; || : true -> const true, false -> right
                                   "||" {:op :branch :cond r-left
                                         :then const-pc :else take-right-pc}
                                   ;; -> : true -> right, false -> const true
                                   "->" {:op :branch :cond r-left
                                         :then take-right-pc :else const-pc}))
                       (assoc-in [:instrs jump-at] {:op :jump :to join-pc}))]
            [st r-out])
          (let [[st r-a] (compile-ast state env (:left ast))
                [st r-b] (compile-ast st env (:right ast))
                [st r] (fresh st)]
            [(emit st {:op :bin :dst r :operator operator :a r-a :b r-b}) r])))

      :if
      (let [[st r-cond] (compile-ast state env (:condition ast))
            [st r-out] (fresh st)
            branch-at (count (:instrs st))
            st (emit st ::branch-placeholder)
            then-pc (count (:instrs st))
            [st r-then] (compile-ast st env (:then ast))
            st (emit st {:op :copy :dst r-out :src r-then})
            jump-at (count (:instrs st))
            st (emit st ::jump-placeholder)
            else-pc (count (:instrs st))
            [st r-else] (compile-ast st env (:else ast))
            st (emit st {:op :copy :dst r-out :src r-else})
            join-pc (count (:instrs st))
            st (-> st
                   (assoc-in [:instrs branch-at]
                             {:op :branch :cond r-cond :then then-pc :else else-pc})
                   (assoc-in [:instrs jump-at] {:op :jump :to join-pc}))]
        [st r-out])

      :let
      ;; sequential bindings only: pnix let is recursive, so any binding that
      ;; references a name of the SAME let (forward, self, or duplicate)
      ;; refuses honestly rather than silently changing semantics.
      (let [bindings (:bindings ast)
            names (map :name bindings)]
        (when (not= (count names) (count (distinct names)))
          (unsupported {:let :duplicate-names}))
        (loop [remaining bindings
               later-names (set names)
               st state
               env' env]
          (if-let [{:keys [name value]} (first remaining)]
            (do
              (when (seq (set/intersection (free-vars value) later-names))
                (unsupported {:let :recursive-or-forward-reference}))
              (let [[st r] (compile-ast st env' value)]
                (recur (rest remaining) (disj later-names name) st (assoc env' name r))))
            (compile-ast st env' (:body ast)))))

      (unsupported {:op (:op ast)}))))

(defn compile-to-ir
  "Parse + compile a fragment source to {:instrs [...] :ret reg} with a final
  :ret instruction, or {:status :unsupported ...}."
  [source]
  (let [parsed (parser/parse-source source)]
    (if (not= :ok (:status parsed))
      {:status :unsupported :reason (:reason parsed)}
      (try
        (let [[st r] (compile-ast {:instrs [] :next-reg 0} {} (:ast parsed))
              st (update st :instrs conj {:op :ret :src r})]
          {:status :ok
           :instrs (:instrs st)
           :ret r})
        (catch clojure.lang.ExceptionInfo e
          (if-let [what (:weval-unsupported (ex-data e))]
            {:status :unsupported :reason :outside-weval-fragment :what what}
            (throw e)))))))

;; --- the interpreter body (what gets specialized away) -----------------------

(defn run-ir
  "The interpreter BODY: fetch + `case` dispatch at a pc loop. Returns
  {:status :ok :value v :dispatch-count n} -- the count is the architectural
  overhead the residual eliminates."
  [{:keys [instrs]} inputs]
  (try
    (loop [pc 0
           regs {}
           dispatches 0]
      (let [instr (nth instrs pc)]
        (case (:op instr)
          :const (recur (inc pc) (assoc regs (:dst instr) (:value instr)) (inc dispatches))
          :input (recur (inc pc) (assoc regs (:dst instr) (get inputs (:name instr))) (inc dispatches))
          :copy (recur (inc pc) (assoc regs (:dst instr) (get regs (:src instr))) (inc dispatches))
          :bin (recur (inc pc)
                      (assoc regs (:dst instr)
                             (bin-boundary (:operator instr)
                                           (get regs (:a instr))
                                           (get regs (:b instr))))
                      (inc dispatches))
          :not (recur (inc pc)
                      (assoc regs (:dst instr)
                             (not (truthy-boundary (get regs (:a instr)))))
                      (inc dispatches))
          :neg (recur (inc pc)
                      (assoc regs (:dst instr) (neg-boundary (get regs (:a instr))))
                      (inc dispatches))
          :jump (recur (long (:to instr)) regs (inc dispatches))
          :branch (recur (long (if (truthy-boundary (get regs (:cond instr)))
                                 (:then instr)
                                 (:else instr)))
                         regs
                         (inc dispatches))
          :ret {:status :ok
                :value (get regs (:src instr))
                :dispatch-count (inc dispatches)})))
    (catch clojure.lang.ExceptionInfo e
      (if-let [held (:weval-failed (ex-data e))]
        {:status :failed :reason held}
        (throw e)))))

;; --- the specializer: pc-as-context residual generation ----------------------

(defn- block-sym [pc] (symbol (str "pc" pc)))

(defn specialize-ir
  "1st Futamura at the IR level: every reachable pc of the STATIC program
  becomes one residual block; instruction fetch and `case` dispatch are folded
  at specialization time. Blocks are MEMOIZED per pc (pc-as-context with
  shared merges), so an if/&&-join is emitted once and reached from both
  arms -- the anti-exponential reconnection the research prescribes. The
  residual is a pure static form: (fn [inputs] (letfn [...] (pc0 {})))."
  [{:keys [instrs]}]
  (let [emitted (atom {})           ; pc -> residual body form (memoized)
        order (atom [])]
    (letfn [(emit! [pc]
              (when-not (contains? @emitted pc)
                (swap! emitted assoc pc ::in-progress)
                (let [instr (nth instrs pc)
                      next-call (fn [to] (list (block-sym to) 'regs))
                      body
                      (case (:op instr)
                        :const `(~(block-sym (inc pc))
                                 (assoc ~'regs ~(:dst instr) ~(:value instr)))
                        :input `(~(block-sym (inc pc))
                                 (assoc ~'regs ~(:dst instr)
                                        (get ~'inputs ~(:name instr))))
                        :copy `(~(block-sym (inc pc))
                                (assoc ~'regs ~(:dst instr)
                                       (get ~'regs ~(:src instr))))
                        :bin `(~(block-sym (inc pc))
                               (assoc ~'regs ~(:dst instr)
                                      (bin-boundary ~(:operator instr)
                                                    (get ~'regs ~(:a instr))
                                                    (get ~'regs ~(:b instr)))))
                        :not `(~(block-sym (inc pc))
                               (assoc ~'regs ~(:dst instr)
                                      (not (truthy-boundary (get ~'regs ~(:a instr))))))
                        :neg `(~(block-sym (inc pc))
                               (assoc ~'regs ~(:dst instr)
                                      (neg-boundary (get ~'regs ~(:a instr)))))
                        :jump (next-call (:to instr))
                        :branch `(if (truthy-boundary (get ~'regs ~(:cond instr)))
                                   ~(next-call (:then instr))
                                   ~(next-call (:else instr)))
                        :ret `(get ~'regs ~(:src instr)))]
                  (swap! emitted assoc pc body)
                  (swap! order conj pc)
                  ;; recurse into successors AFTER memoizing this pc
                  (case (:op instr)
                    :jump (emit! (:to instr))
                    :branch (do (emit! (:then instr)) (emit! (:else instr)))
                    :ret nil
                    (emit! (inc pc))))))]
      (emit! 0)
      (let [pcs (sort @order)]
        {:form `(fn [~'inputs]
                  (letfn [~@(for [pc pcs]
                              `(~(block-sym pc) [~'regs] ~(get @emitted pc)))]
                    (~(block-sym 0) {})))
         :block-count (count pcs)}))))

(defn- residual-dispatch-free?
  "Construction check: the residual form must contain NO interpreter
  machinery -- no `case` dispatch, no instruction fetch (`nth` on an
  instruction vector), no pc arithmetic on a program counter."
  [form]
  (let [found (atom false)]
    (walk/postwalk (fn [x]
                     (when (and (symbol? x)
                                (contains? #{"case" "nth"} (name x)))
                       (reset! found true))
                     x)
                   form)
    (not @found)))

;; --- differential harness ----------------------------------------------------

(def corpus
  "Fragment sources with explicit deterministic input assignments (free vars
  are the DYNAMIC part; the program is the STATIC part)."
  [{:source "1 + 2 * 3" :inputs [{}]}
   {:source "(1 + 2) * (3 + 4)" :inputs [{}]}
   {:source "x + y * 2" :inputs [{"x" 1 "y" 2} {"x" 7 "y" -3} {"x" 0 "y" 5}]}
   {:source "if x < y then x else y" :inputs [{"x" 1 "y" 2} {"x" 9 "y" 2}]}
   {:source "let a = x + 1; b = a * 2; in b - x"
    :inputs [{"x" 1} {"x" 10} {"x" -4}]}
   {:source "if a && b then 1 else 2"
    :inputs [{"a" true "b" true} {"a" true "b" false} {"a" false "b" true}]}
   {:source "if a || !b then 10 else 20"
    :inputs [{"a" false "b" true} {"a" false "b" false} {"a" true "b" true}]}
   {:source "if (x == 3) -> (y > 0) then x else y"
    :inputs [{"x" 3 "y" 1} {"x" 3 "y" -1} {"x" 2 "y" -5}]}
   {:source "let m = if x >= y then x else y; in m * m"
    :inputs [{"x" 3 "y" 7} {"x" 7 "y" 3}]}
   {:source "-x + 5" :inputs [{"x" 2} {"x" -9}]}
   {:source "x / y" :inputs [{"x" 7 "y" 2} {"x" 7 "y" 0}]}
   {:source "if x != 0 then 100 / x else 0" :inputs [{"x" 5} {"x" 0}]}])

(defn- pnix-literal
  [v]
  (cond
    (boolean? v) (str v)
    (and (integer? v) (neg? v)) (str "(0 - " (- v) ")")
    :else (str v)))

(defn- eval-source-under
  "Ground truth: the real pnix evaluator on the same source with inputs bound
  by a let (matching the bool-proof grounding pattern)."
  [source inputs]
  (let [eval-source (requiring-resolve 'pnix-clj.core/eval-source)
        wrapped (if (empty? inputs)
                  (str "(" source ")")
                  (str "let "
                       (apply str (for [[k v] (sort inputs)]
                                    (str k " = " (pnix-literal v) "; ")))
                       "in (" source ")"))]
    (let [r (eval-source wrapped)]
      (if (= :ok (:status r))
        {:status :ok :value (:value r)}
        {:status :failed :reason (:reason r)}))))

(defn- run-residual
  [residual-fn inputs]
  (try
    {:status :ok :value (residual-fn inputs)}
    (catch clojure.lang.ExceptionInfo e
      (if-let [held (:weval-failed (ex-data e))]
        {:status :failed :reason held}
        (throw e)))))

(defn- comparable-result
  [{:keys [status value]}]
  (if (= :ok status) [:ok value] [:failed]))

(defn- row-for
  [{:keys [source inputs]}]
  (let [ir (compile-to-ir source)]
    (if (not= :ok (:status ir))
      {:source source :status :unsupported :reason (:reason ir)}
      (let [{:keys [form block-count]} (specialize-ir ir)
            residual-fn (eval form)
            clj-meta-result (clj-meta/eval-lowered
                             (list form (into {} (first inputs))))
            checks
            (vec (for [in inputs]
                   (let [interp (run-ir ir in)
                         resid (run-residual residual-fn in)
                         truth (eval-source-under source in)
                         agree? (= (comparable-result interp)
                                   (comparable-result resid)
                                   (comparable-result truth))]
                     {:inputs in
                      :agree? agree?
                      :interp (comparable-result interp)
                      :residual (comparable-result resid)
                      :evaluator (comparable-result truth)
                      :dispatch-count (:dispatch-count interp)})))]
        {:source source
         :status (if (every? :agree? checks) :ok :failed)
         :instr-count (count (:instrs ir))
         :block-count block-count
         :dispatch-free? (residual-dispatch-free? form)
         :residual-hash (hash/sha256 (pr-str form))
         :clj-meta-status (:status clj-meta-result)
         :clj-meta-agrees? (and (= :ok (:status clj-meta-result))
                                (= [:ok (:value clj-meta-result)]
                                   (comparable-result
                                    (run-residual residual-fn (first inputs)))))
         :checks checks}))))

(defn report
  []
  (let [rows (mapv row-for corpus)
        supported (filterv #(not= :unsupported (:status %)) rows)
        failed (filterv #(= :failed (:status %)) rows)
        dispatch-total (reduce + 0 (for [row supported
                                         c (:checks row)
                                         :when (:dispatch-count c)]
                                     (:dispatch-count c)))]
    {:kind :weval-report
     :schema :pnix-clj.weval-report.v0
     :status (if (and (empty? failed)
                      (every? :dispatch-free? supported)
                      (every? #(true? (:clj-meta-agrees? %)) supported)
                      (seq supported))
               :ok
               :failed)
     :labels {:correctness :construction-argument-plus-differential-tests
              :performance :heuristic-dispatch-count-not-wall-clock}
     :source-count (count rows)
     :supported (count supported)
     :unsupported (- (count rows) (count supported))
     :failed (count failed)
     :dispatch {:interpreted-steps dispatch-total
                :residual-steps 0
                :note "the residual contains no fetch/case dispatch by construction"}
     :clj-meta-lane {:agreeing (count (filter :clj-meta-agrees? supported))
                     :of (count supported)}
     :rows rows}))

(defn -main
  [& _]
  (let [{:keys [status supported unsupported failed dispatch clj-meta-lane]}
        (report)]
    (println (format "pnix-clj weval: status=%s supported=%d unsupported=%d failed=%d interp-dispatches=%d residual-dispatches=%d clj-meta=%d/%d"
                     (name status) supported unsupported failed
                     (:interpreted-steps dispatch) (:residual-steps dispatch)
                     (:agreeing clj-meta-lane) (:of clj-meta-lane)))
    (shutdown-agents)
    (when (not= :ok status)
      (System/exit 1))))
