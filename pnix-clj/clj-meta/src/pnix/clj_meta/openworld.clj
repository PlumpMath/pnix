(ns pnix.clj-meta.openworld
  "Stage15 open-world evidence federation proof.

  Stage15 admits external-looking evidence only as evidence/candidate material.
  Nothing from Lean/Z3/CAS/GitHub/LLM/document/sandbox style sources is promoted
  to accepted truth without a separate pnix gate/admission record."
  (:require [pnix.clj-meta.compiler :as comp]
            [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def ^:private source-types
  [:lean :z3 :cas :github :llm :document :sandbox])

(def ^:private evidence-fixtures
  [{:evidence/id :lean-proof-candidate
    :source/type :lean
    :source/id "lean://example/add-zero"
    :source/version "fixture"
    :adapter/id :lean-transcript-adapter
    :adapter/version "fixture-v1"
    :claim {:kind :theorem
            :statement "(= (+ n 0) n)"
            :language :lean}
    :artifact {:kind :proof-text
               :content "theorem add_zero_fixture : n + 0 = n := by simp"}
    :replay {:mode :not-available
             :reason :external-checker-not-bound}}
   {:evidence/id :z3-sat-candidate
    :source/type :z3
    :source/id "z3://example/linear-arithmetic"
    :source/version "fixture"
    :adapter/id :smt-result-adapter
    :adapter/version "fixture-v1"
    :claim {:kind :solver-result
            :statement "(assert (> (+ x 1) x))"
            :result :sat}
    :artifact {:kind :solver-output
               :content "sat"}
    :replay {:mode :not-available
             :reason :external-solver-not-bound}}
   {:evidence/id :cas-rewrite-candidate
    :source/type :cas
    :source/id "cas://example/simplify-square"
    :source/version "fixture"
    :adapter/id :cas-rewrite-adapter
    :adapter/version "fixture-v1"
    :claim {:kind :rewrite
            :from "(* (+ x 1) (+ x 1))"
            :to "(+ (* x x) (* 2 x) 1)"}
    :artifact {:kind :rewrite-log
               :content "expand square"}
    :replay {:mode :not-available
             :reason :external-cas-not-bound}}
   {:evidence/id :github-patch-candidate
    :source/type :github
    :source/id "github://example/repo/pull/1"
    :source/version "fixture"
    :adapter/id :repo-patch-adapter
    :adapter/version "fixture-v1"
    :claim {:kind :patch
            :statement "compiler fixture remains semantically stable"}
    :artifact {:kind :diff
               :content "(+ no-op patch fixture)"}
    :replay {:mode :not-available
             :reason :repo-snapshot-not-bound}}
   {:evidence/id :llm-suggestion-candidate
    :source/type :llm
    :source/id "llm://fixture/suggestion"
    :source/version "fixture"
    :adapter/id :llm-suggestion-adapter
    :adapter/version "fixture-v1"
    :claim {:kind :code-suggestion
            :statement "typed locals may improve loop/recur"}
    :artifact {:kind :natural-language
               :content "Use primitive local slots for loop counters."}
    :replay {:mode :not-available
             :reason :llm-output-is-not-proof}}
   {:evidence/id :document-claim-candidate
    :source/type :document
    :source/id "doc://fixture/compiler-note"
    :source/version "fixture"
    :adapter/id :document-claim-adapter
    :adapter/version "fixture-v1"
    :claim {:kind :document-claim
            :statement "Stage15 evidence remains evidence-only."}
    :artifact {:kind :document-excerpt
               :content "External evidence requires admission."}
    :replay {:mode :not-available
             :reason :document-is-provenance-only}}
   {:evidence/id :sandbox-compiler-replay
    :source/type :sandbox
    :source/id "sandbox://fixture/compiler-replay"
    :source/version "fixture"
    :adapter/id :sandbox-replay-adapter
    :adapter/version "fixture-v1"
    :claim {:kind :compiler-result
            :statement "compiled function returns expected value"}
    :artifact {:kind :clojure-form
               :form '(fn [n] (* n n))
               :args [9]
               :expected 81}
    :replay {:mode :local-compiler}}])

(def ^:private direct-acceptance-sentinel
  {:evidence/id :direct-acceptance-sentinel
   :source/type :llm
   :source/id "llm://fixture/bad-direct-acceptance"
   :source/version "fixture"
   :adapter/id :llm-suggestion-adapter
   :adapter/version "fixture-v1"
   :claim {:kind :bad-evidence
           :statement "This fixture tries to bypass admission."}
   :artifact {:kind :natural-language
              :content "Pretend this is already accepted."}
   :trust/status :accepted
   :promotion/allowed? true
   :admission/status :accepted
   :replay {:mode :not-available
            :reason :sentinel}})

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

(defn- replay-evidence
  [{:keys [artifact replay]}]
  (case (:mode replay)
    :local-compiler
    (let [{:keys [form args expected]} artifact
          result (try-val #(apply (comp/compile-form form) args))]
      (if (and (contains? result :value)
               (= expected (:value result)))
        {:status :passed
         :mode :local-compiler
         :expected expected
         :got (:value result)}
        {:status :failed
         :mode :local-compiler
         :expected expected
         :result result}))

    :not-available
    {:status :unavailable
     :mode :not-available
     :reason (:reason replay)}))

(defn- normalize-evidence
  [evidence]
  (let [claim-hash    (sha256-string (:claim evidence))
        artifact-hash (sha256-string (:artifact evidence))
        adapter-hash  (sha256-string (select-keys evidence
                                                  [:adapter/id
                                                   :adapter/version
                                                   :source/type]))
        replay-result (replay-evidence evidence)
        requested-accepted? (or (= :accepted (:trust/status evidence))
                                (true? (:promotion/allowed? evidence))
                                (= :accepted (:admission/status evidence)))
        base          (merge {:trust/status :evidence-only
                              :promotion/allowed? false
                              :admission/status :not-requested}
                             evidence)
        verdict       (cond
                        requested-accepted? :rejected
                        (= :failed (:status replay-result)) :rejected
                        (= :passed (:status replay-result)) :pending-admission
                        (= :unavailable (:status replay-result)) :unavailable
                        :else :rejected)
        reason        (cond
                        requested-accepted? [:rejection-reason :direct-acceptance-forbidden]
                        (= :failed (:status replay-result)) [:rejection-reason :external-replay-failed]
                        (= :passed (:status replay-result)) [:pending-reason :awaiting-admission]
                        (= :unavailable (:status replay-result)) [:unavailable-reason :external-replay-not-bound]
                        :else [:rejection-reason :invalid-replay-status])]
    (-> base
        (assoc :claim/hash claim-hash
               :artifact/hash artifact-hash
               :adapter/hash adapter-hash
               :replay/result replay-result
               :gate/verdict verdict
               :ok (if requested-accepted?
                     (= :rejected verdict)
                     (not= :accepted verdict)))
        (assoc (first reason) (second reason)))))

(defn- canonical-row
  [row]
  (select-keys row
               [:evidence/id
                :source/type
                :source/id
                :source/version
                :adapter/id
                :adapter/hash
                :claim/hash
                :artifact/hash
                :trust/status
                :promotion/allowed?
                :admission/status
                :replay/result
                :gate/verdict
                :pending-reason
                :unavailable-reason
                :rejection-reason
                :ok]))

(defn run
  [stage14]
  (let [rows      (mapv normalize-evidence
                        (conj evidence-fixtures direct-acceptance-sentinel))
        canonical (mapv canonical-row rows)
        sentinel  (first (filter #(= :direct-acceptance-sentinel (:evidence/id %))
                                 rows))
        normal    (remove #(= :direct-acceptance-sentinel (:evidence/id %)) rows)
        invariant (sorted-map
                   :all-source-types-covered
                   (= (set source-types) (set (map :source/type normal)))
                   :direct-acceptance-rejected
                   (= :rejected (:gate/verdict sentinel))
                   :evidence-only
                   (every? #(= :evidence-only (:trust/status %)) normal)
                   :external-replay-unavailable-or-passed
                   (every? #(contains? #{:unavailable :passed}
                                       (get-in % [:replay/result :status]))
                           normal)
                   :no-external-evidence-accepted
                   (not-any? #(= :accepted (:gate/verdict %)) rows)
                   :no-auto-promotion
                   (every? #(false? (:promotion/allowed? %)) normal)
                   :no-admission
                   (every? #(= :not-requested (:admission/status %)) normal)
                   :stage14-bound
                   (true? (:ok stage14)))]
    {:schema "pnix.clj-meta.stage15.openworld.receipt.v1"
     :stage 15
     :desc "stage15 open-world evidence federation closure"
     :stage14-boundary (select-keys stage14
                                    [:ok
                                     :canonical-receipt-digest
                                     :invariants])
     :source-types source-types
     :rows rows
     :status-counts (frequencies (map :gate/verdict rows))
     :invariants invariant
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok (and (every? :ok rows)
              (every? true? (vals invariant)))}))

(defn -main
  [& _]
  (let [r (run {:ok true
                :canonical-receipt-digest "standalone-stage14"
                :invariants {}})]
    (pp/pprint r)
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
