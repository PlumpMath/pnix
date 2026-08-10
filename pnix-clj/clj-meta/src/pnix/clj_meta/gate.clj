(ns pnix.clj-meta.gate
  "M6 — 통합 게이트/receipt.

  meta 레인의 모든 검증(컴파일러 smoke / conformance / self-host 고정점 / 미니
  메타순환 평가기 / full-eval tower)을 한 번에 돌려 컴팩트 receipt 를 만든다.
  이 receipt 는 clj-meta 내부 증명 산출물이며, pnix-clj 런처 소비는 M7 금지 조건
  해제 전까지 연결하지 않는다.

  재현빌드 레인(stock Clojure)과는 별개다."
  (:require [pnix.clj-meta.compiler :as comp]
            [pnix.clj-meta.conformance :as conf]
            [pnix.clj-meta.kernel :as kernel]
            [pnix.clj-meta.mirror :as mirror]
            [pnix.clj-meta.selfhost :as sh]
            [pnix.clj-meta.selfaudit :as audit]
            [pnix.clj-meta.full-source-stage1 :as full-stage1]
            [pnix.clj-meta.quarantine :as quarantine]
            [pnix.clj-meta.longhorizon :as longhorizon]
            [pnix.clj-meta.crosshost :as crosshost]
            [pnix.clj-meta.openworld :as openworld]
            [pnix.clj-meta.stagen :as stagen]
            [pnix.clj-meta.bytecode-witness :as bw]
            [pnix.clj-meta.abstract-interval :as ai]
            [pnix.clj-meta.abstract-octagon :as ao]
            [pnix.clj-meta.linear-ranking :as ranking]
            [pnix.clj-meta.m6aj-framework :as m6aj]
            [pnix.clj-meta.range-migration :as range-migration]
            [pnix.clj-meta.translation-validation :as tv]
            [pnix.clj-meta.lowering-admission :as lowering-admission]
            [pnix.clj-meta.bytecode-verifier :as verifier]
            [pnix.clj-meta.verified-compile :as verified-compile]
            [pnix.clj-meta.determinism-policy :as determinism]
            [pnix.clj-meta.diverse-double-compile :as ddc]
            [pnix.clj-meta.reproducible-ddc :as repro-ddc]
            [pnix.clj-meta.language-surface :as lang-surface]
            [pnix.clj-meta.frontend-selfhost :as frontend-selfhost]
            [pnix.clj-meta.runtime-selfhost :as runtime-selfhost]
            [clojure.java.io :as io]
            [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/metacircular.receipt.edn")
(def stage11-receipt-path
  "clj-meta/proof/stage11-multisurface.receipt.edn")
(def stage12-receipt-path
  "clj-meta/proof/stage12-quarantine.receipt.edn")
(def stage13-receipt-path
  "clj-meta/proof/stage13-long-horizon.receipt.edn")
(def stage14-receipt-path
  "clj-meta/proof/stage14-crosshost.receipt.edn")
(def stage15-receipt-path
  "clj-meta/proof/stage15-openworld.receipt.edn")
(def stagen-receipt-path
  "clj-meta/proof/stageN-recursive.receipt.edn")
(def bytecode-witness-receipt-path bw/receipt-path)
(def abstract-interval-receipt-path ai/receipt-path)
(def abstract-octagon-receipt-path ao/receipt-path)
(def linear-ranking-receipt-path ranking/receipt-path)
(def m6aj-framework-receipt-path m6aj/receipt-path)
(def range-migration-receipt-path range-migration/receipt-path)
(def translation-validation-receipt-path tv/receipt-path)
(def lowering-admission-receipt-path lowering-admission/receipt-path)
(def bytecode-verifier-receipt-path verifier/receipt-path)
(def verified-compile-receipt-path verified-compile/receipt-path)
(def determinism-policy-receipt-path determinism/receipt-path)
(def diverse-double-compile-receipt-path ddc/receipt-path)
(def reproducible-ddc-receipt-path repro-ddc/receipt-path)
(def language-surface-receipt-path lang-surface/receipt-path)
(def frontend-selfhost-receipt-path frontend-selfhost/receipt-path)
(def runtime-selfhost-receipt-path runtime-selfhost/receipt-path)
(def full-source-stage1-receipt-path full-stage1/receipt-path)

(def ^:private stage11-required-surfaces
  [:source-form
   :self-source-target
   :artifact-runtime
   :mirror-ir
   :conformance-corpus
   :kernel-evaluator-target
   :selfhost-chain
   :fallback-boundary])

(defn- boundary-policy
  []
  ;; 경계(hy-meta CPython 위임 대응, todo §13): 우리 emit 부분집합은 host Compiler
  ;; 0회. 미구현 namespace/type-generation op 는 host eval 위임(fallback).
  ;; runtime/read 도 host 위임.
  {:our-emit-uses-host-compiler false   ; 직접 emit 부분집합은 host Compiler 0회
   :fallback-to-host-compiler   true    ; 미구현/host 유지 op(ns/deftype/reify/…)
   :runtime-lib-delegated-to-host true  ; clojure.core
   :read-delegated-to-host true})       ; clojure reader

(defn- proof-claim-policy
  []
  {:stage1->7 "backend self-emit determinism and self-consistency fixed point; not a language-correctness proof"
   :frames "JVM StackMapTable/max stack computation is delegated to clojure.asm ClassWriter(COMPUTE_FRAMES|COMPUTE_MAXS)"
   :trusted-host-base [:tools.analyzer.jvm :clojure-reader :clojure-core-runtime :jvm]
   :ddc "cross-host/DDC lanes are emit-determinism evidence; full Wheeler independent compiler-binary DDC remains held"
   :direct-emit "direct backend paths use host clojure.lang.Compiler 0 times; unsupported top-level forms may still use explicit host fallback"
   :not-claimed [:full-language-correctness
                 :trusting-trust-defense
                 :independent-runtime
                 :independent-reader
                 :manual-stackmap-frame-proof]})

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- status-counts
  [rows]
  (let [freqs (frequencies (map :status rows))]
    (into (array-map)
          (map (fn [k] [k (get freqs k 0)])
               [:accepted :held :fallback :rejected]))))

(defn- stage11-row
  ([surface adapter status ok evidence]
   (stage11-row surface adapter status ok evidence nil))
  ([surface adapter status ok evidence held-reason]
   (cond-> {:surface surface
            :adapter adapter
            :status status
            :ok (boolean ok)
            :evidence evidence}
     held-reason (assoc :held-reason held-reason))))

(defn- canonical-stage11-row
  [{:keys [surface adapter status ok evidence held-reason]}]
  (cond-> (sorted-map :adapter adapter
                      :evidence evidence
                      :ok ok
                      :status status
                      :surface surface)
    held-reason (assoc :held-reason held-reason)))

(defn- stage11-report
  [cs cf cn mir aud m5a m5b twr]
  (let [kernel-checked (filter #(boolean? (:k-ok %)) cf)
        kernel-held    (filter #(= :unsupported (:k-ok %)) cf)
        stage-target   (:bytecode-stage-target aud)
        disk-proof     (:generated-stage-chain-disk-proof aud)
        stage9         (:stage9-clean-process aud)
        stage10        (:stage10-isolated-matrix aud)
        allowed-host   #{:import :ns}
        host-maint     (or (:host-maintained-present aud) [])
        compiler-ok?   (every? :ok cs)
        mirror-ok?     (every? :ok mir)
        conf-ok?       (every? :hc-ok cf)
        neg-ok?        (every? :ok cn)
        kernel-ok?     (not-any? false? (map :k-ok cf))
        fallback-ok?   (and (empty? (:unknown-ops aud))
                            (every? allowed-host host-maint))
        rows           [(stage11-row
                         :source-form
                         :self-source-audit
                         :accepted
                         (and (:ok aud) (empty? (:unknown-ops aud)))
                         (sorted-map
                          :analyzed-forms (:analyzed-forms aud)
                          :direct-subset-forms (get-in aud [:top-level :direct-subset-forms])
                          :host-maintained-present host-maint
                          :top-level-forms (:top-level-forms aud)
                          :unknown-ops (:unknown-ops aud)))
                        (stage11-row
                         :self-source-target
                         :bytecode-stage-target
                         :accepted
                         (:ok stage-target)
                         (sorted-map
                          :classes (:classes stage-target)
                          :direct-subset-candidates (:direct-subset-candidates stage-target)
                          :fixed-point (:fixed-point stage-target)
                          :full-source-accounted-forms (:full-source-accounted-forms stage-target)
                          :full-source-forms (:full-source-forms stage-target)
                          :host-side-effect-forms (:host-side-effect-forms stage-target)
                          :stage-target-safe-forms (:stage-target-safe-forms stage-target)))
                        (stage11-row
                         :artifact-runtime
                         :stage8-stage10-artifact-runtime
                         :accepted
                         (and (:ok disk-proof)
                              (:ok stage9)
                              (:ok stage10))
                         (sorted-map
                          :stage8-disk-proof-ok (:ok disk-proof)
                          :stage8-fresh-exact (:fresh-stage8-exact? disk-proof)
                          :stage9-clean-process-ok (:ok stage9)
                          :stage9-canonical-digest (:canonical-receipt-digest stage9)
                          :stage10-isolated-matrix-ok (:ok stage10)
                          :stage10-cwd-policy (:cwd-policy stage10)
                          :stage10-variants (count (:rows stage10))))
                        (stage11-row
                         :mirror-ir
                         :mirror
                         :accepted
                         mirror-ok?
                         (sorted-map
                          :ok (count (filter :ok mir))
                          :total (count mir)))
                        (stage11-row
                         :conformance-corpus
                         :host-compiler-kernel-corpus
                         :accepted
                         (and compiler-ok? conf-ok? neg-ok? kernel-ok?)
                         (sorted-map
                          :compiler-smoke-ok (count (filter :ok cs))
                          :compiler-smoke-total (count cs)
                          :host-compiler-ok (count (filter :hc-ok cf))
                          :host-compiler-total (count cf)
                          :kernel-checked (count kernel-checked)
                          :kernel-held (count kernel-held)
                          :kernel-pass (count (filter #(true? (:k-ok %)) cf))
                          :negative-ok (count (filter :ok cn))
                          :negative-total (count cn)))
                        (stage11-row
                         :kernel-evaluator-target
                         :selfhost-mini-eval-and-tower
                         :accepted
                         (and (:conformance-ok m5b)
                              (:fixed-point m5b)
                              (:ok twr))
                         (sorted-map
                          :mini-eval-classes (:classes m5b)
                          :mini-eval-conformance (:conformance-ok m5b)
                          :mini-eval-fixed-point (:fixed-point m5b)
                          :mini-eval-rows (count (:rows m5b))
                          :tower-ok (:ok twr)
                          :tower-rows (count (:rows twr))))
                        (stage11-row
                         :selfhost-chain
                         :selfhost-stage1-7
                         :accepted
                         (:fixed-point m5a)
                         (sorted-map
                          :classes (:classes m5a)
                          :fixed-point (:fixed-point m5a)
                          :stages (:stages m5a)))
                        (stage11-row
                         :fallback-boundary
                         :policy-ledger
                         :held
                         fallback-ok?
                         (sorted-map
                          :allowed-host-maintained (vec (sort-by name allowed-host))
                          :boundary (boundary-policy)
                          :host-maintained-present host-maint
                          :unknown-ops (:unknown-ops aud))
                         :explicit-host-maintained-boundary)]
        present         (set (map :surface rows))
        missing         (vec (remove present stage11-required-surfaces))
        rejected        (filter #(= :rejected (:status %)) rows)
        failed          (remove :ok rows)
        canonical       (mapv canonical-stage11-row rows)
        digest          (sha256-string (pr-str canonical))
        ok?             (and (empty? missing)
                             (empty? rejected)
                             (empty? failed))]
    {:schema "pnix.clj-meta.stage11.multisurface.receipt.v1"
     :stage 11
     :desc "stage11 multi-surface compiler adapter closure"
     :required-surfaces stage11-required-surfaces
     :rows rows
     :status-counts (status-counts rows)
     :missing-surfaces missing
     :canonical-receipt canonical
     :canonical-receipt-digest digest
     :ok ok?}))

(defn write-stage11-receipt!
  [r]
  (io/make-parents stage11-receipt-path)
  (spit stage11-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stage12-receipt!
  [r]
  (io/make-parents stage12-receipt-path)
  (spit stage12-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stage13-receipt!
  [r]
  (io/make-parents stage13-receipt-path)
  (spit stage13-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stage14-receipt!
  [r]
  (io/make-parents stage14-receipt-path)
  (spit stage14-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stage15-receipt!
  [r]
  (io/make-parents stage15-receipt-path)
  (spit stage15-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stagen-receipt!
  [r]
  (io/make-parents stagen-receipt-path)
  (spit stagen-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-bytecode-witness-receipt!
  [r]
  (io/make-parents bytecode-witness-receipt-path)
  (spit bytecode-witness-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-abstract-interval-receipt!
  [r]
  (io/make-parents abstract-interval-receipt-path)
  (spit abstract-interval-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-abstract-octagon-receipt!
  [r]
  (io/make-parents abstract-octagon-receipt-path)
  (spit abstract-octagon-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-linear-ranking-receipt!
  [r]
  (io/make-parents linear-ranking-receipt-path)
  (spit linear-ranking-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-m6aj-framework-receipt!
  [r]
  (io/make-parents m6aj-framework-receipt-path)
  (spit m6aj-framework-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-range-migration-receipt!
  [r]
  (io/make-parents range-migration-receipt-path)
  (spit range-migration-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-translation-validation-receipt!
  [r]
  (io/make-parents translation-validation-receipt-path)
  (spit translation-validation-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-lowering-admission-receipt!
  [r]
  (io/make-parents lowering-admission-receipt-path)
  (spit lowering-admission-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-bytecode-verifier-receipt!
  [r]
  (io/make-parents bytecode-verifier-receipt-path)
  (spit bytecode-verifier-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-verified-compile-receipt!
  [r]
  (io/make-parents verified-compile-receipt-path)
  (spit verified-compile-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-determinism-policy-receipt!
  [r]
  (io/make-parents determinism-policy-receipt-path)
  (spit determinism-policy-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-diverse-double-compile-receipt!
  [r]
  (io/make-parents diverse-double-compile-receipt-path)
  (spit diverse-double-compile-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-reproducible-ddc-receipt!
  [r]
  (io/make-parents reproducible-ddc-receipt-path)
  (spit reproducible-ddc-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-full-source-stage1-receipt!
  [r]
  (io/make-parents full-source-stage1-receipt-path)
  (spit full-source-stage1-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-language-surface-receipt!
  [r]
  (io/make-parents language-surface-receipt-path)
  (spit language-surface-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-frontend-selfhost-receipt!
  [r]
  (io/make-parents frontend-selfhost-receipt-path)
  (spit frontend-selfhost-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-runtime-selfhost-receipt!
  [r]
  (io/make-parents runtime-selfhost-receipt-path)
  (spit runtime-selfhost-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn report
  []
  (let [cs   (comp/run-smoke)
        ks   (kernel/run-smoke)
        cf   (conf/run)
        cn   (conf/run-negative)
        mir  (mirror/run-smoke)
        aud  (audit/run)
        full-source-stage1 (full-stage1/run aud)
        m5a  (sh/run 7)
        m5b  (sh/mini-eval-report)
        twr  (sh/tower-report)
        kchk (filter #(boolean? (:k-ok %)) cf)
        stage11 (stage11-report cs cf cn mir aud m5a m5b twr)
        stage12 (quarantine/run stage11)
        stage13 (longhorizon/run stage11 stage12)
        stage14 (crosshost/run stage13)
        stage15 (openworld/run stage14)
        stagen  (stagen/run {:stage8  (:generated-stage-chain-disk-proof aud)
                             :stage9  (:stage9-clean-process aud)
                             :stage10 (:stage10-isolated-matrix aud)
                             :stage11 stage11
                             :stage12 stage12
                             :stage13 stage13
                             :stage14 stage14
                             :stage15 stage15})
        bytecode-witness (bw/run)
        abstract-interval (ai/run)
        abstract-octagon (ao/run)
        linear-ranking (ranking/run)
        m6aj-framework (m6aj/run)
        range-migration (range-migration/run)
        translation-validation (tv/run)
        lowering-admission (lowering-admission/run)
        bytecode-verifier (verifier/run)
        verified-compile (verified-compile/run)
        determinism-policy (determinism/run)
        diverse-double-compile (ddc/run aud)
        reproducible-ddc (repro-ddc/run)
        language-surface (lang-surface/run)
        frontend-selfhost (frontend-selfhost/run)
        runtime-selfhost (runtime-selfhost/run)
        ok   (and (every? :ok cs)
                  (every? :ok ks)
                  (every? :hc-ok cf)
                  (every? :ok cn)
                  (every? :ok mir)
                  (:ok aud)
                  (:ok full-source-stage1)
                  (:fixed-point m5a)
                  (:conformance-ok m5b)
                  (:fixed-point m5b)
                  (:ok twr)
                  (:ok stage11)
                  (:ok stage12)
                  (:ok stage13)
                  (:ok stage14)
                  (:ok stage15)
                  (:ok stagen)
                  (:ok bytecode-witness)
                  (:ok abstract-interval)
                  (:ok abstract-octagon)
                  (:ok linear-ranking)
                  (:ok m6aj-framework)
                  (:ok range-migration)
                  (:ok translation-validation)
                  (:ok lowering-admission)
                  (:ok bytecode-verifier)
                  (:ok verified-compile)
                  (:ok determinism-policy)
                  (:ok diverse-double-compile)
                  (:ok reproducible-ddc)
                  (:ok language-surface)
                  (:ok frontend-selfhost)
                  (:ok runtime-selfhost))]
    {:schema "pnix.clj-meta.metacircular.receipt.v1"
     :lane "clojure-self-written-compiler (host Compiler 0회)"
     :compiler-smoke {:ok (count (filter :ok cs)) :total (count cs)}
     :kernel-smoke {:ok (count (filter :ok ks)) :total (count ks)}
     :mirror-smoke   {:ok (count (filter :ok mir)) :total (count mir)}
     :self-source-audit {:ok (:ok aud)
                         :top-level-forms (:top-level-forms aud)
                         :analyzed-forms (:analyzed-forms aud)
                         :op-count (count (:op-census aud))
                         :unknown-ops (:unknown-ops aud)
                         :host-maintained-present (:host-maintained-present aud)
                         :direct-subset-forms (get-in aud [:top-level :direct-subset-forms])
                         :bytecode-stage-target
                         (select-keys (:bytecode-stage-target aud)
                                      [:stage-target-safe-forms
                                       :direct-subset-candidates
                                       :full-source-forms
                                       :full-source-accounted-forms
                                       :host-side-effect-forms
                                       ;; M12: host clojure.lang.Compiler fallback 0 + ns form backend-compiled
                                       :host-compiler-fallback-forms
                                       :ns-side-effect-backend-compiled?
                                       :classes
                                       :fixed-point])
                         :compiled-execution-smokes
                         (mapv #(select-keys % [:desc :ok])
                               (:compiled-execution-smokes aud))
                         :generated-stage-chain
                         (select-keys (:generated-stage-chain aud)
                                      [:ok
                                       :stages
                                       :classes
                                       :fixed-point
                                       :matches-host-reference?
                                       :disk-fixed-point
                                       :disk-matches-host-reference?
                                       :disk-reload-ok?
                                       :fresh-stage8-exact?
                                       :stage-receipt-match?
                                       :loaded-ok?
                                       :generated?])
                         :generated-stage-chain-disk
                         (select-keys (get-in aud [:generated-stage-chain :disk-bundles])
                                      [:root
                                       :fixed-point
                                       :matches-host-reference?
                                       :jarproof-compatible])
                         :generated-stage-chain-disk-proof
                         (select-keys (:generated-stage-chain-disk-proof aud)
                                      [:ok
                                       :root
                                       :compare-report
                                       :compare-ok?
                                       :disk-fixed-point
                                       :disk-matches-host-reference?
                                       :disk-reload-ok?
                                       :fresh-stage8-exact?])
                         :stage9-clean-process
                         (select-keys (:stage9-clean-process aud)
                                      [:ok
                                       :child-exit
                                       :child-receipt
                                       :compare-report
                                       :compare-ok?
                                       :artifact-match?
                                       :canonical-receipt-digest
                                       :payload-result
                                       :payload-same-bytecode?
                                       :target-got])
                         :stage10-isolated-matrix
                         (select-keys (:stage10-isolated-matrix aud)
                                      [:ok
                                       :compare-report
                                       :compare-ok?
                                       :canonical-fixed?
                                       :artifact-fixed?
                                       :expected-canonical-receipt-digest
                                       :expected-artifact-digest
                                       :cwd-policy])}
     :full-source-stage1 full-source-stage1
     :conformance   {:host=compiler (str (count (filter :hc-ok cf)) "/" (count cf))
                     :negative      (str (count (filter :ok cn)) "/" (count cn))
                     :kernel-3way   (str (count (filter #(true? (:k-ok %)) cf)) "/" (count kchk))}
     :m5a-self-compile-fixed-point {:ok (:fixed-point m5a) :classes (:classes m5a)}
     :m5b-compiler-compiles-evaluator {:conformance (:conformance-ok m5b)
                                       :fixed-point (:fixed-point m5b)
                                       :classes (:classes m5b)}
     :full-eval-tower {:ok (:ok twr)
                       :rows (count (:rows twr))}
     :stage11-multisurface stage11
     :stage12-quarantine stage12
     :stage13-long-horizon stage13
     :stage14-crosshost stage14
     :stage15-openworld stage15
     :stageN-recursive stagen
     :primitive-bytecode-witness bytecode-witness
     :abstract-interval abstract-interval
     :abstract-octagon abstract-octagon
     :linear-ranking linear-ranking
     :m6aj-framework m6aj-framework
     :range-migration range-migration
     :translation-validation translation-validation
     :lowering-admission lowering-admission
     :bytecode-verifier bytecode-verifier
     :verified-compile verified-compile
     :determinism-policy determinism-policy
     :diverse-double-compile diverse-double-compile
     :reproducible-ddc reproducible-ddc
     :language-surface language-surface
     :frontend-selfhost frontend-selfhost
     :runtime-selfhost runtime-selfhost
     :boundary (boundary-policy)
     :proof-claim (proof-claim-policy)
     :target-goal "meta-circular stage15/N Clojure compiler"
     :current-proof-stage "stageN recursive closure ladder + full-source stage1 boundary + generated-name determinism policy + primitive bytecode/direct arithmetic witness + M9 interval/octagon/ranking domain + M6aj framework/range migration witness + M10 translation validation/lowering admission witness + M11 DDC trust witness + M12 language-surface boundary witness + U6 tiny frontend witness + U7 runtime helper witness + M13 verifier/loadability + verified compiler artifact hard-fail witness"
     :goal "meta-circular stage15/N"
     :ready ok}))

(defn -main
  [& _]
  (let [r (report)]
    (io/make-parents receipt-path)
    (spit receipt-path (with-out-str (pp/pprint r)))
    (write-stage11-receipt! (:stage11-multisurface r))
    (write-stage12-receipt! (:stage12-quarantine r))
    (write-stage13-receipt! (:stage13-long-horizon r))
    (write-stage14-receipt! (:stage14-crosshost r))
    (write-stage15-receipt! (:stage15-openworld r))
    (write-stagen-receipt! (:stageN-recursive r))
    (write-full-source-stage1-receipt! (:full-source-stage1 r))
    (write-bytecode-witness-receipt! (:primitive-bytecode-witness r))
    (write-abstract-interval-receipt! (:abstract-interval r))
    (write-abstract-octagon-receipt! (:abstract-octagon r))
    (write-linear-ranking-receipt! (:linear-ranking r))
    (write-m6aj-framework-receipt! (:m6aj-framework r))
    (write-range-migration-receipt! (:range-migration r))
    (write-translation-validation-receipt! (:translation-validation r))
    (write-lowering-admission-receipt! (:lowering-admission r))
    (write-bytecode-verifier-receipt! (:bytecode-verifier r))
    (write-verified-compile-receipt! (:verified-compile r))
    (write-determinism-policy-receipt! (:determinism-policy r))
    (write-diverse-double-compile-receipt! (:diverse-double-compile r))
    (write-reproducible-ddc-receipt! (:reproducible-ddc r))
    (write-language-surface-receipt! (:language-surface r))
    (write-frontend-selfhost-receipt! (:frontend-selfhost r))
    (write-runtime-selfhost-receipt! (:runtime-selfhost r))
    (pp/pprint r)
    (println (str "\nstage11 multisurface: "
                  (if (get-in r [:stage11-multisurface :ok]) "OK" "FAILED")
                  "  (receipt: " stage11-receipt-path
                  ", digest: "
                  (get-in r [:stage11-multisurface :canonical-receipt-digest])
                  ")"))
    (println (str "stage12 quarantine: "
                  (if (get-in r [:stage12-quarantine :ok]) "OK" "FAILED")
                  "  (receipt: " stage12-receipt-path
                  ", digest: "
                  (get-in r [:stage12-quarantine :canonical-receipt-digest])
                  ")"))
    (println (str "stage13 long-horizon: "
                  (if (get-in r [:stage13-long-horizon :ok]) "OK" "FAILED")
                  "  (receipt: " stage13-receipt-path
                  ", digest: "
                  (get-in r [:stage13-long-horizon :canonical-receipt-digest])
                  ")"))
    (println (str "stage14 crosshost: "
                  (if (get-in r [:stage14-crosshost :ok]) "OK" "FAILED")
                  "  (receipt: " stage14-receipt-path
                  ", digest: "
                  (get-in r [:stage14-crosshost :canonical-receipt-digest])
                  ")"))
    (println (str "stage15 openworld: "
                  (if (get-in r [:stage15-openworld :ok]) "OK" "FAILED")
                  "  (receipt: " stage15-receipt-path
                  ", digest: "
                  (get-in r [:stage15-openworld :canonical-receipt-digest])
                  ")"))
    (println (str "stageN recursive: "
                  (if (get-in r [:stageN-recursive :ok]) "OK" "FAILED")
                  "  (receipt: " stagen-receipt-path
                  ", digest: "
                  (get-in r [:stageN-recursive :canonical-receipt-digest])
                  ")"))
    (println (str "full-source stage1: "
                  (if (get-in r [:full-source-stage1 :ok]) "OK" "FAILED")
                  "  (receipt: " full-source-stage1-receipt-path
                  ", digest: "
                  (get-in r [:full-source-stage1 :canonical-receipt-digest])
                  ")"))
    (let [bst (get-in r [:self-source-audit :bytecode-stage-target])]
      (println (str "  M12 fallback-free genuine stage1: "
                    (if (and (zero? (:host-compiler-fallback-forms bst 1))
                             (:ns-side-effect-backend-compiled? bst))
                      "ACCEPTED" "HELD")
                    "  (host-clojure.lang.Compiler-fallback-forms="
                    (:host-compiler-fallback-forms bst)
                    ", ns-form-backend-compiled="
                    (:ns-side-effect-backend-compiled? bst)
                    ")")))
    (println (str "primitive bytecode witness: "
                  (if (get-in r [:primitive-bytecode-witness :ok]) "OK" "FAILED")
                  "  (receipt: " bytecode-witness-receipt-path
                  ", digest: "
                  (get-in r [:primitive-bytecode-witness :canonical-receipt-digest])
                  ")"))
    (println (str "abstract interval: "
                  (if (get-in r [:abstract-interval :ok]) "OK" "FAILED")
                  "  (receipt: " abstract-interval-receipt-path
                  ", digest: "
                  (get-in r [:abstract-interval :canonical-receipt-digest])
                  ")"))
    (println (str "abstract octagon: "
                  (if (get-in r [:abstract-octagon :ok]) "OK" "FAILED")
                  "  (receipt: " abstract-octagon-receipt-path
                  ", digest: "
                  (get-in r [:abstract-octagon :canonical-receipt-digest])
                  ")"))
    (println (str "linear ranking: "
                  (if (get-in r [:linear-ranking :ok]) "OK" "FAILED")
                  "  (receipt: " linear-ranking-receipt-path
                  ", digest: "
                  (get-in r [:linear-ranking :canonical-receipt-digest])
                  ")"))
    (println (str "M6aj framework: "
                  (if (get-in r [:m6aj-framework :ok]) "OK" "FAILED")
                  "  (receipt: " m6aj-framework-receipt-path
                  ", digest: "
                  (get-in r [:m6aj-framework :canonical-receipt-digest])
                  ")"))
    (println (str "range migration: "
                  (if (get-in r [:range-migration :ok]) "OK" "FAILED")
                  "  (receipt: " range-migration-receipt-path
                  ", digest: "
                  (get-in r [:range-migration :canonical-receipt-digest])
                  ")"))
    (println (str "translation validation: "
                  (if (get-in r [:translation-validation :ok]) "OK" "FAILED")
                  "  (receipt: " translation-validation-receipt-path
                  ", digest: "
                  (get-in r [:translation-validation :canonical-receipt-digest])
                  ")"))
    (println (str "lowering admission: "
                  (if (get-in r [:lowering-admission :ok]) "OK" "FAILED")
                  "  (receipt: " lowering-admission-receipt-path
                  ", digest: "
                  (get-in r [:lowering-admission :canonical-receipt-digest])
                  ")"))
    (println (str "bytecode verifier: "
                  (if (get-in r [:bytecode-verifier :ok]) "OK" "FAILED")
                  "  (receipt: " bytecode-verifier-receipt-path
                  ", digest: "
                  (get-in r [:bytecode-verifier :canonical-receipt-digest])
                  ")"))
    (println (str "verified compile: "
                  (if (get-in r [:verified-compile :ok]) "OK" "FAILED")
                  "  (receipt: " verified-compile-receipt-path
                  ", digest: "
                  (get-in r [:verified-compile :canonical-receipt-digest])
                  ")"))
    (println (str "determinism policy: "
                  (if (get-in r [:determinism-policy :ok]) "OK" "FAILED")
                  "  (receipt: " determinism-policy-receipt-path
                  ", digest: "
                  (get-in r [:determinism-policy :canonical-receipt-digest])
                  ")"))
    (println (str "diverse double compile: "
                  (if (get-in r [:diverse-double-compile :ok]) "OK" "FAILED")
                  "  (receipt: " diverse-double-compile-receipt-path
                  ", digest: "
                  (get-in r [:diverse-double-compile :canonical-receipt-digest])
                  ")"))
    (println (str "reproducible DDC lane: "
                  (if (get-in r [:reproducible-ddc :ok]) "OK" "FAILED")
                  "  (receipt: " reproducible-ddc-receipt-path
                  ", digest: "
                  (get-in r [:reproducible-ddc :canonical-receipt-digest])
                  ")"))
    (println (str "language surface: "
                  (if (get-in r [:language-surface :ok]) "OK" "FAILED")
                  "  (receipt: " language-surface-receipt-path
                  ", digest: "
                  (get-in r [:language-surface :canonical-receipt-digest])
                  ")"))
    (println (str "frontend selfhost: "
                  (if (get-in r [:frontend-selfhost :ok]) "OK" "FAILED")
                  "  (receipt: " frontend-selfhost-receipt-path
                  ", digest: "
                  (get-in r [:frontend-selfhost :canonical-receipt-digest])
                  ")"))
    (println (str "runtime selfhost: "
                  (if (get-in r [:runtime-selfhost :ok]) "OK" "FAILED")
                  "  (receipt: " runtime-selfhost-receipt-path
                  ", digest: "
                  (get-in r [:runtime-selfhost :canonical-receipt-digest])
                  ")"))
    (println (str "\nmetacircular gate: " (if (:ready r) "READY ✅" "NOT READY ❌")
                  "  (receipt: " receipt-path ")"))
    (println "proof claim: stage1->7 fixed point = backend determinism+self-consistency; frames=ASM COMPUTE_FRAMES; analyzer/reader/runtime=trusted host base; cross-host DDC is emit-determinism evidence, not full Trusting-Trust defense.")
    (shutdown-agents)
    (when-not (:ready r)
      (System/exit 1))))
