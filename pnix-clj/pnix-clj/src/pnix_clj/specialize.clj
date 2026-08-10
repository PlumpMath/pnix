(ns pnix-clj.specialize
  "pnix partial evaluator — Futamura stage 1 (roadmap M1, first slice).

  (specialize source static-env) folds everything that depends only on the
  static environment and emits a pnix RESIDUAL for the rest. Soundness rules
  (role-translated from pnix-hy's audit lessons, adapted to a same-language
  residual):
  - let is a RECURSIVE scope: a binding name is never resolved against the
    static env inside its own let (sibling set wins), and folding happens at
    the whole-let node via the real evaluator — no sequential approximation.
    Unlike pnix-hy (whose residual target Hy has sequential let), our residual
    IS pnix, so keeping a partially-dynamic let structurally intact is sound.
  - if prunes only on a REAL bool static condition; a non-bool static
    condition records a gap and keeps the full residual.
  - Folding is delegated to the production evaluator on closed subtrees, so a
    fold can never disagree with evaluation. A held fold keeps the residual
    (same held at runtime) and records a gap.
  - Heavy subtrees (lambdas/calls) fold under a FUEL budget on the fixed-stack
    fuel evaluator, so a divergent fold burns out into a gap instead of
    hanging; imports never fold. Lambda-parameter references are closed at
    their lambda (:lams, like let siblings at :sibs), so fully-applied closed
    calls fold — capture-free, since static substitution never crosses a
    shadowing parameter.
  Verification: eval(residual, dynamics) == eval(source, statics+dynamics),
  exercised per case by specialize-report."
  (:require [clojure.set :as set]
            [clojure.string :as str]
            [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.error :as err]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.parser :as parser]
            [pnix-clj.unparse :as unparse]))

(def lane-classification
  {:lane :proof-only
   :scope :partial-evaluation-equivalence-proof
   :product-runtime :forbidden
   :product-codegen :forbidden
   :optimizer-authority :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :residual-use :proof-witness-only
   :default-on-gap :held
   :allowed-output :specialization-equivalence-report})

;; --- static values <-> literal AST -------------------------------------------

(defn- data-value?
  "A value we can re-embed as a literal (plain data only — no closures,
  builtins, thunks, paths, or contextful strings in this slice)."
  [v]
  (cond
    (or (integer? v) (float? v) (boolean? v) (nil? v) (string? v)) true
    (vector? v) (every? data-value? v)
    (map? v) (and (not (contains? v :kind))
                  (not (contains? v "__pnix_value_kind"))
                  (every? (fn [[k x]] (and (string? k) (data-value? x))) v))
    :else false))

(defn- value->ast
  [v]
  (cond
    (integer? v) {:op :int :value v}
    (float? v) {:op :float :value v}
    (boolean? v) {:op :bool :value v}
    (nil? v) {:op :null :value nil}
    (string? v) {:op :string :value v}
    (vector? v) {:op :list :items (mapv value->ast v)}
    (map? v) {:op :attrset
              :recursive false
              :attrs (mapv (fn [[k x]] {:key k :value (value->ast x)})
                           (sort-by key v))}))

(def ^:private literal-ops
  #{:int :float :bool :null :string :path :var :lambda})

;; --- the walk -----------------------------------------------------------------
;;
;; Each node returns {:node residual :dyn? bool :heavy? bool :sibs #{names}}:
;;   dyn?  — depends on a dynamic variable or an import
;;   heavy? — contains a lambda/call/import (never folded in this slice)
;;   sibs  — free references to enclosing let-sibling names, resolved (removed)
;;           at the let that binds them so the WHOLE let can fold as one node.

(declare walk)

(defn- combine
  [results]
  {:dyn? (boolean (some :dyn? results))
   :heavy? (boolean (some :heavy? results))
   :sibs (reduce set/union #{} (map :sibs results))
   :lams (reduce set/union #{} (map :lams results))})

(defn- foldable?
  [{:keys [node dyn? heavy? sibs lams]}]
  (and (not dyn?)
       (not heavy?)
       (empty? sibs)
       (empty? lams)
       (not (contains? literal-ops (:op node)))))

(def ^:private fold-fuel
  "Step budget for folding :heavy (call/lambda-bearing) closed subtrees. Small
  enough that a divergent fold burns out quickly, large enough for ordinary
  builtin applications."
  4096)

(defn- try-fold
  "Fold a closed subtree by running the real evaluator on it. Non-heavy
  subtrees (no calls/lambdas — cannot diverge) evaluate unbounded; heavy ones
  evaluate under a fuel budget so a divergent fold becomes a gap instead of a
  hang. On :ok with a data value, replace with a literal; on :ok with a
  non-data value (closure, path, ...) keep the residual (still sound); on held
  keep the residual and record a gap (runtime holds the same way)."
  [{:keys [node dyn? heavy? sibs lams] :as result} gaps]
  (if (or dyn? (seq sibs) (seq lams)
          (contains? literal-ops (:op node)))
    result
    (let [r (if heavy?
              (evaluator/eval-ast-with-fuel node fold-fuel)
              (try
                (evaluator/eval-ast node)
                (catch Throwable t
                  (err/failed-throwable :specialize :fold-eval-threw t))))]
      (cond
        (and (= :ok (:status r)) (data-value? (:value r)))
        (assoc result :node (value->ast (:value r)))

        (= :ok (:status r))
        result

        (= :suspended (:status r))
        (do (vswap! gaps conj {:reason :fold-fuel-exhausted
                               :op (:op node)
                               :fuel fold-fuel})
            result)

        :else
        (do (vswap! gaps conj {:reason :static-eval-failed
                               :op (:op node)
                               :failure-reason (:reason r)})
            result)))))

(defn- walk-attr-key
  "Attrset/select keys: plain strings pass through; dynamic keys specialize
  their inner expression."
  [k env lambda-bound sib-names gaps]
  (if (and (map? k) (= :dynamic-attr-key (:kind k)))
    (let [r (walk (:expr k) env lambda-bound sib-names gaps)]
      [(assoc k :expr (:node r)) r])
    [k {:dyn? false :heavy? false :sibs #{} :lams #{}}]))

(defn- walk
  [{:keys [op] :as ast} env lambda-bound sib-names gaps]
  (case op
    (:int :float :bool :null :string :path)
    {:node ast :dyn? false :heavy? false :sibs #{} :lams #{}}

    :var
    (let [n (:name ast)]
      (cond
        ;; sibling first (pnix-hy A4): a let-bound name must NEVER be resolved
        ;; against the static env inside its own let.
        (contains? sib-names n)
        {:node ast :dyn? false :heavy? false :sibs #{n} :lams #{}}

        ;; a lambda-bound reference is CLOSED at its lambda (tracked in :lams
        ;; and removed there), so a fully-applied call can still fuel-fold
        (contains? lambda-bound n)
        {:node ast :dyn? false :heavy? false :sibs #{} :lams #{n}}

        (contains? env n)
        {:node (value->ast (get env n)) :dyn? false :heavy? false :sibs #{}
         :lams #{}}

        ;; default-scope globals (builtins, map, toString, ...) are fixed:
        ;; the folding evaluator resolves them itself, and lexical scope beats
        ;; with-scope for these names, so subtrees using them stay foldable.
        (contains? evaluator/default-env n)
        {:node ast :dyn? false :heavy? false :sibs #{} :lams #{}}

        :else
        ;; free: with-scope or truly dynamic
        {:node ast :dyn? true :heavy? false :sibs #{} :lams #{}}))

    :string-template
    (let [parts (mapv (fn [{:keys [kind] :as part}]
                        (if (= :expr kind)
                          (let [r (walk (:expr part) env lambda-bound sib-names gaps)]
                            [(assoc part :expr (:node r)) r])
                          [part {:dyn? false :heavy? false :sibs #{} :lams #{}}]))
                      (:parts ast))]
      (try-fold (merge {:node (assoc ast :parts (mapv first parts))}
                       (combine (map second parts)))
                gaps))

    :list
    (let [items (mapv #(walk % env lambda-bound sib-names gaps) (:items ast))]
      (try-fold (merge {:node (assoc ast :items (mapv :node items))}
                       (combine items))
                gaps))

    :attrset
    (let [entries (mapv (fn [{:keys [key path value] :as entry}]
                          (let [[k kr] (if path
                                         [nil {:dyn? false :heavy? false
                                               :sibs #{} :lams #{}}]
                                         (walk-attr-key key env lambda-bound
                                                        sib-names gaps))
                                pks (when path
                                      (mapv #(walk-attr-key % env lambda-bound
                                                            sib-names gaps)
                                            path))
                                vr (walk value env lambda-bound sib-names gaps)]
                            {:entry (cond-> (assoc entry :value (:node vr))
                                      (and (not path) (some? k)) (assoc :key k)
                                      path (assoc :path (mapv first pks)))
                             :results (concat [vr]
                                              (when-not path [kr])
                                              (map second (or pks [])))}))
                        (:attrs ast))]
      (try-fold (merge {:node (assoc ast :attrs (mapv :entry entries))}
                       (combine (mapcat :results entries)))
                gaps))

    :let
    ;; Recursive scope: binding names join sib-names for both binding values
    ;; and the body; the whole let folds as one node when nothing dynamic or
    ;; sibling-external remains.
    (let [names (set (map :name (:bindings ast)))
          sib' (set/union sib-names names)
          bindings (mapv (fn [{:keys [value] :as b}]
                           (let [r (walk value env lambda-bound sib' gaps)]
                             {:binding (assoc b :value (:node r)) :result r}))
                         (:bindings ast))
          body (walk (:body ast) env lambda-bound sib' gaps)
          inner (combine (conj (mapv :result bindings) body))]
      (try-fold {:node (assoc ast
                              :bindings (mapv :binding bindings)
                              :body (:node body))
                 :dyn? (:dyn? inner)
                 :heavy? (:heavy? inner)
                 ;; this let resolves its own names
                 :sibs (set/difference (:sibs inner) names)
                 :lams (:lams inner)}
                gaps))

    :if
    ;; Prune only on a REAL bool static condition (pnix-hy A15); a non-bool
    ;; static condition is a gap + full residual.
    (let [cond-r (walk (:condition ast) env lambda-bound sib-names gaps)
          then-r (walk (:then ast) env lambda-bound sib-names gaps)
          else-r (walk (:else ast) env lambda-bound sib-names gaps)
          residual (merge {:node (assoc ast
                                        :condition (:node cond-r)
                                        :then (:node then-r)
                                        :else (:node else-r))}
                          (combine [cond-r then-r else-r]))]
      (if (and (not (:dyn? cond-r)) (not (:heavy? cond-r))
               (empty? (:sibs cond-r)))
        ;; static condition: evaluate it (a bare literal evaluates to itself);
        ;; prune ONLY on a real bool. A non-bool static condition is a gap and
        ;; the FULL residual is returned WITHOUT try-fold — otherwise the
        ;; whole-node fold would run the lenient evaluator and erase the gap.
        (let [cv (evaluator/eval-ast (:node cond-r))]
          (cond
            (and (= :ok (:status cv)) (true? (:value cv))) (try-fold then-r gaps)
            (and (= :ok (:status cv)) (false? (:value cv))) (try-fold else-r gaps)
            (= :ok (:status cv))
            (do (vswap! gaps conj {:reason :if-non-bool-condition
                                   :value-type (str (type (:value cv)))})
                residual)
            :else
            (do (vswap! gaps conj {:reason :static-eval-held
                                   :op :if-condition
                                   :held-reason (:reason cv)})
                residual)))
        (try-fold residual gaps)))

    :select
    (let [t (walk (:target ast) env lambda-bound sib-names gaps)
          [k kr] (walk-attr-key (:attr ast) env lambda-bound sib-names gaps)
          d (some-> (:default ast) (walk env lambda-bound sib-names gaps))
          tn (:node t)
          ;; Partial select: when the target is a plain, non-recursive attrset
          ;; literal whose keys are all static strings, selecting one entry is
          ;; sound even if OTHER entries stay dynamic — attrset values are
          ;; lazy, so the discarded entries were never going to be forced.
          plain-entries (when (and (= :attrset (:op tn))
                                   (not (:recursive tn))
                                   (every? #(and (string? (:key %))
                                                 (not (:path %)))
                                           (:attrs tn)))
                          (:attrs tn))
          picked (when (and plain-entries (string? k))
                   (some #(when (= k (:key %)) %) plain-entries))]
      (cond
        picked
        (try-fold (merge {:node (:value picked)}
                         (select-keys t [:dyn? :heavy? :sibs]))
                  gaps)

        ;; key set fully known and missing: the default (if any) applies
        (and plain-entries (string? k) d)
        (try-fold (merge {:node (:node d)}
                         (select-keys d [:dyn? :heavy? :sibs]))
                  gaps)

        :else
        (try-fold (merge {:node (cond-> (assoc ast :target tn :attr k)
                                  d (assoc :default (:node d)))}
                         (combine (remove nil? [t kr d])))
                  gaps)))

    :has-attr
    (let [t (walk (:target ast) env lambda-bound sib-names gaps)
          [k kr] (walk-attr-key (:attr ast) env lambda-bound sib-names gaps)]
      (try-fold (merge {:node (assoc ast :target (:node t) :attr k)}
                       (combine [t kr]))
                gaps))

    (:not :neg)
    (let [r (walk (:expr ast) env lambda-bound sib-names gaps)]
      (try-fold (merge {:node (assoc ast :expr (:node r))}
                       (combine [r]))
                gaps))

    :binary
    (let [l (walk (:left ast) env lambda-bound sib-names gaps)
          r (walk (:right ast) env lambda-bound sib-names gaps)]
      (try-fold (merge {:node (assoc ast :left (:node l) :right (:node r))}
                       (combine [l r]))
                gaps))

    :assert
    (let [c (walk (:condition ast) env lambda-bound sib-names gaps)
          b (walk (:body ast) env lambda-bound sib-names gaps)]
      (try-fold (merge {:node (assoc ast :condition (:node c) :body (:node b))}
                       (combine [c b]))
                gaps))

    :with
    ;; Lexical (static) substitution inside a with-body is sound: lexical
    ;; bindings beat the with-scope in pnix. Free names under with stay
    ;; dynamic via the :var default branch.
    (let [e (walk (:env-expr ast) env lambda-bound sib-names gaps)
          b (walk (:body ast) env lambda-bound sib-names gaps)]
      (try-fold (merge {:node (assoc ast :env-expr (:node e) :body (:node b))}
                       (combine [e b]))
                gaps))

    :lambda
    ;; Never folded in this slice; parameters shadow static names.
    (let [params (if-let [p (:param-pattern ast)]
                   (cond-> (set (map :name (:params p)))
                     (:as p) (conj (:as p)))
                   #{(:param ast)})
          bound' (set/union lambda-bound params)
          ;; defaults evaluate in the pattern scope
          pattern' (when-let [p (:param-pattern ast)]
                     (update p :params
                             (fn [ps]
                               (mapv (fn [{:keys [default] :as prm}]
                                       (if default
                                         (assoc prm :default
                                                (:node (walk default env bound'
                                                             sib-names gaps)))
                                         prm))
                                     ps))))
          b (walk (:body ast) env bound' sib-names gaps)]
      {:node (cond-> (assoc ast :body (:node b))
               pattern' (assoc :param-pattern pattern'))
       :dyn? (:dyn? b)
       :heavy? true
       :sibs (:sibs b)
       ;; this lambda closes over its own parameters
       :lams (set/difference (or (:lams b) #{}) params)})

    :call
    (let [f (walk (:fn ast) env lambda-bound sib-names gaps)
          a (walk (:arg ast) env lambda-bound sib-names gaps)
          c (combine [f a])]
      (try-fold {:node (assoc ast :fn (:node f) :arg (:node a))
                 :dyn? (:dyn? c)
                 :heavy? true
                 :sibs (:sibs c)
                 :lams (:lams c)}
                gaps))

    :import
    {:node ast :dyn? true :heavy? true :sibs #{} :lams #{}}

    ;; Unknown op: keep residual, mark dynamic+heavy (never folded), gap it.
    (do (vswap! gaps conj {:reason :unsupported-op :op op})
        {:node ast :dyn? true :heavy? true :sibs #{} :lams #{}})))

;; --- public API ---------------------------------------------------------------

(defn- observes-positions?
  "True when the AST references a source-position-observing builtin
  (unsafeGetAttrPos). Positions are metadata of the ORIGINAL source text;
  a residual is different text, so no residual can preserve them — folding
  literalizes attrsets and erases position metadata, and re-parsing the
  residual assigns new spans. Such sources are honestly refused instead of
  silently changing an observable answer (caught live by the tower's
  specialize-residual layer)."
  [node]
  (cond
    (map? node) (or (and (= :var (:op node))
                         (= "unsafeGetAttrPos" (:name node)))
                    (and (map? (:attr node))
                         (observes-positions? (:attr node)))
                    (= "unsafeGetAttrPos" (:attr node))
                    (some observes-positions? (vals (dissoc node :attr))))
    (sequential? node) (some observes-positions? node)
    :else false))

(defn specialize
  "Partially evaluate pnix `source` under `static-env` (map of name -> data
  value). Returns {:status :ok :fully-static? .. :value .. :residual-ast ..
  :residual-source .. :gaps [..]} or a failed map."
  [source static-env]
  (cond
    (not (every? (fn [[k v]] (and (string? k) (data-value? v))) static-env))
    (err/failed :specialize
                :static-env-not-data
                {:static-env-keys (vec (keys static-env))})

    :else
    (let [{:keys [status ast] :as parsed} (parser/parse-source (str source))]
      (cond
        (not= :ok status)
        parsed

        (observes-positions? ast)
        (err/failed :specialize
                    :position-observing-source-not-specializable
                    {:builtin "unsafeGetAttrPos"})

        :else
        (let [gaps (volatile! [])
              result (walk ast static-env #{} #{} gaps)
              residual (:node result)
              fully-static? (and (empty? @gaps)
                                 (contains? #{:int :float :bool :null :string}
                                            (:op residual)))
              value (when fully-static? (:value residual))
              residual-source (unparse/unparse residual)]
          {:status :ok
           :fully-static? (boolean fully-static?)
           :value value
           :residual-ast residual
           :residual-source residual-source
           :residual-hash (hash/data-hash (unparse/strip-positions residual))
           :gaps @gaps})))))

;; --- differential verification report -----------------------------------------

(defn- env-wrap
  "Wrap a source with literal let-bindings for the given env."
  [env source]
  (if (empty? env)
    (str "(" source ")")
    (str "let "
         (apply str (map (fn [[k v]]
                           (str k " = " (unparse/unparse (value->ast v)) "; "))
                         env))
         "in (" source ")")))

(defn- eval-source-with-fuel
  [source fuel]
  (let [parsed (parser/parse-source source)]
    (if (= :ok (:status parsed))
      (evaluator/eval-ast-with-fuel (:ast parsed) fuel)
      parsed)))

(def cases
  [{:id :a4-inner-shadow-recursive-let
    :source "let x = 5; in let y = x + 1; x = 10; in y"
    :statics {} :dynamics {}
    :expect {:fully-static? true :value 11}}
   {:id :a4-forward-reference
    :source "let b = a + 1; a = 2; in b"
    :statics {} :dynamics {}
    :expect {:fully-static? true :value 3}}
   {:id :a4-dynamic-sibling-stays-sound
    :source "let b = a + d; a = 2; in b"
    :statics {} :dynamics {"d" 40}
    :expect {:fully-static? false}}
   {:id :a5-multipath-attrset
    :source "{ a.b = 1; c = 2; }.a.b"
    :statics {} :dynamics {}
    :expect {:fully-static? true :value 1}}
   {:id :a15-if-non-bool-condition
    :source "if 1 then 2 else 3"
    :statics {} :dynamics {}
    :expect {:fully-static? false :gap-reason :if-non-bool-condition}}
   {:id :static-substitution
    :source "x + y"
    :statics {"x" 40} :dynamics {"y" 2}
    :expect {:fully-static? false}}
   {:id :if-static-bool-prunes
    :source "if flag then a + 1 else a - 1"
    :statics {"flag" true} :dynamics {"a" 10}
    :expect {:fully-static? false :residual-excludes "else"}}
   {:id :lambda-param-shadows-static
    ;; folds to 6 (capture-free: a captured static x=100 would give 101)
    :source "(x: x + 1) 5"
    :statics {"x" 100} :dynamics {}
    :expect {:fully-static? true :value 6}}
   {:id :attrset-partial-fold
    :source "{ s = a + 1; d = y; }.s"
    :statics {"a" 1} :dynamics {"y" 9}
    :expect {:fully-static? true :value 2}}
   {:id :string-template-folds
    :source "\"v=${builtins.toString (1 + 1)}\""
    :statics {} :dynamics {}
    :expect {:fully-static? true :value "v=2"}}
   {:id :builtin-call-fold
    :source "builtins.length [ 1 2 3 ]"
    :statics {} :dynamics {}
    :expect {:fully-static? true :value 3}}
   {:id :higher-order-fold
    :source "builtins.foldl' (a: b: a + b) 0 [ 1 2 3 4 ]"
    :statics {} :dynamics {}
    :expect {:fully-static? true :value 10}}
   {:id :divergent-fold-burns-fuel
    :source "let f = x: f x; in f 1"
    :statics {} :dynamics {}
    ;; Never feed a known divergence back into the unbounded differential
    ;; evaluator. Compare the original and residual under the same finite
    ;; observation budget; fuel exhaustion remains an observation, not a
    ;; nontermination proof.
    :differential-fuel 64
    :expect {:fully-static? false :gap-reason :fold-fuel-exhausted}}
   {:id :dynamic-arg-blocks-call-fold
    :source "(x: x + 1) d"
    :statics {} :dynamics {"d" 41}
    :expect {:fully-static? false}}])

(defn- run-case
  [{:keys [id source statics dynamics expect differential-fuel]}]
  (let [sp (specialize source statics)
        require-clj (requiring-resolve 'pnix-clj.core/eval-source)
        eval-source (if differential-fuel
                      #(eval-source-with-fuel % differential-fuel)
                      require-clj)
        original (eval-source (env-wrap (merge statics dynamics) source))
        residual-run (when (= :ok (:status sp))
                       (eval-source (env-wrap dynamics (:residual-source sp))))
        meaning-preserved?
        (and (= :ok (:status sp))
             (= (select-keys original [:status :value :reason])
                (select-keys residual-run [:status :value :reason])))
        expect-ok?
        (and (= :ok (:status sp))
             (or (not (contains? expect :fully-static?))
                 (= (:fully-static? expect) (:fully-static? sp)))
             (or (not (contains? expect :value))
                 (= (:value expect) (:value sp)))
             (or (not (contains? expect :gap-reason))
                 (some #(= (:gap-reason expect) (:reason %)) (:gaps sp)))
             (or (not (contains? expect :residual-excludes))
                 (not (.contains ^String (:residual-source sp)
                                 ^String (:residual-excludes expect)))))]
    {:id id
     :source source
     :status (if (and meaning-preserved? expect-ok?) :accepted :rejected)
     :meaning-preserved? meaning-preserved?
     :differential-mode (if differential-fuel :bounded-fuel :finite)
     :differential-fuel differential-fuel
     :expect-ok? expect-ok?
     :fully-static? (:fully-static? sp)
     :value (:value sp)
     :residual-source (:residual-source sp)
     :gaps (:gaps sp)
     :original (select-keys original [:status :value :reason])
     :residual-run (select-keys residual-run [:status :value :reason])}))

;; --- content-addressed specialization cache (M1×M6 composition) ---------------
;;
;; specialize is pure and deterministic (folding delegates to the pure
;; evaluator), so memoizing by canonical content is sound. Same idiom as
;; pnix-clj.cached-eval: schemad key, clear!, stats; bypasses are evaluated
;; fresh so the cache can never change an answer. The tower's
;; specialize-residual layer runs through this cache, so repeated climbs of
;; the same corpus pay the fold cost once per epoch.

(def specialize-cache-epoch 1)

(def ^:private specialize-cache (atom {}))
(def ^:private cache-stats* (atom {:hits 0 :misses 0 :bypasses 0}))

(defn clear-specialize-cache!
  []
  (reset! specialize-cache {})
  (reset! cache-stats* {:hits 0 :misses 0 :bypasses 0}))

(defn specialize-cache-stats
  []
  (assoc @cache-stats* :entries (count @specialize-cache)))

(defn specialize-cached
  "specialize with content-addressed memoization. Key = position-stripped AST
  hash + statics hash + epoch. Parse failures, non-data statics, and held
  results bypass (always computed fresh). The result carries
  :cache {:status :hit|:miss|:bypass ...}."
  [source static-env]
  (let [bypass (fn [r reason]
                 (swap! cache-stats* update :bypasses inc)
                 (assoc r :cache {:status :bypass :reason reason}))
        {:keys [status ast] :as parsed} (parser/parse-source (str source))]
    (cond
      (not= :ok status)
      (bypass parsed :parse-failed)

      (not (every? (fn [[k v]] (and (string? k) (data-value? v))) static-env))
      (bypass (specialize source static-env) :static-env-not-data)

      :else
      (let [k {:schema :pnix-clj.specialize-cache-key.v0
               :content-hash (hash/data-hash (unparse/strip-positions ast))
               :statics-hash (hash/data-hash (into (sorted-map) static-env))
               :epoch specialize-cache-epoch}]
        (if-let [hit (get @specialize-cache k)]
          (do (swap! cache-stats* update :hits inc)
              (assoc hit :cache {:status :hit :key k}))
          (let [r (specialize source static-env)]
            (if (= :ok (:status r))
              (do (swap! specialize-cache assoc k r)
                  (swap! cache-stats* update :misses inc)
                  (assoc r :cache {:status :miss :key k}))
              (bypass r :specialize-held))))))))

;; --- Futamura projection: residual -> lowering -> clj-meta host artifact ------

(defn specialize-to-host
  "The actual Futamura projection: specialize `source` under `statics`, close
  the residual over its dynamic parameter names (sorted), lower the resulting
  pnix lambda through the lowering lane, and compile/evaluate it via clj-meta
  (which carries a bytecode compile receipt with a determinism check).
  Invoking the compiled artifact with the dynamic values must equal evaluating
  the ORIGINAL source under the full env."
  [source statics dynamics]
  (let [sp (specialize source statics)]
    (if (not= :ok (:status sp))
      sp
      (let [names (vec (sort (keys dynamics)))
            lambda-source (if (empty? names)
                            (str "(" (:residual-source sp) ")")
                            (str "(" (str/join ": " names)
                                 ": (" (:residual-source sp) "))"))
            ;; clj-meta verifies eval-form == compiled value, and two function
            ;; INSTANCES never compare equal — so the form handed to the host
            ;; lane is the residual lambda APPLIED to the dynamic arguments (a
            ;; closed call whose value is data). Futamura contract intact:
            ;; the compiled artifact applied to the inputs must equal the
            ;; original program on the full env.
            wrapper-source (reduce (fn [acc n]
                                     (str "(" acc ") "
                                          (unparse/unparse
                                           (value->ast (get dynamics n)))))
                                   lambda-source
                                   names)
            parsed (parser/parse-source wrapper-source)]
        (if (not= :ok (:status parsed))
          (assoc parsed :phase :wrapper-parse :wrapper-source wrapper-source)
          (let [lowered (lowering/lower-ast (:ast parsed))]
            (if (not= :ok (:status lowered))
              (assoc lowered :phase :lowering :wrapper-source wrapper-source)
              ;; Each proof row compiles/evaluates three equivalent units.
              ;; Retaining the default 128 units across the finite report made
              ;; the proof JVM exceed hosted-runner memory even though no
              ;; compiled function escapes this call.
              (let [cm (clj-meta/eval-lowered-determinism-bounded
                        (:form lowered) 1)]
                (if (not= :ok (:status cm))
                  (assoc cm :phase :clj-meta :wrapper-source wrapper-source)
                  {:status :ok
                   :specialize sp
                   :wrapper-source wrapper-source
                   :lowered-form (:form lowered)
                   :clj-meta-mode (:mode cm)
                   :bytecode-determinism
                   (get-in cm [:compile-receipt :determinism :status])
                   :invoked {:status :ok :value (:value cm)}})))))))))

(def futamura-cases
  [{:id :arith-partial
    :source "x + y" :statics {"x" 40} :dynamics {"y" 2}}
   {:id :if-pruned-residual-fn
    :source "if flag then a + 1 else a - 1"
    :statics {"flag" true} :dynamics {"a" 10}}
   {:id :let-mixed
    :source "let s = a * 2; in s + d" :statics {"a" 21} :dynamics {"d" 0}}
   {:id :two-dynamics
    :source "let base = p * 10; in base + x * y"
    :statics {"p" 4} :dynamics {"x" 1 "y" 2}}
   {:id :fully-static-artifact
    :source "builtins.length [ 1 2 3 ]" :statics {} :dynamics {}}
   {:id :attrset-pick-residual
    :source "{ k = a + d; }.k" :statics {"a" 1} :dynamics {"d" 9}}])

(defn- run-futamura-case
  [{:keys [id source statics dynamics]}]
  (let [eval-source (requiring-resolve 'pnix-clj.core/eval-source)
        expected (eval-source (env-wrap (merge statics dynamics) source))
        fut (specialize-to-host source statics dynamics)
        invoked (:invoked fut)
        match? (and (= :ok (:status fut))
                    (= :ok (:status invoked))
                    (= :ok (:status expected))
                    (= (:value expected) (:value invoked)))]
    {:id id
     :source source
     :status (if match? :accepted :rejected)
     :phase (:phase fut)
     :wrapper-source (:wrapper-source fut)
     :residual-source (get-in fut [:specialize :residual-source])
     :bytecode-determinism (:bytecode-determinism fut)
     :invoked (select-keys invoked [:status :value :reason])
     :expected (select-keys expected [:status :value :reason])}))

(defn- run-futamura-case-released
  [case]
  (let [row (run-futamura-case case)]
    ;; run-futamura-case has returned a data-only row, so its generated
    ;; functions and DynamicClassLoaders are no longer live. Request class
    ;; unloading here, after the compile frame has disappeared.
    (System/gc)
    row))

(defn report
  []
  (let [rows (mapv run-case cases)
        frows (mapv run-futamura-case-released futamura-cases)
        rejected (+ (count (filter #(= :rejected (:status %)) rows))
                    (count (filter #(= :rejected (:status %)) frows)))
        body {:kind :pnix-specialize-report
              :schema :pnix-clj.specialize-report.v1
              :policy :futamura-stage1-differential-and-host-projection
              :total (+ (count rows) (count frows))
              :accepted (- (+ (count rows) (count frows)) rejected)
              :rejected rejected
              :differential-total (count rows)
              :futamura-total (count frows)
              :rows rows
              :futamura-rows frows}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash
                         [(mapv #(dissoc % :residual-source) rows)
                          (mapv #(dissoc % :residual-source :wrapper-source)
                                frows)]))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected rows futamura-rows]} (report)]
    (println (format "pnix-clj specialize: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (doseq [{:keys [id status fully-static? value gaps]} rows]
      (println (format "  [%s] %s fully-static=%s value=%s gaps=%d"
                       (if (= :accepted status) "OK" "REJECT")
                       (name id) fully-static? (pr-str value) (count gaps))))
    (doseq [{:keys [id status bytecode-determinism invoked expected]} futamura-rows]
      (println (format "  [%s] futamura:%s bytecode-determinism=%s invoked=%s expected=%s"
                       (if (= :accepted status) "OK" "REJECT")
                       (name id) (pr-str bytecode-determinism)
                       (pr-str (:value invoked)) (pr-str (:value expected)))))
    (shutdown-agents)))
