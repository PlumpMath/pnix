(ns pnix.clj-meta.stagen
  "StageN recursive closure ladder proof.

  StageN is a registry law: whenever a new host/runtime/proof surface appears,
  it must replay the stage8..15 closure sequence. Unknown or drifting surfaces
  are held, never accepted."
  (:require [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def ^:private closure-stages
  [:stage8 :stage9 :stage10 :stage11 :stage12 :stage13 :stage14 :stage15])

(def ^:private registry
  [{:surface/id :clj-meta/current
    :surface/type :current-runtime
    :surface/status :present
    :applies closure-stages}
   {:surface/id :future-host/runtime
    :surface/type :host-runtime
    :surface/status :not-bound
    :held-reason :missing-runtime-transcript
    :applies closure-stages}
   {:surface/id :future-proof/checker
    :surface/type :proof-checker
    :surface/status :not-bound
    :held-reason :missing-proof-adapter
    :applies closure-stages}
   {:surface/id :future-product/entrypoint
    :surface/type :product-entrypoint
    :surface/status :not-bound
    :held-reason :pnix-launcher-parked
    :applies closure-stages}
   {:surface/id :synthetic-stageN-drift
    :surface/type :drift-sentinel
    :surface/status :simulated-drift
    :held-reason :recursive-closure-drift
    :applies closure-stages}])

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- stage-ok?
  [context stage]
  (case stage
    :stage8  (and (true? (get-in context [:stage8 :ok]))
                  (true? (get-in context [:stage8 :fresh-stage8-exact?])))
    :stage9  (true? (get-in context [:stage9 :ok]))
    :stage10 (true? (get-in context [:stage10 :ok]))
    :stage11 (true? (get-in context [:stage11 :ok]))
    :stage12 (true? (get-in context [:stage12 :ok]))
    :stage13 (true? (get-in context [:stage13 :ok]))
    :stage14 (true? (get-in context [:stage14 :ok]))
    :stage15 (true? (get-in context [:stage15 :ok]))))

(defn- stage-digest
  [context stage]
  (or (get-in context [stage :canonical-receipt-digest])
      (sha256-string (select-keys (get context stage)
                                  [:ok
                                   :fresh-stage8-exact?
                                   :canonical-fixed?
                                   :artifact-fixed?
                                   :compare-ok?]))))

(defn- closure-ledger
  [context stages]
  (mapv (fn [stage]
          {:stage stage
           :ok (boolean (stage-ok? context stage))
           :digest (stage-digest context stage)})
        stages))

(defn- registry-row
  [context surface]
  (let [ledger      (closure-ledger context (:applies surface))
        raw-closed? (every? :ok ledger)
        closed?     (case (:surface/status surface)
                      :present raw-closed?
                      :not-bound false
                      :simulated-drift false)
        verdict (case (:surface/status surface)
                  :present (if closed? :accepted :held)
                  :not-bound :held
                  :simulated-drift :held)]
    (merge surface
           {:closure/ledger ledger
            :closure/closed? closed?
            :gate/verdict verdict
            :ok (case (:surface/status surface)
                  :present (= :accepted verdict)
                  :not-bound (= :held verdict)
                  :simulated-drift (= :held verdict))})))

(defn- canonical-row
  [row]
  (select-keys row
               [:surface/id
                :surface/type
                :surface/status
                :applies
                :closure/ledger
                :closure/closed?
                :gate/verdict
                :held-reason
                :ok]))

(defn run
  [context]
  (let [rows      (mapv #(registry-row context %) registry)
        current   (first (filter #(= :clj-meta/current (:surface/id %)) rows))
        not-bound (filter #(= :not-bound (:surface/status %)) rows)
        drift     (first (filter #(= :synthetic-stageN-drift (:surface/id %)) rows))
        canonical (mapv canonical-row rows)
        invariant (sorted-map
                   :all-rows-ok
                   (every? :ok rows)
                   :current-surface-accepted
                   (= :accepted (:gate/verdict current))
                   :drift-held
                   (= :held (:gate/verdict drift))
                   :no-unsupported-accepted
                   (not-any? #(and (not= :present (:surface/status %))
                                   (= :accepted (:gate/verdict %)))
                             rows)
                   :required-stage-law-complete
                   (every? #(= closure-stages (:applies %)) rows)
                   :unsupported-surfaces-held
                   (every? #(= :held (:gate/verdict %)) not-bound))]
    {:schema "pnix.clj-meta.stageN.recursive-closure.receipt.v1"
     :stage :N
     :desc "stageN recursive closure ladder"
     :closure-stages closure-stages
     :registry registry
     :rows rows
     :status-counts (frequencies (map :gate/verdict rows))
     :invariants invariant
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok (and (every? :ok rows)
              (every? true? (vals invariant)))}))

(defn -main
  [& _]
  (let [ok-stage {:ok true :canonical-receipt-digest "standalone"}
        r (run {:stage8 (assoc ok-stage :fresh-stage8-exact? true)
                :stage9 ok-stage
                :stage10 ok-stage
                :stage11 ok-stage
                :stage12 ok-stage
                :stage13 ok-stage
                :stage14 ok-stage
                :stage15 ok-stage})]
    (pp/pprint r)
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
