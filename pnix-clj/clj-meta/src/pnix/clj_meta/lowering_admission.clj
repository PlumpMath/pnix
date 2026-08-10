(ns pnix.clj-meta.lowering-admission
  "M10 lowering admission cross-witness.

  Translation validation proves overflow-freedom VCs; primitive bytecode witness
  proves the emitted artifact shape. This receipt ties the two together without
  expanding compiler authority: accepted VCs must have matching bytecode
  evidence, and rejected VCs must remain fallback/forbidden-opcode evidence."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.bytecode-witness :as bw]
            [pnix.clj-meta.m6aj-framework :as m6aj]
            [pnix.clj-meta.translation-validation :as tv])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/lowering-admission.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- index-by-id
  [rows]
  (into {} (map (juxt :id identity) rows)))

(defn- opcode-ok?
  [case op]
  (some #(and (= op (:op %)) (:ok %))
        (:required-opcodes case)))

(defn- forbidden-ok?
  [case op]
  (some #(and (= op (:op %)) (:ok %))
        (:forbidden-opcodes case)))

(defn- evidence-ok?
  [case evidence-id]
  (some #(and (= evidence-id (:id %)) (:ok %))
        (:required-evidence case)))

(defn- accepted-pairs
  []
  [{:id :literal-add-admission
    :tv-row :literal-add-vc
    :bytecode-case :checked-long-constant-no-overflow-direct
    :required-opcode :ladd
    :required-evidence :direct-checked-long-add-constant-proof}
   {:id :literal-sub-admission
    :tv-row :literal-sub-vc
    :bytecode-case :checked-long-constant-no-overflow-direct
    :required-opcode :lsub
    :required-evidence :direct-checked-long-subtract-constant-proof}
   {:id :literal-mul-admission
    :tv-row :literal-mul-vc
    :bytecode-case :checked-long-constant-no-overflow-direct
    :required-opcode :lmul
    :required-evidence :direct-checked-long-multiply-constant-proof}
   {:id :let-local-range-admission
    :tv-row :let-local-range-vc
    :bytecode-case :checked-long-let-local-range-direct
    :required-opcode :ladd
    :required-evidence :direct-checked-long-add-let-range-proof}
   {:id :guarded-argument-range-admission
    :tv-row :guarded-argument-range-vc
    :bytecode-case :checked-long-fn-argument-guard-range-direct
    :required-opcode :lmul
    :required-evidence :direct-checked-long-multiply-fn-argument-guard-proof}
   {:id :loop-exit-range-admission
    :tv-row :loop-exit-range-vc
    :bytecode-case :checked-long-loop-bounded-step-range-direct
    :required-opcode :ladd
    :required-evidence :direct-checked-long-add-loop-bounded-step-proof}])

(defn- rejected-pairs
  []
  [{:id :overflow-add-refusal
    :tv-row :overflow-add-rejected
    :bytecode-case :checked-long-overflow-fallback
    :forbidden-opcode :ladd
    :required-evidence :checked-long-overflow-add-fallback}
   {:id :opcode-mismatch-refusal
    :tv-row :opcode-mismatch-rejected
    :bytecode-case nil
    :forbidden-opcode nil
    :required-evidence nil}
   {:id :too-narrow-range-independent-domain-refusal
    :tv-row :too-narrow-range-rejected-by-independent-domain
    :bytecode-case nil
    :forbidden-opcode nil
    :required-evidence nil}
   {:id :too-narrow-range-independent-interval-refusal
    :tv-row :too-narrow-range-rejected-by-independent-interval
    :bytecode-case nil
    :forbidden-opcode nil
    :required-evidence nil}])

(defn- accepted-row
  [tv-rows bw-cases {:keys [id tv-row bytecode-case required-opcode required-evidence]}]
  (let [tv-row-data (get tv-rows tv-row)
        bw-case-data (get bw-cases bytecode-case)
        ok? (and (= :accepted (:gate/verdict tv-row-data))
                 (string? (get-in tv-row-data [:vc :digest]))
                 (:ok bw-case-data)
                 (opcode-ok? bw-case-data required-opcode)
                 (evidence-ok? bw-case-data required-evidence))]
    {:id id
     :kind :accepted-vc-bytecode-admission
     :translation-validation {:row tv-row
                              :verdict (:gate/verdict tv-row-data)
                              :vc-digest (get-in tv-row-data [:vc :digest])}
     :bytecode-witness {:case bytecode-case
                        :case-ok (:ok bw-case-data)
                        :required-opcode required-opcode
                        :required-evidence required-evidence}
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- rejected-row
  [tv-rows bw-cases {:keys [id tv-row bytecode-case forbidden-opcode required-evidence]}]
  (let [tv-row-data (get tv-rows tv-row)
        bw-case-data (when bytecode-case
                       (get bw-cases bytecode-case))
        bytecode-ok? (if bw-case-data
                       (and (:ok bw-case-data)
                            (forbidden-ok? bw-case-data forbidden-opcode)
                            (evidence-ok? bw-case-data required-evidence))
                       true)
        ok? (and (= :rejected (:gate/verdict tv-row-data))
                 (:fallback tv-row-data)
                 bytecode-ok?)]
    {:id id
     :kind :rejected-vc-fallback-refusal
     :translation-validation {:row tv-row
                              :verdict (:gate/verdict tv-row-data)
                              :vc-digest (get-in tv-row-data [:vc :digest])
                              :fallback (:fallback tv-row-data)}
     :bytecode-witness (when bw-case-data
                         {:case bytecode-case
                          :case-ok (:ok bw-case-data)
                          :forbidden-opcode forbidden-opcode
                          :required-evidence required-evidence})
     :gate/verdict (if ok? :held :rejected)
     :held-reason (when ok? :validator-refused-raw-lowering)
     :ok ok?}))

(defn- m6aj-row
  "M9b 흡수 후:
  - promotion-allowed accepted → engine admitted with bytecode evidence
  - checked-fallback accepted (promotion false) → boundary held: stays on
    Numbers.*; do NOT require raw-lowered bytecode
  - held framework rows → proof-input held without admission"
  [framework-row]
  (cond
    ;; Checked-fallback boundary (e.g. unbounded nonlinear recurrence):
    ;; m6aj marks gate/verdict :accepted with boundary/status :checked-fallback
    ;; and promotion/allowed? false. That is not raw-lowering admission.
    (and (= :accepted (:gate/verdict framework-row))
         (or (= :checked-fallback (:boundary/status framework-row))
             (= :checked-fallback-boundary (:kind framework-row))
             (false? (:promotion/allowed? framework-row))))
    (let [ok? (and (:ok framework-row)
                   (false? (:promotion/allowed? framework-row))
                   (not (get-in framework-row [:evidence :raw-lowered?])))]
      {:id (keyword (str (name (:id framework-row)) "-checked-fallback-boundary"))
       :kind :m6aj-checked-fallback-boundary
       :framework {:row (:id framework-row)
                   :kind (:kind framework-row)
                   :verdict (:gate/verdict framework-row)
                   :boundary/status (:boundary/status framework-row)
                   :promotion/allowed? (:promotion/allowed? framework-row)
                   :fallback (:fallback framework-row)
                   :ok (:ok framework-row)}
       :bytecode-witness nil
       :gate/verdict (if ok? :held :rejected)
       :held-reason (when ok? :checked-fallback-no-raw-lowering)
       :ok ok?})

    (= :accepted (:gate/verdict framework-row))
    (let [adm (get-in framework-row [:evidence :compiler-admission])
          ok? (and (:ok framework-row)
                   (true? (:promotion/allowed? framework-row))
                   (boolean (:raw-lowered? adm))
                   (boolean (:overflow-falls-back? adm)))]
      {:id (keyword (str (name (:id framework-row)) "-admission-accepted"))
       :kind :m6aj-engine-admitted-with-bytecode
       :framework {:row (:id framework-row)
                   :kind (:kind framework-row)
                   :verdict (:gate/verdict framework-row)
                   :admission-path (:admission-path framework-row)
                   :promotion/allowed? (:promotion/allowed? framework-row)
                   :ok (:ok framework-row)}
       :bytecode-witness {:case-ok (boolean (:raw-lowered? adm))
                          :overflow-falls-back? (boolean (:overflow-falls-back? adm))
                          :direct-opcodes (:direct-opcodes adm)}
       :gate/verdict (if ok? :accepted :rejected)
       :ok ok?})

    :else
    (let [ok? (and (:ok framework-row)
                   (= :held (:gate/verdict framework-row))
                   (false? (:promotion/allowed? framework-row)))]
      {:id (keyword (str (name (:id framework-row)) "-admission-held"))
       :kind :m6aj-proof-input-held-no-bytecode-admission
       :framework {:row (:id framework-row)
                   :kind (:kind framework-row)
                   :verdict (:gate/verdict framework-row)
                   :held-reason (:held-reason framework-row)
                   :promotion/allowed? (:promotion/allowed? framework-row)
                   :ok (:ok framework-row)}
       :bytecode-witness nil
       :gate/verdict (if ok? :held :rejected)
       :held-reason (when ok? :compiler-cfg-vc-admission-not-bound)
       :ok ok?})))

(defn run
  []
  (let [tv-report (tv/run)
        bw-report (bw/run)
        m6aj-report (m6aj/run)
        tv-rows (index-by-id (:rows tv-report))
        bw-cases (index-by-id (:cases bw-report))
        rows (vec (concat (map #(accepted-row tv-rows bw-cases %)
                               (accepted-pairs))
                          (map #(rejected-row tv-rows bw-cases %)
                               (rejected-pairs))
                          (map m6aj-row (:rows m6aj-report))))
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        held (filter #(= :held (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        canonical (mapv #(select-keys %
                                      [:id
                                       :kind
                                       :translation-validation
                                       :bytecode-witness
                                       :gate/verdict
                                       :held-reason
                                       :ok])
                        rows)
        invariants (sorted-map
                    :translation-validation-ok (:ok tv-report)
                    :bytecode-witness-ok (:ok bw-report)
                    :m6aj-framework-ok (:ok m6aj-report)
                    :all-rows-ok (every? :ok rows)
                    ;; tv-paired accepted rows(M6z~M6ai)는 VC digest 를 가져야 한다.
                    :accepted-rows-have-vc-digests
                    (every? #(string? (get-in % [:translation-validation :vc-digest]))
                            (filter :translation-validation accepted))
                    :accepted-rows-have-bytecode-witness
                    (every? #(true? (get-in % [:bytecode-witness :case-ok]))
                            accepted)
                    ;; M9b: engine-admitted M6aj 케이스(branch-dependent + mixed-sign)는 모두
                    ;; raw lowering bytecode 와 overflow fallback 증거를 가져야 한다.
                    :m6aj-engine-admitted-rows-have-bytecode
                    (let [es (filter #(= :m6aj-engine-admitted-with-bytecode (:kind %)) rows)]
                      (and (seq es)
                           (every? (fn [r]
                                     (and (= :accepted (:gate/verdict r))
                                          (true? (get-in r [:bytecode-witness :case-ok]))
                                          (true? (get-in r [:bytecode-witness :overflow-falls-back?]))))
                                   es)))
                    ;; 남은 relational/nonlinear proof-input 은 admission 없이 held.
                    :m6aj-proof-inputs-held-without-admission
                    (= (count (filter #(= :held (:gate/verdict %)) (:rows m6aj-report)))
                       (count (filter #(= :m6aj-proof-input-held-no-bytecode-admission
                                          (:kind %))
                                      held)))
                    ;; checked-fallback accepted in m6aj must map to held boundary here
                    :m6aj-checked-fallback-held
                    (let [fb (filter #(or (= :checked-fallback (:boundary/status %))
                                          (= :checked-fallback-boundary (:kind %)))
                                     (:rows m6aj-report))
                          mapped (filter #(= :m6aj-checked-fallback-boundary (:kind %))
                                         held)]
                      (and (= (count fb) (count mapped))
                           (every? :ok mapped)))
                    :rejected-vcs-held-or-fallback
                    (every? #(= :held (:gate/verdict %)) held)
                    :no-rejected-rows
                    (empty? rejected))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.lowering-admission.receipt.v1"
     :stage [:M10 :M6w]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "cross-check translation-validation VC digests against primitive bytecode witness evidence"
     :source-digests {:translation-validation (:canonical-receipt-digest tv-report)
                      :primitive-bytecode-witness (:canonical-receipt-digest bw-report)
                      :m6aj-framework (:canonical-receipt-digest m6aj-report)}
     :policy {:accepted "accepted VC must have matching bytecode opcode/evidence"
              :held "rejected VC must remain checked fallback or explicit refusal"
              :compiler-admission "no raw lowering without both VC and bytecode evidence"
              :m6aj "relation/ranking proof input remains held until compiler CFG/VC admission and bytecode witness are bound"}
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
                    (name (:gate/verdict row)))))
    (println (str "lowering admission: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
