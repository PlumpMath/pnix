(ns pnix.clj-meta.full-source-stage1
  "Full-source genuine stage1 boundary witness.

  This receipt answers a narrow question: how far is compiler.clj from a
  fallback-free stage1 where every top-level self-source form is compiled by
  the clj-meta backend? It is OK when the accounting is complete and honest,
  not when the gap is pretended closed."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.selfaudit :as audit])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/full-source-stage1.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- status-counts
  [rows]
  (let [freqs (frequencies (map :gate/verdict rows))]
    {:accepted (get freqs :accepted 0)
     :held (get freqs :held 0)
     :rejected (get freqs :rejected 0)}))

(defn- stage-target-row
  [stage]
  {:id :stage-target-direct-subset
   :kind :self-source-stage-target
   :evidence {:stage-target-safe-forms (:stage-target-safe-forms stage)
              :direct-subset-candidates (:direct-subset-candidates stage)
              :classes (:classes stage)
              :fixed-point (:fixed-point stage)}
   :gate/verdict (if (:fixed-point stage) :accepted :rejected)
   :ok (boolean (:fixed-point stage))})

(defn- ns-side-effect-boundary-row
  "compiler.clj 의 단일 `ns` form 경계. 이 form 은 backend 가 직접 컴파일하지만
  (0 host clojure.lang.Compiler, witnessed) require/import/in-ns 의 *runtime*
  side-effect 는 host runtime-lib 에 위임한다(§13 영구 경계 = clojure.core 위임과 동급).
  따라서 held 인 compiler 결함이 아니라 설계상 영구 runtime 경계다."
  [stage]
  (let [rows (:host-side-effect-rows stage)
        ops (vec (sort-by name (distinct (mapcat :host-maintained-ops rows))))
        backend-compiled? (boolean (:ns-side-effect-backend-compiled? stage))]
    {:id :namespace-runtime-side-effect-boundary
     :kind :permanent-runtime-boundary
     :evidence {:namespace-side-effect-forms (:host-side-effect-forms stage)
                :host-maintained-ops ops
                :backend-compiled? backend-compiled?
                :witness (:ns-side-effect-witness stage)}
     :gate/verdict (if backend-compiled? :accepted :held)
     :held-reason (when-not backend-compiled?
                    :namespace-form-needs-host-compiler)
     :boundary :host-runtime-lib-namespace-side-effect
     :promotion/allowed? false
     :ok (and backend-compiled? (every? #{:ns :import :require} ops))}))

(defn- genuine-stage1-row
  "fallback-free genuine stage1 = compiler.clj 의 어떤 top-level form 도 host
  clojure.lang.Compiler(eval)에 위임되지 않는다. namespace form 의 runtime
  side-effect(§13)는 fallback 이 아니다 — 그 form 도 backend 가 컴파일한다(witnessed)."
  [stage]
  (let [host-fallback (:host-compiler-fallback-forms stage)
        fallback-free? (and (zero? host-fallback)
                            (boolean (:ns-side-effect-backend-compiled? stage))
                            (zero? (:non-accounted-forms stage)))]
    {:id :fallback-free-genuine-stage1
     :kind :target-state
     :evidence {:fallback-free? fallback-free?
                :host-compiler-fallback-forms host-fallback
                :namespace-side-effect-forms (:host-side-effect-forms stage)
                :ns-side-effect-backend-compiled? (boolean (:ns-side-effect-backend-compiled? stage))
                :full-source-forms (:full-source-forms stage)
                :full-source-accounted-forms (:full-source-accounted-forms stage)}
     :gate/verdict (if fallback-free? :accepted :held)
     :held-reason (when-not fallback-free?
                    :host-compiler-fallback-remains)
     :promotion/allowed? false
     :ok true}))

(defn- non-accounted-row
  [stage]
  (let [n (:non-accounted-forms stage)]
    {:id :non-accounted-forms
     :kind :accounting-sentinel
     :evidence {:non-accounted-forms n
                :non-accounted (:non-accounted stage)}
     :gate/verdict (if (zero? n) :accepted :rejected)
     :ok (zero? n)}))

(defn run
  ([] (run (audit/run)))
  ([audit-report]
   (let [stage (:bytecode-stage-target audit-report)
         rows [(stage-target-row stage)
               (ns-side-effect-boundary-row stage)
               (genuine-stage1-row stage)
               (non-accounted-row stage)]
         accepted (filter #(= :accepted (:gate/verdict %)) rows)
         held (filter #(= :held (:gate/verdict %)) rows)
         rejected (filter #(= :rejected (:gate/verdict %)) rows)
         canonical (mapv #(select-keys %
                                       [:id
                                        :kind
                                        :evidence
                                        :gate/verdict
                                        :held-reason
                                        :promotion/allowed?
                                        :ok])
                         rows)
         invariants (sorted-map
                     :audit-ok (:ok audit-report)
                     :full-source-accounted (:full-source-accounted? stage)
                     :stage-target-fixed-point (:fixed-point stage)
                     :unknown-ops-empty (empty? (:unknown-ops audit-report))
                     :non-accounted-empty (zero? (:non-accounted-forms stage))
                     :namespace-side-effects-backend-compiled
                     (boolean (:ns-side-effect-backend-compiled? stage))
                     :genuine-stage1-not-claimed-while-host-compiler-fallback-remains
                     (if (pos? (:host-compiler-fallback-forms stage))
                       (= :held
                          (:gate/verdict
                           (first (filter #(= :fallback-free-genuine-stage1 (:id %))
                                          rows))))
                       true)
                     :no-product-launcher-consumption true
                     :no-rejected-rows (empty? rejected))
         ok? (every? true? (vals invariants))]
     {:schema "pnix.clj-meta.full-source-stage1.receipt.v1"
      :stage [:M12 :full-source-stage1]
      :target-goal "meta-circular stage15/N Clojure compiler"
      :desc "honest fallback-free stage1 boundary for compiler.clj self-source"
      :policy {:accepted "every self-source top-level form is compiled by the clj-meta backend (0 host clojure.lang.Compiler); namespace forms compile via backend with host runtime-lib side-effects (§13)"
               :held "fallback-free stage1 remains held while any form needs host clojure.lang.Compiler (deftype/general reify)"
               :rejected "unknown or non-accounted self-source forms fail closed"
               :runtime-boundary "require/import/in-ns side-effects use host runtime-lib (§13), same as clojure.core; this is not a compiler fallback"
               :not-consumed-by "pnix-clj launcher"}
      :rows rows
      :status-counts {:accepted (count accepted)
                      :held (count held)
                      :rejected (count rejected)}
      :invariants invariants
      :canonical-receipt canonical
      :canonical-receipt-digest (sha256-string (pr-str canonical))
      :ok ok?})))

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
    (println (str "full-source stage1: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
