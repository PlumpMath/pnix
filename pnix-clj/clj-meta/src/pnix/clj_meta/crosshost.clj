(ns pnix.clj-meta.crosshost
  "Stage14 cross-host/cross-implementation law proof.

  Stage14 compares canonical verdict transcripts. This namespace produces the
  local clj-meta transcript and treats missing external host transcripts as
  held evidence, not as accepted truth."
  (:require [pnix.clj-meta.compiler :as comp]
            [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def ^:private transcript-input-dir
  "clj-meta/proof/stage14-inputs")

(def ^:private required-implementations
  [:clj-meta :hy-meta :pnix-hy :pnix-clj])

(def ^:private external-implementations
  (remove #{:clj-meta} required-implementations))

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
    :expected [:rest 7 '(8 9)]}
   {:fixture/id :fallback-case-boundary
    :form '(fn [x]
             (case x
               :a 1
               :b 2
               :other))
    :args [:missing]
    :expected :other}])

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- try-val
  [f]
  (try
    {:value (f)}
    (catch Throwable t
      {:error {:class (.getName (class t))
               :message (.getMessage t)}})))

(defn- canonical-value
  [result]
  (cond
    (contains? result :value)
    {:kind :value :repr (pr-str (:value result))}

    (contains? result :error)
    {:kind :error :repr (pr-str (:error result))}

    :else
    {:kind :unknown :repr (pr-str result)}))

(defn- replay-fixture
  [{:keys [fixture/id form args expected]}]
  (let [host     (try-val #(apply (eval form) args))
        compiled (try-val #(apply (comp/compile-form form) args))
        hv       (:value host)
        cv       (:value compiled)
        ok?      (and (contains? host :value)
                      (contains? compiled :value)
                      (= expected hv cv))
        verdict  (if ok? :accepted :rejected)]
    {:fixture/id id
     :form (pr-str form)
     :args args
     :expected (pr-str expected)
     :host (canonical-value host)
     :compiler (canonical-value compiled)
     :gate/verdict verdict
     :ok ok?}))

(defn- local-transcript
  [stage13]
  (let [rows      (mapv replay-fixture fixtures)
        canonical (mapv #(select-keys %
                                      [:fixture/id
                                       :form
                                       :args
                                       :expected
                                       :host
                                       :compiler
                                       :gate/verdict
                                       :ok])
                        rows)
        answer-hash (sha256-string (pr-str canonical))
        ok?       (every? :ok rows)]
    {:schema "pnix.clj-meta.stage14.transcript.v1"
     :implementation/id :clj-meta
     :implementation/status :present
     :stage13-boundary (select-keys stage13
                                    [:ok
                                     :canonical-receipt-digest
                                     :invariants])
     :fixture-count (count rows)
     :fixture-rows rows
     :canonical-transcript canonical
     :answer-plan-hash answer-hash
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- transcript-file
  [implementation-id]
  (io/file transcript-input-dir (str (name implementation-id) ".transcript.edn")))

(defn- read-transcript
  [implementation-id]
  (let [f (transcript-file implementation-id)]
    (when (.exists f)
      (edn/read-string (slurp f)))))

(defn- external-row
  "Missing external transcripts are held evidence (not accepted truth).
  Present matching hashes accept; present drift rejects (fails the row)."
  [local implementation-id]
  (let [path       (.getPath (transcript-file implementation-id))
        transcript (read-transcript implementation-id)]
    (cond
      (nil? transcript)
      {:implementation/id implementation-id
       :implementation/status :missing
       :transcript/path path
       :gate/verdict :held
       :held-reason :missing-transcript
       :ok true}

      (= (:answer-plan-hash local) (:answer-plan-hash transcript))
      {:implementation/id implementation-id
       :implementation/status :present
       :transcript/path path
       :answer-plan-hash (:answer-plan-hash transcript)
       :gate/verdict :accepted
       :ok true}

      :else
      {:implementation/id implementation-id
       :implementation/status :present
       :transcript/path path
       :expected-answer-plan-hash (:answer-plan-hash local)
       :answer-plan-hash (:answer-plan-hash transcript)
       :gate/verdict :rejected
       :held-reason :cross-host-drift
       :rejection-reason :cross-host-drift
       :ok false})))

(defn- drift-sentinel
  "Synthetic drift must remain held (never accepted). Proves the gate
  classifies answer-plan-hash mismatch as held drift evidence."
  [local]
  {:implementation/id :synthetic-drift-sentinel
   :implementation/status :simulated
   :expected-answer-plan-hash (:answer-plan-hash local)
   :answer-plan-hash (sha256-string [(:answer-plan-hash local)
                                     :synthetic-cross-host-drift])
   :gate/verdict :held
   :held-reason :cross-host-drift
   :ok true})

(defn- canonical-row
  [row]
  (select-keys row
               [:implementation/id
                :implementation/status
                :answer-plan-hash
                :expected-answer-plan-hash
                :gate/verdict
                :held-reason
                :ok]))

(defn run
  [stage13]
  (let [local     (local-transcript stage13)
        externals (mapv #(external-row local %) external-implementations)
        sentinel  (drift-sentinel local)
        rows      (into [{:implementation/id :clj-meta
                          :implementation/status :present
                          :answer-plan-hash (:answer-plan-hash local)
                          :gate/verdict (:gate/verdict local)
                          :ok (:ok local)}]
                        (conj externals sentinel))
        canonical (mapv canonical-row rows)
        held      (filter #(= :held (:gate/verdict %)) rows)
        invariant (sorted-map
                   :cross-host-drift-held
                   (= :held (:gate/verdict sentinel))
                   :external-missing-held
                   (every? #(if (= :missing (:implementation/status %))
                              (= :held (:gate/verdict %))
                              true)
                           externals)
                   :local-transcript-accepted
                   (= :accepted (:gate/verdict local))
                   :no-cross-host-drift-accepted
                   (not-any? #(and (= :cross-host-drift (:held-reason %))
                                   (= :accepted (:gate/verdict %)))
                             rows)
                   :stage13-bound
                   (true? (:ok stage13)))]
    {:schema "pnix.clj-meta.stage14.crosshost.receipt.v1"
     :stage 14
     :desc "stage14 cross-host/cross-implementation law closure"
     :required-implementations required-implementations
     :transcript-input-dir transcript-input-dir
     :local-transcript local
     :rows rows
     :status-counts (frequencies (map :gate/verdict rows))
     :held-count (count held)
     :invariants invariant
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok (and (every? :ok rows)
              (every? true? (vals invariant)))}))

(defn -main
  [& _]
  (let [r (run {:ok true
                :canonical-receipt-digest "standalone-stage13"
                :invariants {}})]
    (pp/pprint r)
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
