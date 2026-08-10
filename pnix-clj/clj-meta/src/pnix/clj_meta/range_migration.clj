(ns pnix.clj-meta.range-migration
  "M9 range-ledger migration witness.

  This receipt replays the checked-long range proof ledger through the M9
  abstract interval API. It does not admit raw opcodes by itself; it proves that
  the existing M6z..M6ai direct-lowering cases can be represented by the new
  interval transfer substrate, while overflow sentinels remain unproven."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.abstract-interval :as ai])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/range-migration.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- singleton-env
  [m]
  (into (sorted-map)
        (map (fn [[k v]] [k (ai/singleton v)]) m)))

(defn- bottom-env?
  [env]
  (some ai/bottom? (vals env)))

(defn- assign-all
  [env assignments]
  (reduce (fn [e [local expr]]
            (ai/assign e local expr))
          env
          assignments))

(defn- assign-parallel
  [env assignments]
  (reduce (fn [e [local expr]]
            (assoc e local (ai/eval-expr env expr)))
          env
          assignments))

(defn- replay-loop
  [initial guard assignments max-steps]
  (loop [env initial
         steps 0
         rows []]
    (let [guarded (ai/guard env guard)
          row     {:step steps
                   :env env
                   :guarded guarded}]
      (cond
        (bottom-env? guarded)
        {:status :closed
         :steps steps
         :exit-env env
         :rows (conj rows row)}

        (>= steps max-steps)
        {:status :suspended
         :steps steps
         :exit-env env
         :rows (conj rows row)
         :suspension-reason :resource-budget-exhausted}

        :else
        (recur (assign-parallel guarded assignments)
               (inc steps)
               (conj rows (assoc row
                                 :next-env
                                 (assign-parallel guarded assignments))))))))

(defn- expr-result
  [env expr]
  (ai/eval-expr env expr))

(defn- expr-safe?
  [env expr]
  (ai/finite? (expr-result env expr)))

(defn- expression-case
  [{:keys [id desc env exprs expected safe?]}]
  (let [results (mapv #(expr-result env %) exprs)
        safe    (every? true? (map #(expr-safe? env %) exprs))
        ok?     (and (= results expected)
                     (= safe safe?))]
    {:id id
     :kind :expression-ledger
     :desc desc
     :env env
     :exprs exprs
     :results results
     :expected expected
     :safe? safe
     :expected-safe? safe?
     :ok ok?}))

(defn- loop-case
  [{:keys [id desc initial guard assignments exprs expected-exit expected max-steps]}]
  (let [replay  (replay-loop initial guard assignments max-steps)
        exit    (:exit-env replay)
        results (mapv #(expr-result exit %) exprs)
        safe    (and (= :closed (:status replay))
                     (every? true? (map #(expr-safe? exit %) exprs)))
        ok?     (and (= :closed (:status replay))
                     (= expected-exit exit)
                     (= expected results)
                     safe)]
    {:id id
     :kind :loop-ledger
     :desc desc
     :initial initial
     :guard guard
     :assignments assignments
     :steps (:steps replay)
     :exit-env exit
     :expected-exit expected-exit
     :exprs exprs
     :results results
     :expected expected
     :safe? safe
     :ok ok?}))

(defn- sentinel-case
  [{:keys [id desc env exprs expected]}]
  (let [results (mapv #(expr-result env %) exprs)
        safe    (every? true? (map #(expr-safe? env %) exprs))
        ok?     (and (= expected results)
                     (false? safe))]
    {:id id
     :kind :fallback-sentinel
     :desc desc
     :env env
     :exprs exprs
     :results results
     :expected expected
     :safe? safe
     :expected-safe? false
     :fallback :numbers-primitive-descriptor-calls
     :ok ok?}))

(defn- run-case
  [spec]
  (case (:kind spec)
    :expression (expression-case spec)
    :loop (loop-case spec)
    :sentinel (sentinel-case spec)))

(defn- specs
  []
  [{:kind :expression
    :id :m6z-literal-checked-long
    :desc "literal checked-long + - * no-overflow proofs replay through interval transfer"
    :env {}
    :exprs [[:+ 20 22] [:- 50 8] [:* 6 7]]
    :expected [(ai/singleton 42) (ai/singleton 42) (ai/singleton 42)]
    :safe? true}
   {:kind :expression
    :id :m6aa-let-local-checked-long
    :desc "let-local range ledger replays as Var->Interval transfer"
    :env (assign-all (singleton-env {:a 20 :b 22})
                     [[:c [:+ :a :b]]])
    :exprs [[:+ :a :b] [:- :c :a] [:* :b 2]]
    :expected [(ai/singleton 42) (ai/singleton 22) (ai/singleton 44)]
    :safe? true}
   {:kind :expression
    :id :m6ab-loop-invariant-checked-long
    :desc "loop-invariant locals replay as stable interval environment"
    :env (singleton-env {:a 20 :b 22})
    :exprs [[:+ :a :b] [:- :b :a] [:* :a 2]]
    :expected [(ai/singleton 42) (ai/singleton 2) (ai/singleton 40)]
    :safe? true}
   {:kind :loop
    :id :m6ac-bounded-unit-step-loop
    :desc "positive bounded unit-step loop exit range replays exactly"
    :initial (singleton-env {:i 0})
    :guard [:< :i 10]
    :assignments [[:i [:+ :i 1]]]
    :max-steps 16
    :expected-exit (singleton-env {:i 10})
    :exprs [[:+ :i 20] [:- 30 :i] [:* :i 2]]
    :expected [(ai/singleton 30) (ai/singleton 20) (ai/singleton 20)]}
   {:kind :loop
    :id :m6ad-positive-non-unit-stride-loop
    :desc "positive non-unit stride loop exit range replays exactly"
    :initial (singleton-env {:i 0})
    :guard [:< :i 10]
    :assignments [[:i [:+ :i 3]]]
    :max-steps 16
    :expected-exit (singleton-env {:i 12})
    :exprs [[:+ :i 20] [:- 40 :i] [:* :i 2]]
    :expected [(ai/singleton 32) (ai/singleton 28) (ai/singleton 24)]}
   {:kind :loop
    :id :m6ae-decreasing-stride-loop
    :desc "decreasing stride loop exit range replays exactly"
    :initial (singleton-env {:i 10})
    :guard [:> :i 0]
    :assignments [[:i [:- :i 3]]]
    :max-steps 16
    :expected-exit (singleton-env {:i -2})
    :exprs [[:+ :i 20] [:- 40 :i] [:* :i 2]]
    :expected [(ai/singleton 18) (ai/singleton 42) (ai/singleton -4)]}
   {:kind :loop
    :id :m6af-constant-accumulator-loop
    :desc "constant accumulator recurrence replays through interval transfer"
    :initial (singleton-env {:i 0 :acc 0})
    :guard [:< :i 10]
    :assignments [[:i [:+ :i 1]]
                  [:acc [:+ :acc 2]]]
    :max-steps 16
    :expected-exit (singleton-env {:acc 20 :i 10})
    :exprs [[:+ :acc 20] [:- 50 :acc] [:* :acc 2]]
    :expected [(ai/singleton 40) (ai/singleton 30) (ai/singleton 40)]}
   {:kind :expression
    :id :m6ag-branch-local-fn-guard
    :desc "nested fn argument guards narrow a top long range before checked arithmetic"
    :env (-> {:x ai/top}
             (ai/guard [:> :x -10])
             (ai/guard [:< :x 10]))
    :exprs [[:+ :x 1] [:- 10 :x] [:* :x 2]]
    :expected [(ai/interval -8 10)
               (ai/interval 1 19)
               (ai/interval -18 18)]
    :safe? true}
   {:kind :loop
    :id :m6ah-index-accumulator-loop
    :desc "bounded index-accumulator recurrence replays exact arithmetic-series result"
    :initial (singleton-env {:i 0 :acc 0})
    :guard [:< :i 10]
    :assignments [[:acc [:+ :acc :i]]
                  [:i [:+ :i 1]]]
    :max-steps 16
    :expected-exit (singleton-env {:acc 45 :i 10})
    :exprs [[:+ :acc 1] [:- 50 :acc] [:* :acc 2]]
    :expected [(ai/singleton 46) (ai/singleton 5) (ai/singleton 90)]}
   {:kind :loop
    :id :m6ai-multiplicative-accumulator-loop
    :desc "positive multiplicative accumulator recurrence replays exact factor-power result"
    :initial (singleton-env {:i 0 :acc 1})
    :guard [:< :i 5]
    :assignments [[:i [:+ :i 1]]
                  [:acc [:* :acc 2]]]
    :max-steps 16
    :expected-exit (singleton-env {:acc 32 :i 5})
    :exprs [[:+ :acc 1] [:- 100 :acc] [:* :acc 2]]
    :expected [(ai/singleton 33) (ai/singleton 68) (ai/singleton 64)]}
   {:kind :sentinel
    :id :overflow-literal-fallback
    :desc "overflow literal checked arithmetic remains unproven"
    :env {}
    :exprs [[:+ Long/MAX_VALUE 1]]
    :expected [ai/top]}
   {:kind :sentinel
    :id :overflow-multiplicative-accumulator-fallback
    :desc "overflowing accumulator recurrence becomes top and keeps checked fallback"
    :env (singleton-env {:acc 4611686018427387904})
    :exprs [[:* :acc 2]]
    :expected [ai/top]}])

(defn run
  []
  (let [cases      (mapv run-case (specs))
        canonical  (mapv #(select-keys %
                                       [:id
                                        :kind
                                        :results
                                        :expected
                                        :safe?
                                        :ok])
                         cases)
        status     (frequencies (map :kind cases))
        invariants (sorted-map
                    :all-cases-ok (every? :ok cases)
                    :all-direct-cases-safe
                    (every? true?
                            (map :safe?
                                 (remove #(= :fallback-sentinel (:kind %))
                                         cases)))
                    :fallback-sentinels-not-safe
                    (every? false?
                            (map :safe?
                                 (filter #(= :fallback-sentinel (:kind %))
                                         cases)))
                    :migration-covers-m6z-through-m6ai
                    (= #{:m6z-literal-checked-long
                         :m6aa-let-local-checked-long
                         :m6ab-loop-invariant-checked-long
                         :m6ac-bounded-unit-step-loop
                         :m6ad-positive-non-unit-stride-loop
                         :m6ae-decreasing-stride-loop
                         :m6af-constant-accumulator-loop
                         :m6ag-branch-local-fn-guard
                         :m6ah-index-accumulator-loop
                         :m6ai-multiplicative-accumulator-loop}
                       (set (map :id
                                 (remove #(= :fallback-sentinel (:kind %))
                                         cases)))))
        ok?        (and (every? :ok cases)
                        (every? true? (vals invariants)))]
    {:schema "pnix.clj-meta.range-migration.receipt.v1"
     :stage [:M9 :M9-migration]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "M6z..M6ai checked-long range ledger replayed through M9 abstract interval transfer"
     :policy {:direct-cases "finite interval results may feed later translation validation"
              :fallback-sentinels "top/unknown is not accepted as raw opcode proof"
              :compiler-admission "existing compiler lowering unchanged in this pass"}
     :status-counts status
     :cases cases
     :invariants invariants
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok ok?}))

(defn write-receipt!
  [r]
  (io/make-parents receipt-path)
  (spit receipt-path (with-out-str (pp/pprint r)))
  r)

(defn -main
  [& _]
  (let [r (write-receipt! (run))]
    (doseq [row (:cases r)]
      (println (str "  [" (if (:ok row) "OK" "FAIL") "] "
                    (name (:id row)))))
    (println (str "range migration: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
