(ns pnix-clj.arith-proof
  "PROVEN equivalence for the pnix arithmetic fragment (+, -, *, int, var) via
  canonical polynomial normalization -- a genuine proof for ALL variable
  values, not a differential test on chosen inputs.

  Each arithmetic expression normalizes to a canonical polynomial (a map from
  monomial -> integer coefficient; a monomial is a sorted map var -> power).
  Two expressions are PROVABLY equal (forall assignments) iff their canonical
  polynomials are identical -- commutativity/associativity/distributivity/
  constant-folding are all absorbed by the normal form.

  This UPGRADES the specializer's soundness (M1) from generative testing to a
  proof on the arithmetic fragment: specialize(source, statics).residual is
  proven equal to source with the statics substituted, over all remaining
  (dynamic) variables. A proven-vs-heuristic boundary, in the pnix-clj
  constitution's spirit: honest about WHAT is proven (arithmetic only; a
  non-arithmetic residual is reported :unprovable, never a false claim)."
  (:require [pnix-clj.hash :as hash]
            [pnix-clj.parser :as parser]
            [pnix-clj.specialize :as specialize]))

(def lane-classification
  {:lane :proof-only
   :scope :arithmetic-fragment-equivalence-proof
   :product-runtime :forbidden
   :optimizer-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :proof-boundary :canonical-polynomial-fragment-only
   :default-on-non-fragment :unprovable
   :allowed-output :arithmetic-proof-report})

;; ---- canonical polynomial -----------------------------------------------
;; polynomial : {monomial -> coeff}, coeff nonzero
;; monomial   : {var -> power}, power positive ({} = the constant monomial)

(defn- poly-add
  [p q]
  (into {} (remove (comp zero? val))
        (merge-with + p q)))

(defn- poly-scale
  [p k]
  (if (zero? k) {} (into {} (map (fn [[m c]] [m (* c k)])) p)))

(defn- mono-mul
  [m n]
  (merge-with + m n))

(defn- poly-mul
  [p q]
  (reduce poly-add {}
          (for [[mp cp] p [mq cq] q]
            {(mono-mul mp mq) (* cp cq)})))

(defn poly-of
  "Canonical polynomial of an arithmetic AST, or nil if it uses any non-
  arithmetic construct (let/if/builtin/…): those cannot be proven here."
  [ast]
  (case (:op ast)
    ;; normalize: a zero constant is the empty polynomial (so `0` and `x - x`
    ;; and `0 - 0` all canonicalize identically -- zero-coeff monomials never
    ;; appear, matching poly-add/poly-substitute which drop them).
    :int (if (zero? (:value ast)) {} {{} (:value ast)})
    :var {{(:name ast) 1} 1}
    ;; unary negation: `-e` (a folded negative literal re-parses as :neg)
    :neg (some-> (poly-of (:expr ast)) (poly-scale -1))
    :binary (let [l (poly-of (:left ast))
                  r (poly-of (:right ast))]
              (when (and l r)
                (case (:operator ast)
                  "+" (poly-add l r)
                  "-" (poly-add l (poly-scale r -1))
                  "*" (poly-mul l r)
                  nil)))
    nil))

(defn poly-substitute
  "Substitute var->int-value assignments into a polynomial, yielding a
  polynomial over the remaining variables."
  [p env]
  (reduce poly-add {}
          (for [[m c] p]
            (let [[coeff-mult mono']
                  (reduce (fn [[k mm] [v pow]]
                            (if (contains? env v)
                              [(* k (long (Math/pow (env v) pow))) mm]
                              [k (assoc mm v pow)]))
                          [1 {}]
                          m)]
              {mono' (* c coeff-mult)}))))

(defn poly-of-source
  [source]
  (let [{:keys [status ast]} (parser/parse-source source)]
    (when (= :ok status) (poly-of ast))))

(defn equivalent?
  "True iff both sources are arithmetic and PROVABLY equal for all variable
  values (identical canonical polynomials)."
  [src-a src-b]
  (let [a (poly-of-source src-a)
        b (poly-of-source src-b)]
    (boolean (and a b (= a b)))))

;; ---- proven specializer soundness on the arithmetic fragment ------------

(defn prove-specialize-meaning
  "Prove (not test) that specialize preserved meaning on an arithmetic source:
  poly(source)[statics] == poly(residual), over all dynamic variables.
  Returns {:status :proven | :unprovable | :refuted ...}."
  [source statics]
  (let [sp (specialize/specialize source statics)]
    (if (not= :ok (:status sp))
      {:status :unprovable :reason (:reason sp)}
      (let [p-src (poly-of-source source)
            p-resid (poly-of-source (:residual-source sp))]
        (cond
          (or (nil? p-src) (nil? p-resid))
          {:status :unprovable :reason :non-arithmetic-fragment
           :residual-source (:residual-source sp)}

          (= (poly-substitute p-src statics) p-resid)
          {:status :proven
           :residual-source (:residual-source sp)
           :dynamic-vars (vec (sort (distinct (mapcat keys (keys p-resid)))))}

          :else
          {:status :refuted
           :residual-source (:residual-source sp)
           :source-poly (poly-substitute p-src statics)
           :residual-poly p-resid})))))

(def proof-cases
  [{:id :fold-const     :source "x + (2 + 3)"        :statics {}}
   {:id :static-var     :source "x + y"              :statics {"y" 7}}
   {:id :distribute     :source "(x + 1) * (x - 1)"  :statics {}}
   {:id :collect-terms  :source "2 * x + 3 * x"      :statics {}}
   {:id :static-mul     :source "a * x + b"          :statics {"a" 4 "b" 10}}
   {:id :all-static     :source "p * q + r"          :statics {"p" 2 "q" 3 "r" 1}}
   {:id :nested         :source "((x - y) * (x + y))" :statics {"y" 2}}])

(defn- run-case
  [{:keys [id source statics]}]
  (let [r (prove-specialize-meaning source statics)]
    {:id id :source source :statics statics
     :status (if (= :proven (:status r)) :accepted :rejected)
     :proof-status (:status r)
     :residual-source (:residual-source r)}))

(defn report
  []
  (let [rows (mapv run-case proof-cases)
        rejected (count (remove #(= :accepted (:status %)) rows))
        body {:kind :pnix-arith-proof-report
              :schema :pnix-clj.arith-proof-report.v0
              :policy :canonical-polynomial-proven-equivalence-arithmetic-fragment
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :rows rows}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash rows))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected rows]} (report)]
    (println (format "pnix-clj arith-proof: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (doseq [{:keys [id proof-status source residual-source]} rows]
      (println (format "  [%s] %-14s %s => %s"
                       (name proof-status) (name id)
                       (pr-str source) (pr-str residual-source))))
    (shutdown-agents)))
