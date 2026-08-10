(ns pnix.clj-meta.linear-ranking
  "M9b first linear ranking-function synthesis witness.

  This is a small BMS-style substrate, not a full Presburger solver. It
  enumerates affine templates over bounded integer loop states and validates
  that a candidate is non-negative on the guard and strictly decreases on every
  transition. Successful rows are proof inputs for later VC validation; they do
  not admit raw opcodes by themselves."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/linear-ranking.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- eval-expr
  [state expr]
  (cond
    (integer? expr) (long expr)
    (keyword? expr) (long (get state expr))
    (and (vector? expr) (= 3 (count expr)))
    (let [[op a b] expr
          av (eval-expr state a)
          bv (eval-expr state b)]
      (case op
        :+ (Math/addExact av bv)
        :- (Math/subtractExact av bv)))
    :else (throw (ex-info "unsupported ranking expr"
                          {:expr expr}))))

(defn- guard-satisfied?
  [state [op local bound]]
  (let [v (long (get state local))
        b (long bound)]
    (case op
      :< (< v b)
      :<= (<= v b)
      :> (> v b)
      :>= (>= v b))))

(defn- apply-update
  [state update]
  (reduce (fn [s [local expr]]
            (assoc s local (eval-expr state expr)))
          state
          update))

(defn- cartesian-states
  [bounds]
  (let [entries (sort-by (comp name key) bounds)]
    (reduce (fn [states [local [lo hi]]]
              (for [state states
                    v (range (long lo) (inc (long hi)))]
                (assoc state local v)))
            [{}]
            entries)))

(defn- cartesian-values
  [domains]
  (reduce (fn [rows domain]
            (for [row rows
                  v domain]
              (conj (vec row) v)))
          [[]]
          domains))

(defn- loop-states
  [{:keys [bounds guard]}]
  (filter #(guard-satisfied? % guard)
          (cartesian-states bounds)))

(defn- eval-linear
  [{:keys [coeffs const]} state]
  (reduce (fn [acc [local coeff]]
            (Math/addExact (long acc)
                           (Math/multiplyExact (long coeff)
                                               (long (get state local)))))
          (long const)
          coeffs))

(defn- candidate-space
  [locals]
  (let [coeff-values [-2 -1 0 1 2]
        const-values (range -20 21)
        locals (vec (sort-by name locals))]
    (sort-by (juxt (fn [{:keys [coeffs const]}]
                     (+ (Math/abs (long const))
                        (reduce + (map #(Math/abs (long %))
                                       (vals coeffs)))))
                   pr-str)
             (for [coeff-tuple (cartesian-values (repeat (count locals)
                                                         coeff-values))
                   :let [coeffs (into (sorted-map)
                                      (map vector locals coeff-tuple))]
                   :when (some (complement zero?) (vals coeffs))
                   const const-values]
               {:coeffs coeffs
                :const const}))))

(defn- transitions
  [spec state]
  (map #(apply-update state %) (:updates spec)))

(defn- valid-ranking?
  [spec candidate]
  (let [states (vec (loop-states spec))]
    (and (seq states)
         (every?
          (fn [state]
            (let [r (eval-linear candidate state)]
              (and (not (neg? r))
                   (every?
                    (fn [next-state]
                      (< (eval-linear candidate next-state) r))
                    (transitions spec state)))))
          states))))

(defn- synthesize
  [spec]
  (first (filter #(valid-ranking? spec %)
                 (candidate-space (keys (:bounds spec))))))

(defn- ranking-row
  [{:keys [id desc expected-candidate] :as spec}]
  (let [states (vec (loop-states spec))
        candidate (synthesize spec)
        ok? (= expected-candidate candidate)]
    {:id id
     :kind :linear-ranking-synthesis
     :desc desc
     :bounds (:bounds spec)
     :guard (:guard spec)
     :updates (:updates spec)
     :state-count (count states)
     :candidate candidate
     :expected-candidate expected-candidate
     :gate/verdict (if candidate :held :held)
     :held-reason (if candidate
                    :ranking-proof-input-no-compiler-admission
                    :no-ranking-function-found)
     :promotion/allowed? false
     :ok ok?}))

(defn- specs
  []
  [{:id :increasing-counter
    :desc "synthesize minimal R=9-i for i<10, i'=i+1"
    :bounds {:i [0 9]}
    :guard [:< :i 10]
    :updates [[[:i [:+ :i 1]]]]
    :expected-candidate {:coeffs {:i -1}
                         :const 9}}
   {:id :decreasing-counter
    :desc "synthesize R=i for i>0, i'=i-3"
    :bounds {:i [1 10]}
    :guard [:> :i 0]
    :updates [[[:i [:- :i 3]]]]
    :expected-candidate {:coeffs {:i 1}
                         :const 0}}
   {:id :branch-dependent-stride
    :desc "synthesize one minimal ranking for branch-dependent i'=i+1 or i'=i+2"
    :bounds {:i [0 9]}
    :guard [:< :i 10]
    :updates [[[:i [:+ :i 1]]]
              [[:i [:+ :i 2]]]]
    :expected-candidate {:coeffs {:i -1}
                         :const 9}}
   {:id :non-decreasing-sentinel
    :desc "no ranking accepted when the transition does not decrease"
    :bounds {:i [0 9]}
    :guard [:< :i 10]
    :updates [[[:i :i]]]
    :expected-candidate nil}])

(defn run
  []
  (let [rows (mapv ranking-row (specs))
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        held (filter #(= :held (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        canonical (mapv #(select-keys %
                                      [:id
                                       :kind
                                       :bounds
                                       :guard
                                       :updates
                                       :candidate
                                       :expected-candidate
                                       :gate/verdict
                                       :held-reason
                                       :promotion/allowed?
                                       :ok])
                        rows)
        invariants (sorted-map
                    :all-rows-ok (every? :ok rows)
                    :branch-dependent-ranking-found
                    (some? (:candidate
                            (first (filter #(= :branch-dependent-stride (:id %))
                                           rows))))
                    :sentinel-has-no-ranking
                    (nil? (:candidate
                           (first (filter #(= :non-decreasing-sentinel (:id %))
                                          rows))))
                    :no-accepted-compiler-admission
                    (empty? accepted)
                    :no-rejected-rows
                    (empty? rejected)
                    :all-proof-inputs-held
                    (= (count rows) (count held)))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.linear-ranking.receipt.v1"
     :stage [:M9b]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "bounded affine linear ranking-function synthesis witness"
     :domain {:template "c + Σ ai*xi"
              :coefficient-search [-2 -1 0 1 2]
              :constant-search [-20 20]
              :validation "non-negative on guard and strictly decreasing on all bounded transitions"}
     :policy {:proved-ranking "held proof input for later M10/VC discharge"
              :no-ranking "held, never raw lowering"
              :compiler-admission "not performed in this witness"}
     :rows rows
     :status-counts {:accepted (count accepted)
                     :held (count held)
                     :rejected (count rejected)}
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
    (doseq [row (:rows r)]
      (println (str "  [" (if (:ok row) "OK" "FAIL") "] "
                    (name (:id row))
                    " -> "
                    (pr-str (:candidate row)))))
    (println (str "linear ranking: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
