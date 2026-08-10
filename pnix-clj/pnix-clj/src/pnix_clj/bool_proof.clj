(ns pnix-clj.bool-proof
  "PROVEN equivalence for the pnix boolean fragment (true/false, ! && || ->,
  over boolean variables) by EXHAUSTIVE truth-table evaluation -- a complete
  proof over the finite boolean domain, not a sample.

  For an expression over n boolean variables there are 2^n assignments; two
  expressions are PROVABLY equal iff they agree on ALL of them. Evaluation
  delegates to the real pnix evaluator (substitute each assignment as a let),
  so the proof is grounded in the actual semantics, not a re-implementation.

  Companion to pnix-clj.arith-proof (polynomial normal form for arithmetic):
  together they give PROVEN -- not merely tested -- equivalence for the two
  decidable fragments. Honest boundary: an expression whose vars exceed the
  bound, or that evaluates to a non-boolean, is reported :unprovable, never a
  false claim."
  (:require [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :boolean-fragment-equivalence-proof
   :product-runtime :forbidden
   :optimizer-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :proof-boundary :bounded-truth-table-fragment-only
   :default-on-non-fragment :unprovable
   :allowed-output :boolean-proof-report})

(def ^:private max-vars 8)

(defn free-vars
  "The set of variable names in a parsed AST (any :var node)."
  [ast]
  (cond
    (map? ast) (let [here (when (= :var (:op ast)) #{(:name ast)})]
                 (into (or here #{})
                       (mapcat free-vars (vals (dissoc ast :op :span :source-hash)))))
    (sequential? ast) (into #{} (mapcat free-vars ast))
    :else #{}))

(defn- assignments
  "All 2^n true/false maps over `vars`."
  [vars]
  (let [vs (vec vars)]
    (reduce (fn [acc v]
              (for [m acc b [true false]] (assoc m v b)))
            [{}]
            vs)))

(defn- bool-literal
  [b]
  (if b "true" "false"))

(defn- eval-under
  [source env]
  (let [eval-source (requiring-resolve 'pnix-clj.core/eval-source)
        wrapped (if (empty? env)
                  (str "(" source ")")
                  (str "let "
                       (apply str (map (fn [[k v]] (str k " = " (bool-literal v) "; ")) env))
                       " in (" source ")"))]
    (eval-source wrapped)))

(defn prove-equivalent
  "Prove (by exhaustive truth table) whether two boolean sources are equal for
  ALL assignments. Returns {:status :proven | :refuted | :unprovable ...}."
  [src-a src-b]
  (let [parse (requiring-resolve 'pnix-clj.parser/parse-source)
        pa (parse src-a) pb (parse src-b)]
    (if (or (not= :ok (:status pa)) (not= :ok (:status pb)))
      {:status :unprovable :reason :parse-failed}
      (let [vars (into (free-vars (:ast pa)) (free-vars (:ast pb)))]
        (if (> (count vars) max-vars)
          {:status :unprovable :reason :too-many-vars :var-count (count vars)}
          (loop [remaining (assignments vars)]
            (if-let [env (first remaining)]
              (let [ra (eval-under src-a env)
                    rb (eval-under src-b env)]
                (cond
                  (or (not= :ok (:status ra)) (not= :ok (:status rb)))
                  {:status :unprovable :reason :evaluation-held :assignment env}

                  (or (not (boolean? (:value ra))) (not (boolean? (:value rb))))
                  {:status :unprovable :reason :non-boolean-value :assignment env}

                  (not= (:value ra) (:value rb))
                  {:status :refuted :assignment env
                   :value-a (:value ra) :value-b (:value rb)}

                  :else (recur (rest remaining))))
              {:status :proven
               :vars (vec (sort vars))
               :assignments-checked (bit-shift-left 1 (count vars))})))))))

(def proof-cases
  "Pairs that must be PROVEN equal (classic boolean identities)."
  [{:id :commute-and   :a "a && b"          :b "b && a"}
   {:id :commute-or    :a "a || b"          :b "b || a"}
   {:id :de-morgan-1   :a "(!(a && b))"     :b "((!a) || (!b))"}
   {:id :de-morgan-2   :a "(!(a || b))"     :b "((!a) && (!b))"}
   {:id :double-neg    :a "(!(!a))"         :b "a"}
   {:id :absorption    :a "(a || (a && b))" :b "a"}
   {:id :impl-as-or    :a "(a -> b)"        :b "((!a) || b)"}
   {:id :distribute    :a "(a && (b || c))" :b "((a && b) || (a && c))"}])

(def refute-cases
  [{:id :and-not-or :a "a && b" :b "a || b"}
   {:id :not-id     :a "!a"     :b "a"}])

(defn- run-case
  [{:keys [id a b]} expect]
  (let [r (prove-equivalent a b)]
    {:id id :a a :b b
     :status (if (= expect (:status r)) :accepted :rejected)
     :proof-status (:status r)}))

(defn report
  []
  (let [rows (into (mapv #(run-case % :proven) proof-cases)
                   (mapv #(run-case % :refuted) refute-cases))
        rejected (count (remove #(= :accepted (:status %)) rows))
        body {:kind :pnix-bool-proof-report
              :schema :pnix-clj.bool-proof-report.v0
              :policy :exhaustive-truth-table-proven-equivalence-boolean-fragment
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
    (println (format "pnix-clj bool-proof: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (doseq [{:keys [id proof-status a b]} rows]
      (println (format "  [%s] %-14s %s <=> %s"
                       (name proof-status) (name id) (pr-str a) (pr-str b))))
    (shutdown-agents)))
