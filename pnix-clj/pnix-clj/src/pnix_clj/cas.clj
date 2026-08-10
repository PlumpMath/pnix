(ns pnix-clj.cas
  "§3 — canonical normalization + content-addressed TERM store (BUILD 1st of the
  evidence-store spine; see docs/SPINE_ROADMAP.md).

  Pipeline: pnix AST → canonical form → term hash → term key → put/get.

  ★§0 LOAD-BEARING PRINCIPLE (research-verified, Maziarz PLDI'21 / Nix
  reproducibility empirics): a content hash is a PROPOSE FILTER, never a proof
  of equivalence. So a hash-hit here only PROPOSES that two terms are the same;
  `put-term!`/`get-term` CONFIRM it by exact structural equality on the canonical
  form (a hash collision is caught, not trusted). Constitution's proven-vs-
  heuristic boundary, made concrete.

  Canonicalization (v2, §3b): strip source positions; sort order-independent
  binder groups (attrset attrs by key; recursive `let` bindings by name -- pnix
  `let` is recursive, so binding ORDER is semantically irrelevant); AND
  α-quotient lambda/let binders to de-Bruijn refs WITH CORRECT SHADOWING (the
  nearest binder wins), so x:x ≡ y:y and `let x=1;in x` ≡ `let y=1;in y`. Free
  variables and attrset LABELS (observable, not α-renameable) keep their names.
  This is sound for whole terms; the unsound de-Bruijn-Merkle shortcut for OPEN
  subterms is deliberately avoided (Blaauwbroek 2024)."
  (:require [pnix-clj.hash :as hash]
            [pnix-clj.unparse :as unparse]))

(def lane-classification
  {:lane :core
   :scope :content-addressed-term-identity
   :role :canonical-form-term-hash-and-structural-confirmation
   :product-runtime :allowed
   :semantic-authority :identity-filter-not-proof
   :mutation :term-store-only
   :admission :structural-confirmation-required
   :determinism :canonical-form-required
   :allowed-output :term-key-or-structural-equivalence-verdict})

;; ---- purity guard: only pure EDN data may enter the store ---------------

(defn pure-term?
  "A term is storable only if it is pure EDN-compatible data -- no fn / atom /
  delay / IDeref / host object (identity-bearing or mutable). This is the CAS
  guard the spine's §5 event log will share (hermeticity: reject contamination)."
  [x]
  (cond
    (or (nil? x) (boolean? x) (number? x) (string? x)
        (keyword? x) (symbol? x)) true
    (map? x)    (and (every? pure-term? (keys x)) (every? pure-term? (vals x)))
    (coll? x)   (every? pure-term? x)
    :else       false))

;; ---- canonical form ------------------------------------------------------

(declare canon)

(defn- bound-ref
  "Find the nearest binder GROUP on `stack` (innermost = last) that binds `nm`;
  returns a 2-level de-Bruijn ref {:group g :slot s} (g = groups outward from
  the innermost, s = position in that group), or nil if `nm` is free."
  [stack nm]
  (first (keep-indexed
          (fn [g grp]
            (when-let [s (some (fn [[i x]] (when (= x nm) i))
                               (map-indexed vector grp))]
              [g s]))
          (rseq stack))))

(defn- canon
  "α-canonical walk with a binder-group stack. LAMBDA params and (recursive)
  LET names are α-quotiented to de-Bruijn refs (their names are local, so
  x:x ≡ y:y and `let x=1;in x` ≡ `let y=1;in y`), with correct shadowing (the
  nearest binder wins). ATTRSET keys are KEPT (record labels are observable --
  not α-renameable) but sorted (order-independent). Free vars keep their names
  (sound for open terms -- no unsound open-subterm de-Bruijn-Merkle)."
  [ast stack]
  (let [ast (if (and (map? ast) (:span ast)) (unparse/strip-positions ast) ast)]
    (cond
      (and (map? ast) (= :var (:op ast)))
      (if-let [[g s] (bound-ref stack (:name ast))]
        {:op :bound :group g :slot s}
        {:op :var :name (:name ast)})

      (and (map? ast) (= :lambda (:op ast)) (:param ast))
      {:op :lambda :body (canon (:body ast) (conj stack [(:param ast)]))}

      (and (map? ast) (= :let (:op ast)))
      ;; bindings keep a {:value ...} shape (names dropped = α, wrapper kept so
      ;; canonical-form is IDEMPOTENT); the group uses the pre-canon names
      ;; (already-canonical bindings have none, but their refs are already
      ;; :bound so the group is unused).
      (let [bs (sort-by :name (:bindings ast))
            st (conj stack (mapv :name bs))]
        {:op :let
         :bindings (mapv (fn [b] {:value (canon (:value b) st)}) bs)
         :body (canon (:body ast) st)})

      (and (map? ast) (= :attrset (:op ast)))
      (assoc ast :attrs (->> (:attrs ast)
                             (map (fn [a] (update a :value #(canon % stack))))
                             (sort-by :key) vec))

      (map? ast)    (into (empty ast) (map (fn [[k v]] [k (canon v stack)])) ast)
      (vector? ast) (mapv #(canon % stack) ast)
      (seq? ast)    (map #(canon % stack) ast)
      :else ast)))

(defn canonical-form
  "Deterministic α-CANONICAL form of a pnix AST: positions stripped,
  order-independent binder groups sorted (attrset by key, let by name), AND
  lambda/let binders α-quotiented to de-Bruijn refs (with correct shadowing).
  Two α-equivalent terms ⇒ identical bytes; distinct terms ⇒ different bytes
  (free vars and attrset labels, being observable, are kept)."
  [ast]
  (canon ast []))

(defn term-hash
  "The content hash (term-key) of a pnix AST -- sha256 over its canonical form.
  PROPOSE filter only: confirm equality with `structurally-equivalent?`."
  [ast]
  (hash/data-hash (canonical-form ast)))

(defn structurally-equivalent?
  "Exact confirmation that two ASTs are the same term (up to binder ORDER,
  positions, and α-renaming of lambda/let binders). This is what a hash-hit
  must be checked against -- never trust the hash alone."
  [a b]
  (= (canonical-form a) (canonical-form b)))

(def ^{:doc "α-equivalence: two terms equal up to bound-variable renaming (and
  binder order + positions). The exact confirmation for a content-hash hit."}
  alpha-equivalent? structurally-equivalent?)

;; ---- content-addressed term store ---------------------------------------

(defn empty-store
  []
  {})

(defonce ^:private the-store (atom (empty-store)))

(defn put-term!
  "Store a term by its content hash. Returns {:key hash :status :stored|:hit|
  :collision|:impure}. A hash-hit is CONFIRMED by exact structural equality; a
  genuine hash collision (astronomically unlikely with sha256) is reported, not
  silently merged."
  [ast]
  (if-not (pure-term? ast)
    {:status :impure :reason :non-edn-term}
    (let [k (term-hash ast)
          canon (canonical-form ast)
          existing (get @the-store k)]
      (cond
        (nil? existing)
        (do (swap! the-store assoc k {:canonical canon :key k})
            {:key k :status :stored})

        (= (:canonical existing) canon)
        {:key k :status :hit}

        :else
        {:key k :status :collision :reason :hash-collision-distinct-terms}))))

(defn get-term
  "Fetch the canonical term for a content hash, or nil."
  [k]
  (:canonical (get @the-store k)))

(defn has-term?
  [k]
  (contains? @the-store k))

(defn term-count
  []
  (count @the-store))

(defn clear-store!
  []
  (reset! the-store (empty-store)))

;; ---- report --------------------------------------------------------------

(def store-cases
  "Sources exercising canonicalization: order-independence (attrset/let) must
  DEDUP; distinct terms must not; a re-store is a confirmed hit."
  [{:id :attrset-order      :a "{ a = 1; b = 2; }" :b "{ b = 2; a = 1; }" :same? true}
   {:id :let-order          :a "let a = 1; b = 2; in a + b"
                            :b "let b = 2; a = 1; in a + b" :same? true}
   {:id :nested-attrset     :a "{ x = { p = 1; q = 2; }; }"
                            :b "{ x = { q = 2; p = 1; }; }" :same? true}
   {:id :distinct-values    :a "{ a = 1; }" :b "{ a = 2; }" :same? false}
   {:id :distinct-structure :a "1 + 2"      :b "1 * 2"      :same? false}
   ;; §3b: alpha-equivalence now DEDUPS (x:x == y:y), with correct shadowing
   {:id :alpha-lambda       :a "x: x"       :b "y: y"       :same? true}
   {:id :alpha-let          :a "let a = 1; in a" :b "let z = 1; in z" :same? true}
   {:id :alpha-shadowing    :a "x: (x: x)"  :b "y: (z: z)"  :same? true}
   {:id :alpha-free-kept    :a "x: x + y"   :b "z: z + w"   :same? false}
   {:id :attrset-labels-kept :a "{ a = 1; }" :b "{ z = 1; }" :same? false}])

(defn- run-case
  [{:keys [id a b same? note]}]
  (let [parse (requiring-resolve 'pnix-clj.parser/parse-source)
        aa (:ast (parse a)) bb (:ast (parse b))
        equiv? (structurally-equivalent? aa bb)
        hash-eq? (= (term-hash aa) (term-hash bb))]
    {:id id :status (if (= same? equiv?) :accepted :rejected)
     :structurally-equivalent? equiv?
     ;; the invariant: hash-equality must AGREE with confirmed structural
     ;; equality on these (no collisions) -- hash is the propose filter.
     :hash-agrees-with-structural? (= equiv? hash-eq?)
     :note note}))

(defn report
  []
  (let [rows (mapv run-case store-cases)
        rejected (count (remove #(= :accepted (:status %)) rows))
        ;; round-trip: every source stores, re-stores as a confirmed :hit
        rt (let [parse (requiring-resolve 'pnix-clj.parser/parse-source)]
             (doall
              (for [{:keys [a]} store-cases
                    :let [_ (clear-store!)                 ; each case independent
                          ast (:ast (parse a))
                          s1 (put-term! ast) s2 (put-term! ast)]]
                (and (= :stored (:status s1))
                     (= :hit (:status s2))
                     (= (get-term (:key s1)) (canonical-form ast))))))
        body {:kind :pnix-cas-report
              :schema :pnix-clj.cas-report.v0
              :policy :canonicalize-then-content-address-hash-is-propose-filter
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :hash-agrees-with-structural?
              (every? :hash-agrees-with-structural? rows)
              :store-roundtrip-ok? (every? true? rt)
              :rows rows}]
    (assoc body
           :status (if (and (zero? rejected)
                            (:hash-agrees-with-structural? body)
                            (:store-roundtrip-ok? body))
                     :ok :failed)
           :report-hash (hash/data-hash rows))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected store-roundtrip-ok?]} (report)]
    (println (format "pnix-clj cas: status=%s total=%d accepted=%d rejected=%d roundtrip=%s"
                     (name status) total accepted rejected store-roundtrip-ok?))
    (shutdown-agents)))
