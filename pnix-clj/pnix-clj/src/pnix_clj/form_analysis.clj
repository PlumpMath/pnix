(ns pnix-clj.form-analysis
  "Clojure-on-Clojure AST-pass lane (deep-research F4) — a reusable static
  analysis substrate over Clojure forms built on tools.analyzer.jvm, the
  canonical manipulable Clojure AST a Python/Hy host fundamentally lacks
  (clojure.lang.Compiler exposes no such AST; the analyzer's whole reason to
  exist is that gap).

  `analyze-form` runs the analyzer and walks the resulting AST to report:
  - RESOLVABILITY: does the form analyze at all? An unresolvable var / class is
    a deterministic analysis failure (the analyzer throws; we structure it).
  - OP HISTOGRAM: the node-op fingerprint (:if / :let / :invoke / :static-call…).
  - HOST-INTEROP SURFACE: every :new / :instance-call / non-allowlisted
    :static-call with its class+method — the form's effect surface at the AST
    level, distinct from a name-based scan.
  - PURE-CORE verdict: the form touches no host beyond a small numeric/core
    allowlist (`+`/`Math` compile to :static-call on clojure.lang.Numbers /
    java.lang.Math, which are pure and allowlisted).

  This is a substrate, not a proof: it classifies the STATIC host surface of a
  Clojure form (usable by safe-eval / synthesize to reject impure projections
  before compiling), and is exact for what the JVM analyzer resolves."
  (:require [clojure.tools.analyzer.ast :as ast]
            [clojure.tools.analyzer.jvm :as analyzer]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :host-form-static-analysis-boundary
   :role :classify-clojure-form-host-surface-before-compile
   :product-runtime :allowed
   :semantic-authority :static-classification-only
   :behavior-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :determinism :analyzer-result-required
   :allowed-output :form-analysis-host-surface-report})

(def ^:private pure-static-classes
  "Classes whose static calls are pure value operations (numeric tower, core
  runtime helpers) — the `+`/`*`/`Math` surface Clojure lowers arithmetic to."
  #{clojure.lang.Numbers clojure.lang.RT clojure.lang.Util
    java.lang.Math java.lang.Long java.lang.Double java.lang.Integer
    java.lang.Boolean java.lang.Character java.lang.String})

(defn- node-class
  "The referenced class of a host node (:new carries it as a :const child)."
  [n]
  (let [c (:class n)]
    (if (map? c) (:val c) c)))

(defn- host-node?
  [n]
  (case (:op n)
    (:new :instance-call :instance-field :host-interop) true
    :static-call (not (contains? pure-static-classes (node-class n)))
    false))

(defn analyze-form
  "Static analysis of one Clojure `form`. Returns
  {:status :ok/:failed ... :op-histogram :host-interop :pure-core?} — never
  throws (an unresolvable var/class becomes a structured failure)."
  [form]
  (try
    (let [ast (analyzer/analyze form)
          nodes (ast/nodes ast)
          host (->> nodes
                    (filter host-node?)
                    (mapv (fn [n]
                            (let [c (node-class n)]
                              {:op (:op n)
                               :class (if (class? c) (.getName ^Class c) (some-> c str))
                               :method (some-> (:method n) name)}))))]
      {:status :ok
       :node-count (count nodes)
       :op-histogram (into (sorted-map) (frequencies (map :op nodes)))
       :host-interop host
       :pure-core? (empty? host)})
    (catch clojure.lang.ExceptionInfo _
      {:status :failed
       :reason :form-does-not-analyze
       :error {:phase :host-analysis
               :class :form-does-not-analyze}})
    (catch Throwable _
      {:status :failed
       :reason :form-analysis-threw
       :error {:phase :host-analysis
               :class :form-analysis-failed}})))

;; ---- report: classify a corpus, verifying the analyzer agrees -----------

(def analysis-cases
  "id, form, and the EXPECTED classification (pure-core? / analyzes?)."
  [{:id :pure-arith        :form '(+ (* 2 3) 4)          :pure-core? true}
   {:id :pure-let-if       :form '(let [x 40] (if (pos? x) (+ x 2) 0)) :pure-core? true}
   {:id :pure-fn-apply     :form '((fn [a b] (+ a b)) 3 4) :pure-core? true}
   {:id :pure-core-coll    :form '(reduce + 0 [1 2 3])   :pure-core? true}
   {:id :host-new          :form '(java.util.ArrayList.)  :pure-core? false}
   {:id :host-instance     :form '(.toUpperCase "hi")     :pure-core? false}
   {:id :host-static       :form '(System/getProperty "user.home") :pure-core? false}
   {:id :unresolvable      :form 'definitely-not-a-var    :analyzes? false}])

(defn- run-case
  [{:keys [id form pure-core? analyzes?] :as c}]
  (let [a (analyze-form form)
        analyzes-ok? (= :ok (:status a))
        expect-analyzes? (get c :analyzes? true)
        ok? (if (not expect-analyzes?)
              (= :failed (:status a))
              (and analyzes-ok?
                   (= pure-core? (:pure-core? a))))]
    {:id id
     :status (if ok? :accepted :rejected)
     :analysis-status (:status a)
     :pure-core? (:pure-core? a)
     :expected-pure-core? pure-core?
     :host-interop (:host-interop a)
     :op-histogram (:op-histogram a)}))

(defn report
  []
  (let [rows (mapv run-case analysis-cases)
        rejected (count (remove #(= :accepted (:status %)) rows))
        body {:kind :pnix-form-analysis-report
              :schema :pnix-clj.form-analysis-report.v0
              :policy :tools-analyzer-jvm-ast-pass-host-surface-classification
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :rows rows}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash rows))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected rows]} (report)]
    (println (format "pnix-clj form-analysis: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (doseq [{:keys [id status analysis-status pure-core? host-interop]} rows]
      (println (format "  [%s] %-16s analyze=%s pure-core=%s host=%d"
                       (if (= :accepted status) "OK" "REJECT")
                       (name id) (name analysis-status)
                       (pr-str pure-core?) (count host-interop))))
    (shutdown-agents)))
