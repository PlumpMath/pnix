(ns pnix.clj-meta.longhorizon
  "Stage13 long-horizon compiler organism proof.

  This is a bounded replay model, not a wall-clock soak test. It replays the
  same compiler fixtures across logical day/session/snapshot labels and proves
  that stale source/artifact/stage proof drift is held instead of accepted."
  (:require [pnix.clj-meta.compiler :as comp]
            [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def ^:private fixtures
  [{:fixture/id :square
    :form '(fn [n] (* n n))
    :args [9]
    :expected 81}
   {:fixture/id :loop-sum
    :form '(fn [n]
             (loop [i n acc 0]
               (if (< i 1)
                 acc
                 (recur (- i 1) (+ acc i)))))
    :args [5]
    :expected 15}
   {:fixture/id :mixed-variadic
    :form '(fn ([x] [:fixed x])
             ([x & r] [:rest x r]))
    :args [7 8 9]
    :expected [:rest 7 '(8 9)]}])

(def ^:private scenarios
  [{:snapshot/id :day001-session-a
    :day-label "day-001"
    :session/id :session-a
    :scenario/type :clean-replay}
   {:snapshot/id :day007-session-b
    :day-label "day-007"
    :session/id :session-b
    :scenario/type :clean-replay}
   {:snapshot/id :day030-session-c
    :day-label "day-030"
    :session/id :session-c
    :scenario/type :clean-replay}
   {:snapshot/id :day030-source-update-stale-artifact
    :day-label "day-030"
    :session/id :session-c
    :scenario/type :source-drift}
   {:snapshot/id :day030-artifact-drift
    :day-label "day-030"
    :session/id :session-c
    :scenario/type :artifact-drift}
   {:snapshot/id :day030-stage11-drift
    :day-label "day-030"
    :session/id :session-c
    :scenario/type :stage11-drift}
   {:snapshot/id :day030-stage12-drift
    :day-label "day-030"
    :session/id :session-c
    :scenario/type :stage12-drift}])

(defn- sha256-bytes
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md bs)))))

(defn- sha256-string
  [s]
  (sha256-bytes (.getBytes ^String (str s) "UTF-8")))

(defn- class-map-digest
  [classes]
  (sha256-string
   (pr-str
    (mapv (fn [[class-name class-bytes]]
            [class-name (sha256-bytes class-bytes)])
          (sort-by key classes)))))

(defn- try-val
  [f]
  (try
    {:value (f)}
    (catch Throwable t
      {:error {:class (.getName (class t))
               :message (.getMessage t)}})))

(defn- replay-fixture
  [{:keys [fixture/id form args expected] :as fixture}]
  (let [run-result (try-val #(apply (comp/compile-form form) args))
        classes    (comp/compile-classes form)
        got        (:value run-result)
        result-ok? (and (contains? run-result :value)
                        (= expected got))]
    (cond-> {:fixture/id id
             :form (pr-str form)
             :args args
             :expected expected
             :artifact-digest (class-map-digest classes)
             :class-count (count classes)
             :result-ok? result-ok?}
      (contains? run-result :value) (assoc :got got)
      (:error run-result)           (assoc :error (:error run-result)))))

(defn- replay-fixtures
  []
  (mapv replay-fixture fixtures))

(defn- source-digest
  []
  (sha256-string (pr-str (mapv #(select-keys % [:fixture/id :form :args :expected])
                               fixtures))))

(defn- fixture-artifact-digest
  [replayed]
  (sha256-string
   (pr-str (mapv #(select-keys % [:fixture/id :artifact-digest :class-count])
                 replayed))))

(defn- fixture-verdict-digest
  [replayed]
  (sha256-string
   (pr-str (mapv #(select-keys % [:fixture/id :expected :got :result-ok?])
                 replayed))))

(defn- stage-digest
  [stage fallback]
  (or (:canonical-receipt-digest stage) fallback))

(defn- baseline-manifest
  [stage11 stage12 replayed]
  (sorted-map
   :artifact-digest (fixture-artifact-digest replayed)
   :source-digest (source-digest)
   :stage11-digest (stage-digest stage11 "standalone-stage11")
   :stage12-digest (stage-digest stage12 "standalone-stage12")
   :verdict-digest (fixture-verdict-digest replayed)))

(defn- observed-manifest
  [baseline scenario]
  (case (:scenario/type scenario)
    :clean-replay baseline
    :source-drift (assoc baseline :source-digest
                         (sha256-string [(:source-digest baseline)
                                         :simulated-source-update]))
    :artifact-drift (assoc baseline :artifact-digest
                           (sha256-string [(:artifact-digest baseline)
                                           :simulated-artifact-update]))
    :stage11-drift (assoc baseline :stage11-digest
                          (sha256-string [(:stage11-digest baseline)
                                          :simulated-stage11-update]))
    :stage12-drift (assoc baseline :stage12-digest
                          (sha256-string [(:stage12-digest baseline)
                                          :simulated-stage12-update]))))

(defn- drift-reasons
  [baseline observed]
  (cond-> []
    (not= (:source-digest baseline) (:source-digest observed))
    (conj :source-digest-drift)
    (not= (:artifact-digest baseline) (:artifact-digest observed))
    (conj :artifact-digest-drift)
    (not= (:stage11-digest baseline) (:stage11-digest observed))
    (conj :stage11-digest-drift)
    (not= (:stage12-digest baseline) (:stage12-digest observed))
    (conj :stage12-digest-drift)
    (not= (:verdict-digest baseline) (:verdict-digest observed))
    (conj :verdict-digest-drift)))

(defn- scenario-row
  [baseline replayed scenario]
  (let [observed      (observed-manifest baseline scenario)
        drift         (drift-reasons baseline observed)
        replay-passed (every? :result-ok? replayed)
        verdict       (cond
                        (not replay-passed) :rejected
                        (seq drift)         :held
                        :else               :accepted)
        expected      (if (= :clean-replay (:scenario/type scenario))
                        :accepted
                        :held)]
    {:snapshot/id (:snapshot/id scenario)
     :day-label (:day-label scenario)
     :session/id (:session/id scenario)
     :scenario/type (:scenario/type scenario)
     :replay/status (if replay-passed :passed :failed)
     :manifest/status (if (seq drift) :stale :current)
     :drift-reasons drift
     :gate/verdict verdict
     :expected/verdict expected
     :ok (= expected verdict)
     :observed-manifest observed}))

(defn- canonical-row
  [row]
  (sorted-map
   :day-label (:day-label row)
   :drift-reasons (:drift-reasons row)
   :expected/verdict (:expected/verdict row)
   :gate/verdict (:gate/verdict row)
   :manifest/status (:manifest/status row)
   :ok (:ok row)
   :replay/status (:replay/status row)
   :scenario/type (:scenario/type row)
   :session/id (:session/id row)
   :snapshot/id (:snapshot/id row)))

(defn run
  [stage11 stage12]
  (let [replayed  (replay-fixtures)
        baseline  (baseline-manifest stage11 stage12 replayed)
        rows      (mapv #(scenario-row baseline replayed %) scenarios)
        canonical (mapv canonical-row rows)
        clean     (filter #(= :clean-replay (:scenario/type %)) rows)
        drift     (remove #(= :clean-replay (:scenario/type %)) rows)
        invariant (sorted-map
                   :clean-replays-accepted
                   (every? #(= :accepted (:gate/verdict %)) clean)
                   :drift-held
                   (every? #(= :held (:gate/verdict %)) drift)
                   :fixtures-replay-passed
                   (every? :result-ok? replayed)
                   :no-drift-accepted
                   (not-any? #(and (seq (:drift-reasons %))
                                   (= :accepted (:gate/verdict %)))
                             rows)
                   :session-labels-isolated
                   (= (count clean) (count (set (map :session/id clean))))
                   :stage11-bound
                   (true? (:ok stage11))
                   :stage12-bound
                   (true? (:ok stage12)))]
    {:schema "pnix.clj-meta.stage13.long-horizon.receipt.v1"
     :stage 13
     :desc "stage13 long-horizon compiler organism closure"
     :stage11-boundary (select-keys stage11
                                    [:ok
                                     :canonical-receipt-digest
                                     :status-counts])
     :stage12-boundary (select-keys stage12
                                    [:ok
                                     :canonical-receipt-digest
                                     :invariants])
     :baseline-manifest baseline
     :fixture-replay replayed
     :rows rows
     :invariants invariant
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok (and (every? :ok rows)
              (every? true? (vals invariant)))}))

(defn -main
  [& _]
  (let [r (run {:ok true
                :canonical-receipt-digest "standalone-stage11"
                :status-counts {}}
               {:ok true
                :canonical-receipt-digest "standalone-stage12"
                :invariants {}})]
    (pp/pprint r)
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
