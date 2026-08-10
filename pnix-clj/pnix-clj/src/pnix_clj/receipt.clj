(ns pnix-clj.receipt)

(def lane-classification
  {:lane :core
   :scope :multi-lane-receipt-verdict
   :role :summarize-cross-lane-frontier-and-verdict
   :product-runtime :allowed
   :semantic-authority :verdict-from-evidence-only
   :mutation :forbidden
   :admission :all-lanes-agree
   :determinism :required-upstream
   :allowed-output :receipt-summary-or-verdict})

(def lane-order
  [:pnix-clj-evaluator
   :pnix-clj-lowering-clj-meta
   :clojure-stage15-mirror
   :px-runtime-pnix-mirror])

(defn- ok-value
  [result]
  (when (= :ok (:status result))
    (:value result)))

(defn- result-row
  ([lane result frontier]
   (result-row lane result frontier nil))
  ([lane result frontier extra]
   (merge {:lane lane
           :status (or (:status result) :not-run)
           :reason (:reason result)
           :error (:error result)
           :frontier frontier}
          extra)))

(defn- frontier-row
  [lane status reason error frontier]
  {:lane lane
   :status status
   :reason reason
   :error error
   :frontier frontier})

(defn- propagate-verdict
  [result]
  (select-keys result
               [:status :reason :error :resource-reason :checkpoint]))

(defn lane-summary
  [{:keys [parse-result eval-result lowering-result clj-meta-result
           clojure-mirror stage15-control px-runtime pnix-mirror]}]
  (if (not= :ok (:status parse-result))
    (mapv #(frontier-row %
                         (:status parse-result)
                         (:reason parse-result)
                         (:error parse-result)
                         :parser)
          lane-order)
    [(result-row :pnix-clj-evaluator eval-result :evaluator)
     (cond
       (not= :ok (:status lowering-result))
       (frontier-row :pnix-clj-lowering-clj-meta
                     (:status lowering-result)
                     (:reason lowering-result)
                     (:error lowering-result)
                     :lowering)

       :else
       (result-row :pnix-clj-lowering-clj-meta
                   clj-meta-result
                   :clj-meta
                   {:lowering-status (:status lowering-result)}))
     (cond
       (not clojure-mirror)
       (frontier-row :clojure-stage15-mirror
                     :failed
                     :clojure-mirror-missing
                     {:phase :evidence :class :clojure-mirror-missing}
                     :clojure-mirror)

       (not= :ok (:clj-meta-status clojure-mirror))
       (frontier-row :clojure-stage15-mirror
                     (:status clj-meta-result)
                     (or (:reason clj-meta-result)
                         :clj-meta-result-not-ok)
                     (:error clj-meta-result)
                     :clojure-mirror)

       :else
       {:lane :clojure-stage15-mirror
        :status :ok
        :reason nil
        :frontier :clojure-mirror
        :stage15-control-status (:status stage15-control)
        :stage15-control-reason (:reason stage15-control)})
     (cond
       (not= :ok (:status px-runtime))
       (frontier-row :px-runtime-pnix-mirror
                     (:status px-runtime)
                     (:reason px-runtime)
                     (:error px-runtime)
                     :px-runtime)

       :else
       (result-row :px-runtime-pnix-mirror
                   pnix-mirror
                   :pnix-mirror
                   {:px-runtime-status (:status px-runtime)}))]))

(defn verdict
  [{:keys [parse-result eval-result lowering-result clj-meta-result
           oracle-result px-runtime pnix-mirror]}]
  (cond
    (not= :ok (:status parse-result))
    (propagate-verdict parse-result)

    (not= :ok (:status eval-result))
    (propagate-verdict eval-result)

    (and oracle-result
         (= :ok (:status oracle-result))
         (not= (ok-value eval-result) (ok-value oracle-result)))
    {:status :rejected
     :reason :oracle-mismatch}

    (and oracle-result
         (not= :ok (:status oracle-result)))
    (propagate-verdict oracle-result)

    (not= :ok (:status lowering-result))
    (propagate-verdict lowering-result)

    (not= :ok (:status clj-meta-result))
    (propagate-verdict clj-meta-result)

    (not= (ok-value eval-result) (ok-value clj-meta-result))
    {:status :rejected
     :reason :evaluator-clj-meta-mismatch}

    (not= :ok (:status px-runtime))
    (propagate-verdict px-runtime)

    (not= (ok-value eval-result) (:value px-runtime))
    {:status :rejected
     :reason :px-runtime-value-mismatch}

    (not= :ok (:status pnix-mirror))
    (propagate-verdict pnix-mirror)

    (not= (ok-value eval-result) (:value pnix-mirror))
    {:status :rejected
     :reason :pnix-mirror-value-mismatch}

    :else
    {:status :accepted
     :reason :all-lanes-agree}))

(defn- reason-counts
  [receipts status]
  (->> receipts
       (filter #(= status (:status %)))
       (keep :reason)
       frequencies
       (into {})))

(defn- receipt-frontier
  [receipt]
  (when (not= :accepted (:status receipt))
    (let [lane (first (filter #(not= :ok (:status %))
                              (:lane-summary receipt)))]
      (cond-> {:source-id (:source-id receipt)
               :status (:status receipt)
               :reason (:reason receipt)}
        lane
        (assoc :lane (:lane lane)
               :lane-status (:status lane)
               :lane-reason (:reason lane)
               :lane-frontier (:frontier lane))))))

(defn summarize
  [receipts]
  (let [counts (frequencies (map :status receipts))]
    {:total (count receipts)
     :accepted (long (get counts :accepted 0))
     :rejected (long (get counts :rejected 0))
     :held (long (get counts :held 0))
     :held-reason-counts (reason-counts receipts :held)
     :rejected-reason-counts (reason-counts receipts :rejected)
     :reason-counts (->> receipts
                         (keep :reason)
                         frequencies
                         (into {}))
     :first-rejected (first (filter #(= :rejected (:status %)) receipts))
     :first-held (first (filter #(= :held (:status %)) receipts))
     :first-frontier (first (keep receipt-frontier receipts))}))
