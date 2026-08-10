(ns pnix-clj.store
  "§5 — append-only EVENT log (spine BUILD 2nd; see docs/SPINE_ROADMAP.md).

  A VERIFYING TRACE (Build Systems à la Carte, ICFP'18): the log stores small
  hashes + pure-EDN payloads, NOT the values themselves -- values live in the
  §3 term store / cached-eval keyed by the SAME hashes (roles kept distinct).

  The log is append-only and TAMPER-EVIDENT: each event carries an event-hash
  chained on the previous event's hash (a Merkle/hash chain), so `verify-chain`
  re-derives every hash and detects any retroactive edit. No update/delete API;
  pointer movement is itself an event (`set-pointer!` → :pointer/moved).

  HERMETICITY discipline (Bazel's non-determinism sources): `append!` rejects
  contamination -- non-EDN payloads (fn/atom/host object, via pnix-clj.cas/
  pure-term?) and identity-bearing/time-varying values (java.util.Date, host
  instances). Only pure, replayable data enters the durable log."
  (:require [pnix-clj.cas :as cas]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :append-only-event-store
   :role :tamper-evident-hermetic-event-log
   :product-runtime :allowed
   :semantic-authority :evidence-storage-only
   :mutation :append-only
   :admission :forbidden
   :determinism :hash-chain-required
   :allowed-output :event-log-or-chain-verification-report})

(def ^:private genesis "genesis")

(defn open-store
  "A fresh append-only event store (an atom holding immutable EDN events)."
  []
  (atom {:events [] :head-hash genesis}))

(defn- contaminated?
  "Hermeticity guard: reject anything that would make the log non-replayable --
  non-EDN data, or identity/time-bearing host objects (Date, arbitrary Java)."
  [payload]
  (letfn [(bad? [x]
            (cond
              (instance? java.util.Date x) true
              (or (nil? x) (boolean? x) (number? x) (string? x)
                  (keyword? x) (symbol? x)) false
              (map? x) (or (some bad? (keys x)) (some bad? (vals x)))
              (coll? x) (boolean (some bad? x))
              :else true))]           ; anything else = non-EDN host object
    (or (not (cas/pure-term? payload)) (bad? payload))))

(defn- event-hash
  [prev-hash seq kind payload]
  (hash/data-hash [prev-hash seq kind payload]))

(defn append!
  "Append an event {:kind :payload}. Returns {:status :ok :seq :event-hash} or
  {:status :rejected :reason :contaminated-payload}. The event-hash chains on
  the store head (tamper-evident)."
  [store kind payload]
  (if (contaminated? payload)
    {:status :rejected :reason :contaminated-payload}
    (let [{:keys [events head-hash]} @store
          seq (count events)
          eh (event-hash head-hash seq kind payload)
          event {:seq seq :kind kind :payload payload
                 :prev-hash head-hash :event-hash eh}]
      (swap! store (fn [s] (-> s (update :events conj event) (assoc :head-hash eh))))
      {:status :ok :seq seq :event-hash eh})))

(defn events        [store] (:events @store))
(defn head-hash     [store] (:head-hash @store))
(defn events-of     [store kind] (filterv #(= kind (:kind %)) (events store)))
(defn by-hash       [store eh] (first (filter #(= eh (:event-hash %)) (events store))))
(defn by-field      [store k v] (filterv #(= v (get-in % [:payload k])) (events store)))

(defn set-pointer!
  "Move a named pointer to a target -- recorded as a :pointer/moved event."
  [store pointer-name target]
  (append! store :pointer/moved {:pointer pointer-name :target target}))

(defn get-pointer
  "The latest target of a named pointer (folded from the event log)."
  [store pointer-name]
  (->> (events-of store :pointer/moved)
       (filter #(= pointer-name (get-in % [:payload :pointer])))
       last
       (#(get-in % [:payload :target]))))

(defn verify-chain
  "Re-derive every event-hash from its predecessor; the log is intact iff every
  link matches (append-only tamper-evidence)."
  [store]
  (loop [prev genesis, remaining (events store)]
    (if-let [{:keys [seq kind payload event-hash prev-hash]} (first remaining)]
      (if (and (= prev-hash prev)
               (= event-hash (@#'event-hash prev seq kind payload)))
        (recur event-hash (rest remaining))
        {:status :broken :at seq})
      {:status :intact :length (count (events store)) :head (head-hash store)})))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [store (open-store)
        a (append! store :eval/run {:source-hash "aaa" :result-hash "111"})
        b (append! store :eval/run {:source-hash "bbb" :result-hash "222"})
        p (set-pointer! store :active "bbb")
        bad (append! store :eval/run {:when (java.util.Date.)})
        bad2 (append! store :eval/run {:f (fn [] 1)})
        chain (verify-chain store)
        rows [{:id :append-seq :ok? (and (= 0 (:seq a)) (= 1 (:seq b)))}
              {:id :hash-chained :ok? (not= (:event-hash a) (:event-hash b))}
              {:id :index-by-kind :ok? (= 2 (count (events-of store :eval/run)))}
              {:id :index-by-field :ok? (= 1 (count (by-field store :source-hash "aaa")))}
              {:id :pointer-as-event :ok? (= "bbb" (get-pointer store :active))}
              {:id :reject-date :ok? (= :rejected (:status bad))}
              {:id :reject-fn :ok? (= :rejected (:status bad2))}
              {:id :chain-intact :ok? (= :intact (:status chain))}
              {:id :append-only-no-mutation-api
               ;; contaminated appends did NOT grow the log
               :ok? (= 3 (count (events store)))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-store-report
              :schema :pnix-clj.store-report.v0
              :policy :append-only-verifying-trace-tamper-evident-hermetic
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :rows (mapv (fn [r] (assoc r :status (if (:ok? r) :accepted :rejected))) rows)}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash (:rows body)))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected]} (report)]
    (println (format "pnix-clj store: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
