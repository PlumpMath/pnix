(ns pnix-clj.safe-eval
  "Resource-limited pure evaluation tier (roadmap M5), role-translated from
  pnix-hy's safe_eval: evaluation here is a sandbox BY DESIGN (impure builtins
  are purity-gated at runtime) and this tier ADDS explicit limits with a
  structured verdict instead of a crash —
    {:status .. :value .. :limit-exceeded :fuel|:impure ..}
  Reuses the M1 fuel seam (evaluator/*fuel* via eval-ast-with-fuel, fixed
  8MB-stack thread) — nothing is re-implemented. static-purity-check is a
  conservative AST walk (no evaluation): it reports builtins.<impure> selects
  and bare impure names, and flags dynamic builtins access as undecidable."
  (:require [clojure.string :as str]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.parser :as parser]))

(def lane-classification
  {:lane :core
   :scope :bounded-safe-evaluation-gate
   :role :fuel-limited-purity-gated-eval-tier
   :product-runtime :allowed
   :semantic-authority :bounded-gate-only
   :mutation :forbidden
   :admission :forbidden
   :purity :static-and-runtime-gated
   :determinism :required-upstream
   :allowed-output :safe-eval-result-or-limit-verdict})

(def default-fuel 100000)

;; --- static purity check -------------------------------------------------------

(defn- impure-select?
  "builtins.<impure-name> as a static select."
  [{:keys [op target attr]}]
  (and (= :select op)
       (= :var (:op target))
       (= "builtins" (:name target))
       (string? attr)
       (contains? evaluator/impure-builtins attr)))

(defn- dynamic-builtins-select?
  "builtins.${..} — statically undecidable, reported conservatively."
  [{:keys [op target attr]}]
  (and (= :select op)
       (= :var (:op target))
       (= "builtins" (:name target))
       (map? attr)))

(defn- walk-impure
  [node acc]
  (cond
    (map? node)
    (let [acc (cond
                (impure-select? node)
                (conj acc {:builtin (:attr node)
                           :via :builtins-select
                           :span (:span node)})

                (dynamic-builtins-select? node)
                (conj acc {:builtin :dynamic
                           :via :dynamic-builtins-attr
                           :span (:span node)})

                (and (= :var (:op node))
                     (contains? evaluator/impure-builtins (:name node)))
                ;; not bound unprefixed today, but reachable via `with builtins;`
                (conj acc {:builtin (:name node)
                           :via :bare-name
                           :span (:span node)})

                :else acc)]
      (reduce (fn [a v] (walk-impure v a)) acc (vals node)))

    (sequential? node)
    (reduce (fn [a v] (walk-impure v a)) acc node)

    :else acc))

(defn static-purity-check
  "Conservative static purity report — no evaluation. {:status :ok :pure? bool
  :impure-uses [..]} or the parse failure."
  [source]
  (let [{:keys [status ast] :as parsed} (parser/parse-source (str source))]
    (if (not= :ok status)
      parsed
      (let [uses (walk-impure ast [])]
        {:status :ok
         :pure? (empty? uses)
         :impure-uses (vec uses)}))))

;; --- safe-eval ------------------------------------------------------------------

(defn- purity-gated-reason?
  [reason]
  (and (keyword? reason)
       (str/ends-with? (name reason) "purity-gated")))

(defn- effect-denied
  [reason evidence]
  {:status :failed
   :reason :effect-denied
   :error {:phase :effect
           :class :effect-denied
           :evidence (merge {:policy-reason reason} evidence)}})

(defn safe-eval
  "Evaluate `source` in the pure sandbox under a fuel budget. Verdicts:
  - fuel exhausted        -> {:status :suspended :limit-exceeded :fuel :fuel n}
  - pure-only? static hit -> Failed(effect, effect-denied)
  - runtime purity gate   -> Failed(effect, effect-denied)
  - otherwise             -> the evaluation result unchanged."
  ([source] (safe-eval source {}))
  ([source {:keys [fuel pure-only?]
            :or {fuel default-fuel pure-only? false}}]
   (let [{:keys [status ast] :as parsed} (parser/parse-source (str source))]
     (cond
       (not= :ok status)
       parsed

       (and pure-only?
            (let [{:keys [pure?]} (static-purity-check source)]
              (not pure?)))
       (let [uses (:impure-uses (static-purity-check source))]
         (merge (effect-denied :static-impure-use
                               {:impure-uses uses})
                {:limit-exceeded :impure
                 :impure-uses uses}))

       :else
       (let [r (binding [evaluator/*pure-eval* true]
                 (evaluator/eval-ast-with-fuel ast fuel))]
         (cond
           (= :suspended (:status r))
           (assoc r :limit-exceeded :fuel :fuel fuel)

           (purity-gated-reason? (:reason r))
           (merge (effect-denied (:reason r) {})
                  {:limit-exceeded :impure})

           :else r))))))

;; --- report ---------------------------------------------------------------------

(def cases
  [{:id :pure-value
    :source "1 + 2 * 3"
    :opts {}
    :expect {:status :ok :value 7}}
   {:id :fuel-exhaustion-is-a-verdict
    :source "let f = x: f x; in f 1"
    :opts {:fuel 4096}
    :expect {:status :suspended :limit-exceeded :fuel}}
   {:id :runtime-purity-gate-tagged
    :source "builtins.getEnv \"HOME\""
    :opts {}
    :expect {:status :failed :limit-exceeded :impure :reason :effect-denied}}
   {:id :pure-only-refuses-before-eval
    :source "builtins.readFile \"/etc/passwd\""
    :opts {:pure-only? true}
    :expect {:status :failed :limit-exceeded :impure :reason :effect-denied}}
   {:id :pure-only-passes-pure-source
    :source "builtins.length [ 1 2 3 ]"
    :opts {:pure-only? true}
    :expect {:status :ok :value 3}}
   {:id :static-check-reports-uses
    :source "if c then builtins.readFile p else builtins.getEnv \"X\""
    :static-check {:pure? false :count 2}}
   {:id :static-check-flags-dynamic-access
    :source "builtins.${k} \"/x\""
    :static-check {:pure? false}}
   {:id :static-check-pure
    :source "let a = 1; in a + 2"
    :static-check {:pure? true :count 0}}])

(defn- run-case
  [{:keys [id source opts expect static-check]}]
  (if static-check
    (let [r (static-purity-check source)
          ok? (and (= :ok (:status r))
                   (= (:pure? static-check) (:pure? r))
                   (or (not (contains? static-check :count))
                       (= (:count static-check) (count (:impure-uses r)))))]
      {:id id :source source :mode :static
       :status (if ok? :accepted :rejected)
       :result (select-keys r [:pure? :impure-uses])})
    (let [r (safe-eval source opts)
          ok? (every? (fn [[k v]] (= v (get r k))) expect)]
      {:id id :source source :mode :eval
       :status (if ok? :accepted :rejected)
       :result (select-keys r [:status :value :reason :limit-exceeded])})))

(defn report
  []
  (let [rows (mapv run-case cases)
        rejected (count (remove #(= :accepted (:status %)) rows))
        body {:kind :pnix-safe-eval-report
              :schema :pnix-clj.safe-eval-report.v0
              :policy :sandbox-by-design-plus-explicit-limits
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
    (println (format "pnix-clj safe-eval: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (doseq [{:keys [id status mode result]} rows]
      (println (format "  [%s] %s (%s) %s"
                       (if (= :accepted status) "OK" "REJECT")
                       (name id) (name mode) (pr-str result))))
    (shutdown-agents)))
