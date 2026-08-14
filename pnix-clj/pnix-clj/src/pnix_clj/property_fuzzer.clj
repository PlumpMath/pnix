(ns pnix-clj.property-fuzzer
  "Property-based differential fuzzer (deep-research F3 extension). Where
  grammar-fuzzer runs a FIXED generated set, this uses test.check to GENERATE
  random valid pnix expressions and assert a CROSS-LANE property over them --
  and, crucially, SHRINKS any counterexample to its minimal failing source.

  The property: for a generated pure closed pnix expression, the four
  substrates (direct evaluator, clj-meta bytecode, .px runtime, pnix mirror)
  must COLLAPSE to one value -- no lane may silently disagree. The TYPE-DIRECTED
  generator (gen-typed-expr) emits closed, TOTAL sources of int/bool type --
  arithmetic (+ - *), let, if, lists, builtins.length/foldl', attrset-select,
  comparisons, and boolean ops -- never ill-typed (`int + bool`) and never
  partial (no division, no head of a possibly-empty list), so any non-:accepted
  or lane-disagreeing result is a REAL cross-substrate bug. When one is found,
  test.check reports the SMALLEST such source (automatic minimization), which
  grammar-fuzzer's fixed corpus cannot do.

  This capability has already paid off: it found clj-meta nested-let shadowing
  (fixed) and host-parser let/if-as-operator-RHS leniency (filed). The wider the
  generator, the more it can surface -- 300 tests x 4 seeds now collapse clean.

  A genuine automated silent-wrong hunt with minimization -- Clojure-standard
  generative testing (clojure.test.check) applied to the N-version harness."
  (:require [clojure.string :as str]
            [clojure.test.check :as tc]
            [clojure.test.check.generators :as gen]
            [clojure.test.check.properties :as prop]
            [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :bounded-property-fuzz-evidence
   :product-runtime :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :counterexample-use :debug-evidence-only
   :default-on-failure :report-only
   :allowed-output :shrunk-counterexample-or-proof-report})

;; ---- generator: closed, always-evaluating pnix integer expressions ------

(def ^:private gen-int-literal
  (gen/fmap str (gen/choose 0 20)))

(defn- gen-expr
  "Recursive integer-expression generator, size-bounded via gen/recursive-gen.
  Uses only + - * (no division/mod) over literals, so every generated source is
  closed and terminates with an integer value on every lane."
  []
  (gen/recursive-gen
   (fn [inner]
     (gen/one-of
      [;; (a OP b)
       (gen/let [a inner
                 op (gen/elements ["+" "-" "*"])
                 b inner]
         (str "(" a " " op " " b ")"))
       ;; (let v = a; in (v OP b))  -- bindings; PARENTHESIZED so it is a valid
       ;; operand (Nix rejects an unparenthesized let/if as an operator RHS --
       ;; a conformance point the fuzzer surfaced, see roadmap host-parser-let-rhs).
       (gen/let [a inner
                 op (gen/elements ["+" "-" "*"])
                 b inner]
         (str "(let v = " a "; in (v " op " " b "))"))
       ;; (if (a < b) then a else b) -- branch + comparison, parenthesized
       (gen/let [a inner b inner]
         (str "(if (" a " < " b ") then " a " else " b ")"))]))
   gen-int-literal))

;; ---- typed generator: broader pnix coverage, still closed + total -------
;;
;; The wider the generator, the more cross-lane bugs it can surface. This one
;; is TYPE-DIRECTED (int / bool / list-of-int) so it never emits an ill-typed
;; source (`int + bool`), and every construct is TOTAL (no division, no head of
;; a possibly-empty list) -- so any divergence is a real bug, not noise.

(declare gen-int gen-bool gen-list)

(defn- gen-int
  [depth]
  (if (<= depth 0)
    gen-int-literal
    (let [i (gen-int (dec depth))
          l (gen-list (dec depth))]
      (gen/one-of
       [gen-int-literal
        (gen/let [a i op (gen/elements ["+" "-" "*"]) b i]
          (str "(" a " " op " " b ")"))
        (gen/let [a i op (gen/elements ["+" "-" "*"]) b i]
          (str "(let v = " a "; in (v " op " " b "))"))
        (gen/let [c (gen-bool (dec depth)) a i b i]
          (str "(if " c " then " a " else " b ")"))
        ;; builtins.length is total on any list
        (gen/let [xs l] (str "(builtins.length " xs ")"))
        ;; attrset select -- always resolves the named key
        (gen/let [a i] (str "({ k = " a "; }.k)"))
        ;; builtins.foldl' with + over a list (total)
        (gen/let [init i xs l]
          (str "(builtins.foldl' (acc: e: acc + e) " init " " xs ")"))]))))

(defn- gen-bool
  [depth]
  (if (<= depth 0)
    (gen/elements ["true" "false"])
    (let [b (gen-bool (dec depth))
          i (gen-int (dec depth))]
      (gen/one-of
       [(gen/elements ["true" "false"])
        (gen/let [a i b2 i] (str "(" a " < " b2 ")"))
        (gen/let [a i b2 i] (str "(" a " == " b2 ")"))
        (gen/let [x b y b] (str "(" x " && " y ")"))
        (gen/let [x b y b] (str "(" x " || " y ")"))
        (gen/let [x b] (str "(!" x ")"))]))))

(defn- gen-list
  [depth]
  (gen/let [xs (gen/vector (gen-int (max 0 (dec depth))) 0 4)]
    (str "[ " (str/join " " xs) " ]")))

(defn gen-typed-expr
  "A closed, total pnix expression of int OR bool type (both collapse to a
  scalar the four lanes must agree on), covering arithmetic, let, if, lists,
  builtins.length/foldl', attrset-select, comparisons, and boolean ops."
  []
  (gen/sized (fn [size]
               (let [d (min 4 (quot size 2))]
                 (gen/one-of [(gen-int d) (gen-bool d)])))))

;; ---- open expressions over free params, for the specializer property ----

(defn- gen-open-int
  "Integer expression over the FREE parameters x and y (plus literals), total
  and closed once x,y are supplied. Used to fuzz the partial evaluator: any
  static/dynamic split must preserve meaning."
  [depth]
  (if (<= depth 0)
    (gen/one-of [gen-int-literal (gen/elements ["x" "y"])])
    (let [i (gen-open-int (dec depth))]
      (gen/one-of
       [gen-int-literal
        (gen/elements ["x" "y"])
        (gen/let [a i op (gen/elements ["+" "-" "*"]) b i]
          (str "(" a " " op " " b ")"))
        (gen/let [a i b i c i]
          (str "(if (" a " < " b ") then " c " else " a ")"))]))))

(def ^:private full-env {"x" 3 "y" 5})

(defn specialize-preserves-meaning?
  "The partial-evaluator soundness property (F1): for `source` over {x,y} and a
  chosen STATIC subset, the residual re-evaluated under the remaining dynamics
  must equal the direct evaluation under the full env. A violation is a real
  specializer meaning-change bug."
  [source static-keys]
  (let [statics (select-keys full-env static-keys)
        dynamics (apply dissoc full-env static-keys)
        specialize-to-host (requiring-resolve 'pnix-clj.specialize/specialize-to-host)
        eval-source (requiring-resolve 'pnix-clj.core/eval-source)
        direct (eval-source (str "let x = " (full-env "x") "; y = " (full-env "y")
                                 "; in (" source ")"))
        fut (specialize-to-host source statics dynamics)]
    (and (= :ok (:status direct))
         (= :ok (:status fut))
         (= (:value direct) (get-in fut [:invoked :value])))))

(defn specializer-property
  []
  (prop/for-all [source (gen/sized (fn [n] (gen-open-int (min 4 (quot n 2)))))
                 static-keys (gen/elements [[] ["x"] ["y"] ["x" "y"]])]
    (specialize-preserves-meaning? source static-keys)))

(defn- gen-pure-arith
  "Pure-arithmetic open expression over {x,y} -- no if -- so the arithmetic
  proof (canonical polynomial) applies and specializer soundness can be PROVEN
  for all values, not merely tested at one point."
  [depth]
  (if (<= depth 0)
    (gen/one-of [gen-int-literal (gen/elements ["x" "y"])])
    (let [i (gen-pure-arith (dec depth))]
      (gen/one-of
       [gen-int-literal
        (gen/elements ["x" "y"])
        (gen/let [a i op (gen/elements ["+" "-" "*"]) b i]
          (str "(" a " " op " " b ")"))]))))

(defn specializer-proven-property
  "STRONGER than specializer-property: for a generated pure-arithmetic source
  and a static split, PROVE (canonical polynomial equality, forall values) that
  specialize preserved meaning -- a generative PROOF, not a point test."
  []
  (let [prove (requiring-resolve 'pnix-clj.arith-proof/prove-specialize-meaning)]
    (prop/for-all [source (gen/sized (fn [n] (gen-pure-arith (min 4 (quot n 2)))))
                   static-keys (gen/elements [[] ["x"] ["y"] ["x" "y"]])]
      (= :proven (:status (prove source (select-keys full-env static-keys)))))))

;; ---- cached-eval soundness property (M6) --------------------------------

(defn cache-preserves-meaning?
  "The content-addressed cache (M6) must NEVER change an answer: a first
  (miss) and a second (hit) cached-eval of `source` equal each other AND equal
  a bypassing fresh evaluation. Generative check over random sources."
  [source]
  (let [cached-eval (requiring-resolve 'pnix-clj.cached-eval/cached-eval)
        clear! (requiring-resolve 'pnix-clj.cached-eval/clear-eval-cache!)
        eval-source (requiring-resolve 'pnix-clj.core/eval-source)
        _ (clear!)                       ; each check is independent (miss then hit)
        fresh (eval-source source)
        c1 (cached-eval source)
        c2 (cached-eval source)]
    (and (= :ok (:status fresh))
         (= :ok (:status c1))
         (= :ok (:status c2))
         (= (:value fresh) (:value c1) (:value c2))
         (= :miss (get-in c1 [:cache :status]))
         (= :hit (get-in c2 [:cache :status])))))

(defn cache-property
  []
  (prop/for-all [source (gen-typed-expr)]
    (cache-preserves-meaning? source)))

;; ---- the cross-lane property --------------------------------------------

(defn lanes-collapse?
  "True iff `source` is accepted and every succeeding lane agrees on the value
  (no silent cross-substrate divergence)."
  [source]
  (let [row (pnix/verify-source source)
        vs (keep #(when (= :ok (:status (get row %)))
                    (:value (get row %)))
                 [:eval-result :clj-meta-result :px-runtime :pnix-mirror])]
    (and (= :accepted (:status row))
         (seq vs)
         (apply = vs))))

(defn cross-lane-property
  []
  (prop/for-all [source (gen-typed-expr)]
    (lanes-collapse? source)))

;; ---- the machine property (M7h) ------------------------------------------

(defn machine-agrees?
  "True iff the derived abstract machine (pnix-clj.machine) and the
  definitional evaluator agree EXACTLY on source — ok and held alike, a
  STRONGER check than lanes-collapse? (which only compares accepted values).
  The machine's shared differential corpus lives in machine/differential-corpus;
  this property sweeps the rest of the expression space randomly, so a frame
  bug the corpus misses shrinks to a minimal source here."
  [source]
  (let [machine-eval (requiring-resolve 'pnix-clj.machine/eval-source)
        eval-source (requiring-resolve 'pnix-clj.core/eval-source)
        comparable (fn [r] (if (= :ok (:status r))
                             [:ok (:value r)]
                             [(:status r) (:reason r)]))]
    (= (comparable (machine-eval source))
       (comparable (eval-source source)))))

(defn machine-property
  []
  (prop/for-all [source (gen-typed-expr)]
    (machine-agrees? source)))

(defn report
  "Run FIVE generative properties under a fixed `:seed` (deterministic), each
  with SHRINKING: (1) cross-lane collapse -- generated exprs agree on all four
  substrates; (2) specializer soundness -- partial evaluation preserves meaning
  under any static/dynamic split; (3) cache meaning preservation; (4) the
  proven-arith specializer; (5) machine⇄evaluator EXACT agreement (M7h -- the
  derived abstract machine as a generative differential lane). :ok iff all
  hold; :held with the shrunk minimal counterexample of whichever failed."
  ([] (report {}))
  ([{:keys [num-tests seed] :or {num-tests 200 seed 42}}]
   (let [cross (tc/quick-check num-tests (cross-lane-property) :seed seed)
         spec  (tc/quick-check num-tests (specializer-property) :seed seed)
         cache (tc/quick-check num-tests (cache-property) :seed seed)
         proof (tc/quick-check num-tests (specializer-proven-property) :seed seed)
         machine (tc/quick-check num-tests (machine-property) :seed seed)
         both? (and (:pass? cross) (:pass? spec) (:pass? cache) (:pass? proof)
                    (:pass? machine))
         body {:kind :pnix-property-fuzzer-report
               :schema :pnix-clj.property-fuzzer-report.v4
               :policy :generative-collapse-specializer-cache-proven-arith-AND-machine
               :num-tests num-tests
               :seed seed
               :cross-lane-pass? (:pass? cross)
               :specializer-pass? (:pass? spec)
               :cache-pass? (:pass? cache)
               :specializer-proven-arith-pass? (:pass? proof)
               :machine-pass? (:pass? machine)
               :pass? both?
               :num-tests-run (+ (:num-tests cross) (:num-tests spec)
                                 (:num-tests cache) (:num-tests proof)
                                 (:num-tests machine))}]
     (if both?
       (assoc body
              :status :ok
              :report-hash (hash/data-hash [num-tests seed both?]))
       (assoc body
              :status :failed
              :reason (cond (not (:pass? cross)) :cross-lane-divergence-found
                            (not (:pass? spec)) :specializer-meaning-change-found
                            (not (:pass? cache)) :cache-meaning-change-found
                            (not (:pass? proof)) :specializer-proof-refuted
                            :else :machine-divergence-found)
              :smallest-failing-source
              (cond (not (:pass? cross)) (get-in cross [:shrunk :smallest 0])
                    (not (:pass? spec)) (get-in spec [:shrunk :smallest])
                    (not (:pass? cache)) (get-in cache [:shrunk :smallest 0])
                    (not (:pass? proof)) (get-in proof [:shrunk :smallest])
                    :else (get-in machine [:shrunk :smallest 0])))))))

(defn -main
  [& _]
  (let [{:keys [status num-tests seed cross-lane-pass? specializer-pass?
                cache-pass? machine-pass? smallest-failing-source]} (report)]
    (println (format "pnix-clj property-fuzzer: status=%s num-tests=%d seed=%d cross-lane=%s specializer=%s cache=%s machine=%s"
                     (name status) num-tests seed cross-lane-pass? specializer-pass? cache-pass? machine-pass?))
    (when smallest-failing-source
      (println "  SMALLEST failing input:" (pr-str smallest-failing-source)))
    (shutdown-agents)
    (when (not= :ok status) (System/exit 1))))
