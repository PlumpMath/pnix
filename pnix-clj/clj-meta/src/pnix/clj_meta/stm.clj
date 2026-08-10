(ns pnix.clj-meta.stm)

(defn new-store
  []
  (ref {}))

(defn put!
  [store session-id key value]
  (dosync
    (alter store assoc-in [session-id key] value)
    {:op :put
     :session session-id
     :key key
     :value value}))

(defn append!
  [store session-id key value]
  (dosync
    (alter store update-in [session-id key] (fnil conj []) value)
    {:op :append
     :session session-id
     :key key
     :value value}))

(defn snapshot
  [store]
  @store)

(defn demo
  []
  (let [store (new-store)
        txs [(put! store :session-a :state :initialized)
             (append! store :session-a :receipt-candidates
                      {:kind :invariant-check
                       :input [2 3]
                       :status :pending})
             (put! store :session-b :state :initialized)]]
    {:schema "pnix.clj-meta.stm-smoke.v1"
     :stm-ref (instance? clojure.lang.Ref store)
     :transactions (count txs)
     :sessions (count (snapshot store))
     :snapshot (snapshot store)
     :ready (and (instance? clojure.lang.Ref store)
                 (= 3 (count txs))
                 (= #{:session-a :session-b} (set (keys (snapshot store)))))}))

(defn -main
  [& _]
  (let [result (demo)]
    (prn result)
    (shutdown-agents)
    (when-not (:ready result)
      (System/exit 1))))
