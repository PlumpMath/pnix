(ns pnix-clj.cached-eval
  "Content-addressed evaluation cache (roadmap M6), role-translated from
  pnix-hy's cached_eval: purity + determinism make memoizing by CANONICAL
  CONTENT sound. The key hashes the position-stripped AST (M1's
  strip-positions), so whitespace/paren/span differences share one entry —
  genuine content addressing, not source-string equality. Follows the
  lowering-cache idiom (schemad key, clear!, stats). Guards, all reused:
  only sources that pass the M5 static purity check, evaluate to :ok, and
  yield plain data values are cached; everything else BYPASSES with a reason
  (never a wrong answer from a cache). origin/main's cas.clj is a different
  design line (PORT reference only, deliberately not read — branch reality)."
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]
            [pnix-clj.parser :as parser]
            [pnix-clj.safe-eval :as safe-eval]
            [pnix-clj.unparse :as unparse]))

(def lane-classification
  {:lane :core
   :scope :content-addressed-eval-cache
   :role :deterministic-eval-acceleration
   :product-runtime :allowed
   :semantic-authority :forbidden
   :mutation :cache-state-only
   :admission :forbidden
   :purity :required-before-cache
   :determinism :fresh-cross-check-required
   :allowed-output :cached-or-fresh-eval-result})

(def cache-epoch
  "Bump when evaluator semantics change in a way that invalidates cached
  values (the gate's determinism/conformance lanes catch when this is
  forgotten: cached==fresh is cross-checked per report run)."
  1)

(def ^:private eval-cache (atom {}))
(def ^:private stats* (atom {:hits 0 :misses 0 :bypasses 0}))

(defn clear-eval-cache!
  []
  (reset! eval-cache {})
  (reset! stats* {:hits 0 :misses 0 :bypasses 0}))

(defn eval-cache-stats
  []
  (assoc @stats* :entries (count @eval-cache)))

(defn cache-key
  "Content-addressed key: position-stripped AST hash + epoch."
  [ast]
  {:schema :pnix-clj.eval-cache-key.v0
   :content-hash (hash/data-hash (unparse/strip-positions ast))
   :epoch cache-epoch})

(defn- cacheable-value?
  "Plain data only — callables/thunks and tagged runtime values must not be
  replayed from a cache."
  [v]
  (cond
    (or (integer? v) (float? v) (boolean? v) (nil? v) (string? v)) true
    (vector? v) (every? cacheable-value? v)
    (map? v) (and (not (contains? v :kind))
                  (every? (fn [[k x]] (and (string? k) (cacheable-value? x))) v))
    :else false))

(defn- bypass
  [result reason]
  (swap! stats* update :bypasses inc)
  (assoc result :cache {:status :bypass :reason reason}))

(defn cached-eval
  "Evaluate `source` with content-addressed memoization. The result carries
  :cache {:status :hit|:miss|:bypass ...}; a bypass is always evaluated
  fresh, so the cache can never change an answer, only skip recomputation."
  [source]
  (let [{:keys [status ast] :as parsed} (parser/parse-source (str source))]
    (if (not= :ok status)
      (bypass parsed :parse-failed)
      (let [purity (safe-eval/static-purity-check source)]
        (if-not (:pure? purity)
          (bypass (pnix/eval-source source) :statically-impure)
          (let [k (cache-key ast)]
            (if-let [hit (get @eval-cache k)]
              (do (swap! stats* update :hits inc)
                  (assoc hit :cache {:status :hit :key k}))
              (let [result (pnix/eval-source source)]
                (if (and (= :ok (:status result))
                         (cacheable-value? (:value result)))
                  (let [entry (select-keys result [:status :value])]
                    (swap! eval-cache assoc k entry)
                    (swap! stats* update :misses inc)
                    (assoc entry :cache {:status :miss :key k}))
                  (bypass result (if (= :ok (:status result))
                                   :value-not-cacheable
                                   :result-not-ok)))))))))))

;; --- report ---------------------------------------------------------------------

(defn- corpus-cross-check
  "Determinism cross-check on a corpus sample: cached (miss then hit) must
  equal a fresh uncached evaluation, value for value."
  [sources]
  (mapv (fn [s]
          (let [r1 (cached-eval s)
                r2 (cached-eval s)
                fresh (pnix/eval-source s)
                ok? (and (= :miss (get-in r1 [:cache :status]))
                         (= :hit (get-in r2 [:cache :status]))
                         (= (:value r1) (:value r2) (:value fresh)))]
            {:source s
             :status (if ok? :accepted :rejected)
             :first (get-in r1 [:cache :status])
             :second (get-in r2 [:cache :status])
             :value (:value r1)}))
        sources))

(defn report
  []
  (clear-eval-cache!)
  (let [mirror-cases (requiring-resolve 'pnix-clj.mirror-pair/cases)
        corpus (into []
                     (comp (map :source)
                           (filter #(:pure? (safe-eval/static-purity-check %)))
                           (take 25))
                     (mirror-cases))
        cross (corpus-cross-check corpus)
        _ (clear-eval-cache!)
        content-a (cached-eval "1 + 2")
        content-b (cached-eval "  1   +   2  ")
        content-c (cached-eval "(1 + 2)")
        content-addressed? (and (= :miss (get-in content-a [:cache :status]))
                                (= :hit (get-in content-b [:cache :status]))
                                (= :hit (get-in content-c [:cache :status]))
                                (= 3 (:value content-a) (:value content-b)
                                   (:value content-c)))
        impure (cached-eval "builtins.getEnv \"HOME\"")
        impure-bypassed? (= :statically-impure
                            (get-in impure [:cache :reason]))
        held (cached-eval "1 / 0")
        held-bypassed? (= :result-not-ok (get-in held [:cache :reason]))
        closure (cached-eval "x: x")
        closure-bypassed? (= :value-not-cacheable
                             (get-in closure [:cache :reason]))
        checks [{:id :content-addressing :ok? content-addressed?}
                {:id :impure-bypasses :ok? impure-bypassed?}
                {:id :held-not-cached :ok? held-bypassed?}
                {:id :closure-not-cached :ok? closure-bypassed?}]
        rejected (+ (count (remove #(= :accepted (:status %)) cross))
                    (count (remove :ok? checks)))
        body {:kind :pnix-cached-eval-report
              :schema :pnix-clj.cached-eval-report.v0
              :policy :content-addressed-memoization-purity-guarded
              :epoch cache-epoch
              :corpus-total (count cross)
              :checks checks
              :total (+ (count cross) (count checks))
              :accepted (- (+ (count cross) (count checks)) rejected)
              :rejected rejected
              :stats (eval-cache-stats)
              :rows cross}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash [cross checks]))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected checks stats]} (report)]
    (println (format "pnix-clj cached-eval: status=%s total=%d accepted=%d rejected=%d stats=%s"
                     (name status) total accepted rejected (pr-str stats)))
    (doseq [{:keys [id ok?]} checks]
      (println (format "  [%s] %s" (if ok? "OK" "REJECT") (name id))))
    (shutdown-agents)))
