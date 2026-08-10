(ns pnix.clj-meta.quarantine
  "Stage12 self-improvement quarantine proof.

  This namespace does not apply compiler improvements. It proves the opposite
  boundary: candidate rule/emit/fallback updates remain evidence-only until a
  separate admission step promotes them."
  (:require [pnix.clj-meta.compiler :as comp]
            [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def ^:private tracked-live-vars
  (sorted-map
   :compile-classes #'comp/compile-classes
   :compile-form #'comp/compile-form
   :compile-ns #'comp/compile-ns
   :eval-form #'comp/eval-form
   :load-compiled-ns #'comp/load-compiled-ns))

(def ^:private candidates
  [{:candidate/id :emit-case-direct
    :source/frontier :host-maintained-op
    :proposal/type :emit-rule
    :proposal/target-op :case
    :proposal/status :quarantined
    :trust/status :evidence-only
    :promotion/allowed? false
    :admission/status :not-requested
    :fixtures [{:fixture/id :case-semantics
                :form '(fn [n] (case n 1 :one 2 :two :other))
                :args [2]
                :expected :two}]}
   {:candidate/id :emit-letfn-direct
    :source/frontier :host-maintained-op
    :proposal/type :emit-rule
    :proposal/target-op :letfn
    :proposal/status :quarantined
    :trust/status :evidence-only
    :promotion/allowed? false
    :admission/status :not-requested
    :fixtures [{:fixture/id :letfn-mutual-recursion
                :form '(fn [n]
                         (letfn [(ev? [x]
                                   (if (zero? x) true (od? (dec x))))
                                 (od? [x]
                                   (if (zero? x) false (ev? (dec x))))]
                           (ev? n)))
                :args [4]
                :expected true}]}
   {:candidate/id :typed-locals-primitive
    :source/frontier :performance-frontier
    :proposal/type :typed-local-rule
    :proposal/target-op :loop-recur
    :proposal/status :quarantined
    :trust/status :evidence-only
    :promotion/allowed? false
    :admission/status :not-requested
    :fixtures [{:fixture/id :loop-recur-sum
                :form '(fn [n]
                         (loop [i n acc 0]
                           (if (< i 1)
                             acc
                             (recur (- i 1) (+ acc i)))))
                :args [5]
                :expected 15}]}
   {:candidate/id :fallback-policy-hardening
    :source/frontier :fallback-boundary
    :proposal/type :policy-update
    :proposal/target-op :fallback-admission
    :proposal/status :quarantined
    :trust/status :evidence-only
    :promotion/allowed? false
    :admission/status :not-requested
    :fixtures [{:fixture/id :fallback-boundary-still-semantic
                :form '(fn [x]
                         (case x
                           :a 1
                           :b 2
                           :other))
                :args [:missing]
                :expected :other}]}])

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- live-root-snapshot
  []
  (into (sorted-map)
        (map (fn [[k v]]
               [k {:var (str v)
                   :root-identity (System/identityHashCode (var-get v))}])
             tracked-live-vars)))

(defn- try-val
  [f]
  (try
    {:value (f)}
    (catch Throwable t
      {:error {:class (.getName (class t))
               :message (.getMessage t)}})))

(defn- replay-fixture
  [{:keys [form args expected] :as fixture}]
  (let [result (try-val #(apply (comp/compile-form form) args))
        got    (:value result)
        ok?    (and (contains? result :value)
                    (= expected got))]
    (cond-> {:fixture/id (:fixture/id fixture)
             :form (pr-str form)
             :args args
             :expected expected
             :ok ok?}
      (contains? result :value) (assoc :got got)
      (:error result)           (assoc :error (:error result)))))

(defn- replay-candidate
  [candidate]
  (let [fixtures       (mapv replay-fixture (:fixtures candidate))
        replay-passed? (every? :ok fixtures)
        quarantine-ok? (and (= :quarantined (:proposal/status candidate))
                            (= :evidence-only (:trust/status candidate))
                            (false? (:promotion/allowed? candidate))
                            (= :not-requested (:admission/status candidate)))
        verdict        (cond
                         (not quarantine-ok?) :rejected
                         replay-passed?       :held
                         :else                :rejected)]
    (-> candidate
        (dissoc :fixtures)
        (assoc :replay {:status (if replay-passed? :passed :failed)
                        :fixtures fixtures}
               :gate/verdict verdict
               :quarantine/boundary-ok? (and quarantine-ok?
                                             (not= :accepted verdict))))))

(defn- canonical-candidate
  [candidate]
  (sorted-map
   :admission/status (:admission/status candidate)
   :candidate/id (:candidate/id candidate)
   :fixtures (mapv #(select-keys % [:fixture/id :form :args :expected :got :ok :error])
                   (get-in candidate [:replay :fixtures]))
   :gate/verdict (:gate/verdict candidate)
   :promotion/allowed? (:promotion/allowed? candidate)
   :proposal/status (:proposal/status candidate)
   :proposal/target-op (:proposal/target-op candidate)
   :proposal/type (:proposal/type candidate)
   :quarantine/boundary-ok? (:quarantine/boundary-ok? candidate)
   :replay/status (get-in candidate [:replay :status])
   :source/frontier (:source/frontier candidate)
   :trust/status (:trust/status candidate)))

(defn run
  [stage11]
  (let [before    (live-root-snapshot)
        rows      (mapv replay-candidate candidates)
        after     (live-root-snapshot)
        canonical (mapv canonical-candidate rows)
        invariant (sorted-map
                   :admission-required
                   (every? #(= :not-requested (:admission/status %)) rows)
                   :evidence-only
                   (every? #(= :evidence-only (:trust/status %)) rows)
                   :live-truth-unchanged
                   (= before after)
                   :no-accepted-verdict
                   (not-any? #(= :accepted (:gate/verdict %)) rows)
                   :no-auto-promotion
                   (every? #(false? (:promotion/allowed? %)) rows)
                   :quarantine-boundary
                   (every? :quarantine/boundary-ok? rows)
                   :replay-passed
                   (every? #(= :passed (get-in % [:replay :status])) rows)
                   :stage11-bound
                   (true? (:ok stage11)))
        ok?       (every? true? (vals invariant))]
    {:schema "pnix.clj-meta.stage12.quarantine.receipt.v1"
     :stage 12
     :desc "stage12 self-improvement quarantine closure"
     :stage11-boundary (select-keys stage11
                                    [:ok
                                     :canonical-receipt-digest
                                     :status-counts])
     :live-truth-before before
     :live-truth-after after
     :invariants invariant
     :candidates rows
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok ok?}))

(defn -main
  [& _]
  (let [r (run {:ok true
                :canonical-receipt-digest "standalone"
                :status-counts {}})]
    (pp/pprint r)
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
