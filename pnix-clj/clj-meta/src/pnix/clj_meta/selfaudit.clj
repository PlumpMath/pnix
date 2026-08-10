(ns pnix.clj-meta.selfaudit
  "Self-source audit for the Clojure-written compiler.

  This does not claim that compiler.clj fully compiles itself yet. It fixes the
  next honest checkpoint: analyze the real compiler source, count every
  analyzer :op, and classify each op as directly emitted, structurally owned by
  our emitter helpers, intentionally host-maintained, or accidental fallback."
  (:require [clojure.java.io :as io]
            [clojure.edn :as edn]
            [clojure.pprint :as pp]
            [clojure.java.shell :as shell]
            [clojure.tools.analyzer.jvm :as ana]
            [pnix.clj-meta.compiler :as comp]
            [pnix.clj-meta.jarproof :as jarproof])
  (:import [clojure.lang DynamicClassLoader LineNumberingPushbackReader]
           [java.io ByteArrayOutputStream StringReader]
           [java.security MessageDigest]
           [java.util.jar JarEntry JarFile JarOutputStream]))

(def source-path
  ;; Resolve under CLJ_META_ROOT (stage10 sandbox relocates cwd but keeps root).
  ;; Prefer pnix-clj layout (`clj-meta/src/...`); fall back to clj-meta cwd (`src/...`).
  (let [root (or (System/getenv "CLJ_META_ROOT") ".")
        candidates ["clj-meta/src/pnix/clj_meta/compiler.clj"
                    "src/pnix/clj_meta/compiler.clj"]]
    (or (first (filter #(.isFile (io/file root %)) candidates))
        "clj-meta/src/pnix/clj_meta/compiler.clj")))
(def receipt-path "clj-meta/proof/self-source-audit.receipt.edn")
(def stage-receipt-path "clj-meta/proof/self-source-stage-target.receipt.edn")
(def generated-stage-chain-receipt-path
  "clj-meta/proof/self-source-generated-stage-chain.receipt.edn")
(def generated-stage-chain-disk-receipt-path
  "clj-meta/proof/self-source-generated-stage-chain-disk.receipt.edn")
(def stage9-clean-process-receipt-path
  "clj-meta/proof/self-source-stage9-clean-process.receipt.edn")
(def stage10-isolated-matrix-receipt-path
  "clj-meta/proof/self-source-stage10-isolated-matrix.receipt.edn")
(def generated-stage-chain-work-dir
  (or (System/getenv "CLJ_META_STAGE_WORK_DIR")
      "work/self-source-generated-stage-chain"))
(def generated-stage-chain-compare-path
  "work/self-source-generated-stage-chain/compare.txt")
(def stage9-clean-process-child-receipt-path
  "work/self-source-stage9-clean-process/child.receipt.edn")
(def stage9-clean-process-compare-path
  "work/self-source-stage9-clean-process/compare.txt")
(def stage10-isolated-matrix-work-dir
  "work/self-source-stage10-isolated-matrix")
(def stage10-isolated-matrix-compare-path
  "work/self-source-stage10-isolated-matrix/compare.txt")
(def target-ns 'pnix.clj-meta.compiler)
(def ^:private api-host-smoke-ns 'pnix.clj-meta.selfaudit.api-host-smoke)
(def ^:private api-compiled-smoke-ns 'pnix.clj-meta.selfaudit.api-compiled-smoke)
(def ^:private compiled-impl-smoke-ns 'pnix.clj-meta.selfaudit.compiled-impl-smoke)

(def ^:private eof-sentinel (Object.))
(def ^:private project-root
  (or (System/getenv "CLJ_META_ROOT") "."))

(def ^:private direct-emit-ops
  #{:const :local :var :the-var :invoke :prim-invoke :protocol-invoke
    :static-call :if :do :let :loop :recur :vector :map :set :quote
    :throw :try :new :instance-call :static-field :instance-field
    :host-interop :keyword-invoke :instance? :fn :def :with-meta :set!
    :monitor-enter :monitor-exit :case :letfn})

(def ^:private direct-helper-ops
  #{:binding :fn-method :catch :case-test :case-then})

(def ^:private host-maintained-reasons
  {:ns        "namespace side effects are executed by the host loader"
   :import    "namespace/import side effects are executed by the host loader"
   :require   "namespace/require side effects are executed by the host loader"
   :deftype   "new JVM type generation remains host-maintained until full compiler self-source fixed point"
   :reify     "anonymous JVM type generation remains host-maintained until full compiler self-source fixed point"
   :method    "structural child of host-maintained JVM type generation"})

(defn- project-file
  [path]
  (let [f (io/file path)]
    (if (.isAbsolute f)
      f
      (io/file project-root path))))

(defn- absolute-path
  [path]
  (.getCanonicalPath (project-file path)))

(defn- read-source-forms
  [path]
  (let [reader (LineNumberingPushbackReader.
                (StringReader. (slurp (project-file path))))]
    (loop [forms []]
      (let [form (read reader false eof-sentinel)]
        (if (identical? form eof-sentinel)
          forms
          (recur (conj forms form)))))))

(defn- form-head
  [form]
  (cond
    (seq? form) (first form)
    (symbol? form) form
    :else (some-> form class .getName)))

(defn- form-line
  [form]
  (or (:line (meta form)) 0))

(defn- child-nodes
  [node]
  (->> (:children node)
       (mapcat (fn [k]
                 (let [v (get node k)]
                   (cond
                     (vector? v) v
                     (nil? v) []
                     :else [v]))))
       (filter map?)))

(defn- ast-nodes
  [ast]
  (tree-seq map? child-nodes ast))

(defn- analyze-forms
  [forms]
  (require target-ns)
  (let [ns-obj (the-ns target-ns)]
    (binding [*ns* ns-obj
              *file* source-path]
      (doall
       (map-indexed
        (fn [idx form]
          (try
            {:idx idx
             :line (form-line form)
             :head (form-head form)
             :ast (ana/analyze form)}
            (catch Throwable t
              {:idx idx
               :line (form-line form)
               :head (form-head form)
               :error {:class (.getName (class t))
                       :message (.getMessage t)}})))
        forms)))))

(defn- op-census
  [rows]
  (let [ops (map :op (mapcat #(ast-nodes (:ast %)) (remove :error rows)))]
    (into (sorted-map-by #(compare (name %1) (name %2)))
          (frequencies ops))))

(defn- op-policy
  [op]
  (cond
    (contains? direct-emit-ops op)
    {:status :direct-emit
     :reason "emitted by compiler.clj without host Compiler for supported compile targets"}

    (contains? direct-helper-ops op)
    {:status :direct-helper
     :reason "structural analyzer node consumed by an owning direct emitter"}

    (contains? host-maintained-reasons op)
    {:status :host-maintained
     :reason (get host-maintained-reasons op)}

    :else
    {:status :accidental-fallback
     :reason "no ledger entry; add an explicit direct or host-maintained policy before using this in a self-source target"}))

(defn- ledger
  [census]
  (mapv (fn [[op n]]
          (merge {:op op :count n} (op-policy op)))
        census))

(defn- row-ops
  [row]
  (->> (:ast row)
       ast-nodes
       (map :op)
       distinct
       (sort-by name)
       vec))

(defn- top-level-ledger
  [rows]
  (mapv
   (fn [row]
     (if-let [err (:error row)]
       (select-keys row [:idx :line :head :error])
       (let [ops      (row-ops row)
             policies (map op-policy ops)
             host     (vec (keep (fn [op]
                                    (when (= :host-maintained (:status (op-policy op)))
                                      op))
                                  ops))
             unknown  (vec (keep (fn [op]
                                    (when (= :accidental-fallback (:status (op-policy op)))
                                      op))
                                  ops))]
         {:idx (:idx row)
          :line (:line row)
          :head (:head row)
          :root-op (:op (:ast row))
          :ops ops
          :host-maintained-ops host
          :unknown-ops unknown
          :direct-subset? (and (empty? host)
                               (empty? unknown)
                               (every? #(contains? #{:direct-emit :direct-helper}
                                                   (:status %))
                                       policies))})))
   rows))

(defn report
  []
  (let [forms    (read-source-forms source-path)
        rows     (analyze-forms forms)
        census   (op-census rows)
        led      (ledger census)
        top      (top-level-ledger rows)
        errors   (vec (filter :error rows))
        unknown  (vec (map :op (filter #(= :accidental-fallback (:status %)) led)))
        host-ops (vec (map :op (filter #(= :host-maintained (:status %)) led)))
        direct-top (count (filter :direct-subset? top))
        ok       (and (empty? errors) (empty? unknown))]
    {:schema "pnix.clj-meta.self-source-audit.v1"
     :source source-path
     :namespace target-ns
     :top-level-forms (count forms)
     :analyzed-forms (- (count rows) (count errors))
     :analyze-errors (mapv #(dissoc % :ast) errors)
     :op-census census
     :ledger led
     :unknown-ops unknown
     :host-maintained-present host-ops
     :top-level {:direct-subset-forms direct-top
                 :total (count top)
                 :rows top}
     :fixed-point-target-policy
     {:allowed-statuses [:direct-emit :direct-helper]
      :host-maintained-present host-ops
      :direct-subset-only? (empty? host-ops)}
     :ok ok}))

(defn- compile-classes-in-target-ns
  [form]
  (binding [*ns* (the-ns target-ns)
            *file* source-path]
    (comp/compile-classes form)))

(defn- class-fingerprint
  [classes]
  (into (sorted-map)
        (map (fn [[k v]] [k (vec v)]) classes)))

(defn- sha256
  [^bytes bs]
  (let [digest (.digest (MessageDigest/getInstance "SHA-256") bs)]
    (apply str (map #(format "%02x" (bit-and 0xff %)) digest))))

(defn- sha256-string
  [s]
  (sha256 (.getBytes ^String s "UTF-8")))

(defn- class-digests
  [classes]
  (into (sorted-map)
        (map (fn [[k v]] [k (sha256 v)]) classes)))

(defn- class-digest-diff-sample
  [expected actual]
  (let [expected-digests (class-digests expected)
        actual-digests   (class-digests actual)
        names            (sort (distinct (concat (keys expected-digests)
                                                 (keys actual-digests))))]
    (->> names
         (keep (fn [class-name]
                 (let [expected-digest (get expected-digests class-name)
                       actual-digest   (get actual-digests class-name)]
                   (when (not= expected-digest actual-digest)
                     {:class class-name
                      :expected expected-digest
                      :actual actual-digest}))))
         (take 8)
         vec)))

(defn- class-entry-name
  [class-name]
  (str (.replace ^String class-name \. \/) ".class"))

(defn- write-class-jar!
  [classes jar-path]
  (io/make-parents jar-path)
  (with-open [out (JarOutputStream. (io/output-stream jar-path))]
    (doseq [[class-name bytes] (sort-by first classes)]
      (let [entry (JarEntry. (class-entry-name class-name))]
        (.setTime entry 0)
        (.putNextEntry out entry)
        (.write out ^bytes bytes)
        (.closeEntry out))))
  jar-path)

(defn- write-stage-bundle!
  [label classes]
  (let [jar-path (str generated-stage-chain-work-dir "/" label "/classes.jar")]
    (write-class-jar! classes jar-path)
    {:label label
     :jar jar-path
     :classes (count classes)
     :stable-jar-digest (jarproof/stable-jar-digest jar-path)}))

(defn- disk-proof-triples
  [chain]
  (let [host-jar (get-in chain [:disk-bundles :host-reference :jar])
        stages   (get-in chain [:disk-bundles :stages])
        stage8   (get-in chain [:disk-reload :fresh-stage8 :stage8-bundle])
        stage-jars (mapv :jar stages)]
    (vec
     (concat
      [["host-stage1" host-jar (first stage-jars)]]
      (map-indexed
       (fn [i [left right]]
         [(str "stage" (inc i) "-stage" (+ i 2)) left right])
       (partition 2 1 stage-jars))
      [[(str (:label (peek stages)) "-" (:label stage8))
        (peek stage-jars)
        (:jar stage8)]]))))

(defn- disk-proof-digests
  [chain]
  (let [bundles (concat [(get-in chain [:disk-bundles :host-reference])]
                        (get-in chain [:disk-bundles :stages])
                        [(get-in chain [:disk-reload :fresh-stage8 :stage8-bundle])])]
    (mapv #(select-keys % [:label :jar :classes :stable-jar-digest])
          bundles)))

(defn- generated-stage-chain-disk-proof!
  [chain]
  (try
    (if-not (:ok chain)
      {:schema "pnix.clj-meta.self-source-generated-stage-chain-disk-proof.v1"
       :desc "generated stage-chain disk bundle replay proof"
       :skipped :generated-stage-chain-failed
       :ok false}
      (let [triples (disk-proof-triples chain)
            compare-ok? (jarproof/compare-stage! generated-stage-chain-compare-path triples)]
        {:schema "pnix.clj-meta.self-source-generated-stage-chain-disk-proof.v1"
         :desc "generated stage-chain disk bundle replay proof"
         :root generated-stage-chain-work-dir
         :compare-report generated-stage-chain-compare-path
         :pairs (mapv (fn [[label left right]]
                         {:label label :left left :right right})
                       triples)
         :digests (disk-proof-digests chain)
         :disk-fixed-point (:disk-fixed-point chain)
         :disk-matches-host-reference? (:disk-matches-host-reference? chain)
         :disk-reload-ok? (:disk-reload-ok? chain)
         :fresh-stage8-exact? (:fresh-stage8-exact? chain)
         :compare-ok? compare-ok?
         :ok (and compare-ok?
                  (:disk-fixed-point chain)
                  (:disk-matches-host-reference? chain)
                  (:disk-reload-ok? chain)
                  (:fresh-stage8-exact? chain))}))
    (catch Throwable t
      {:schema "pnix.clj-meta.self-source-generated-stage-chain-disk-proof.v1"
       :desc "generated stage-chain disk bundle replay proof"
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- class-name-from-entry
  [entry-name]
  (let [raw (subs entry-name 0 (- (count entry-name) (count ".class")))]
    (.replace ^String raw \/ \.)))

(defn- read-entry-bytes
  [^JarFile jar ^JarEntry entry]
  (with-open [input (.getInputStream jar entry)
              output (ByteArrayOutputStream.)]
    (io/copy input output)
    (.toByteArray output)))

(defn- read-class-jar
  [jar-path]
  (with-open [jar (JarFile. (io/file jar-path))]
    (reduce (fn [acc entry]
              (let [entry ^JarEntry entry
                    entry-name (.getName entry)]
                (if (and (not (.isDirectory entry))
                         (.endsWith ^String entry-name ".class"))
                  (assoc acc
                         (class-name-from-entry entry-name)
                         (read-entry-bytes jar entry))
                  acc)))
            {}
            (enumeration-seq (.entries jar)))))

(defn- compile-row-result
  [forms row]
  (if-not (:direct-subset? row)
    (assoc (select-keys row [:idx :line :head :host-maintained-ops :unknown-ops])
           :stage-target-safe? false
           :reason :not-direct-subset)
    (try
      (let [classes (compile-classes-in-target-ns
                     (list 'fn [] (nth forms (:idx row))))]
        (assoc (select-keys row [:idx :line :head])
               :stage-target-safe? true
               :classes (count classes)))
      (catch Throwable t
        (assoc (select-keys row [:idx :line :head])
               :stage-target-safe? false
               :reason :compile-classes-failed
               :message (.getMessage t))))))

(defn- stage-target-form
  [forms safe-rows]
  (list 'fn []
        (cons 'do
              (conj (mapv #(nth forms (:idx %)) safe-rows)
                    :self-source-stage-target-ok))))

(defn- stage-target-report
  [audit-report stages]
  (let [forms      (vec (read-source-forms source-path))
        rows       (get-in audit-report [:top-level :rows])
        compiled   (mapv #(compile-row-result forms %) rows)
        direct     (filter :direct-subset? rows)
        safe-rows  (vec (filter :stage-target-safe? compiled))
        host-side-effect-rows (vec (filter #(and (not (:stage-target-safe? %))
                                                 (seq (:host-maintained-ops %)))
                                           compiled))
        non-accounted (vec (remove #(or (:stage-target-safe? %)
                                        (seq (:host-maintained-ops %)))
                                   compiled))
        ;; M12: host-side-effect 행(= compiler.clj 의 단일 `ns` form)이 host
        ;; clojure.lang.Compiler 없이 우리 backend 로 컴파일되는지 직접 witness 한다.
        ;; backend 가 compile-classes 로 bytecode 를 내면(0 fallback) 그 form 은
        ;; 'host Compiler fallback' 이 아니라 'backend 컴파일 + host runtime-lib
        ;; namespace side-effect(§13 require/import/in-ns 영구 경계)' 이다.
        ;; deftype/general reify 처럼 backend 가 못 내는 form 만 host-compiler-fallback.
        ns-side-effect-witness
        (mapv (fn [row]
                (let [idx  (:idx row)
                      form (nth forms idx)]
                  (try
                    (let [classes (compile-classes-in-target-ns (list 'fn [] form))]
                      {:idx idx :head (:head row)
                       :host-maintained-ops (:host-maintained-ops row)
                       :backend-compiled? true
                       :host-compiler-fallback? false
                       :classes (count classes)})
                    (catch Throwable t
                      {:idx idx :head (:head row)
                       :host-maintained-ops (:host-maintained-ops row)
                       :backend-compiled? false
                       :host-compiler-fallback? true
                       :message (.getMessage t)}))))
              host-side-effect-rows)
        ns-side-effect-backend-compiled? (every? :backend-compiled? ns-side-effect-witness)
        host-compiler-fallback-rows (vec (filter :host-compiler-fallback? ns-side-effect-witness))
        target     (stage-target-form forms safe-rows)
        classes-by-stage (mapv (fn [_] (compile-classes-in-target-ns target))
                               (range stages))
        fps        (mapv class-fingerprint classes-by-stage)
        ref        (first fps)
        fixed?     (every? #(= ref %) fps)
        accounted  (+ (count safe-rows) (count host-side-effect-rows))
        first-classes (first classes-by-stage)]
    {:schema "pnix.clj-meta.self-source-stage-target.v1"
     :source source-path
     :namespace target-ns
     :full-source-forms (count rows)
     :full-source-accounted-forms accounted
     :full-source-accounted? (= accounted (count rows))
     :host-side-effect-forms (count host-side-effect-rows)
     :host-side-effect-rows (mapv #(select-keys % [:idx :line :head :host-maintained-ops])
                                  host-side-effect-rows)
     ;; M12 witness: namespace side-effect form 들이 backend 로 컴파일되는가(0 host Compiler).
     :ns-side-effect-witness ns-side-effect-witness
     :ns-side-effect-backend-compiled? ns-side-effect-backend-compiled?
     ;; backend 가 못 내서 host clojure.lang.Compiler 가 필요한 form 수(genuine-stage1 의 진짜 게이트).
     :host-compiler-fallback-forms (count host-compiler-fallback-rows)
     :host-compiler-fallback-rows host-compiler-fallback-rows
     :stages stages
     :direct-subset-candidates (count direct)
     :stage-target-safe-forms (count safe-rows)
     :non-accounted-forms (count non-accounted)
     :non-accounted (mapv #(dissoc % :ops) non-accounted)
     :target {:wrapper '(fn [] (do ... :self-source-stage-target-ok))
              :form-indices (mapv :idx safe-rows)}
     :classes (count first-classes)
     :class-names (vec (keys ref))
     :class-digests (class-digests first-classes)
     :fixed-point fixed?
     :ok (and fixed? (= accounted (count rows)) (empty? non-accounted))}))

(defn- proxy-super-smoke
  []
  (let [form '(fn []
                (proxy [clojure.asm.ClassWriter] [0]
                  (getCommonSuperClass [t1 t2]
                    (proxy-super getCommonSuperClass t1 t2))))]
    (try
      (let [proxy-instance (binding [*ns* (the-ns target-ns)
                                     *file* source-path]
                             ((comp/compile-form form)))
            method (.getDeclaredMethod (class proxy-instance)
                                       "getCommonSuperClass"
                                       (into-array Class [String String]))
            _      (.setAccessible method true)
            got    (.invoke method proxy-instance
                            (object-array ["java/lang/String" "java/lang/Object"]))
            ok?    (= "java/lang/Object" got)]
        {:desc "compiled proxy-super dynamic instance-call"
         :method "getCommonSuperClass"
         :want "java/lang/Object"
         :got got
         :proxy-class (.getName (class proxy-instance))
         :ok ok?})
      (catch Throwable t
        {:desc "compiled proxy-super dynamic instance-call"
         :error {:class (.getName (class t))
                 :message (.getMessage t)}
         :ok false}))))

(defn- compiled-api-smoke-source
  [ns-sym]
  (str "(ns " ns-sym "\n"
       "  (:require [pnix.clj-meta.compiler :as c]))\n\n"
       "(def api-value-form-src\n"
       "  \"(fn [n] (let [f (fn [x] (* (+ x n) 2))] (f 1)))\")\n\n"
       "(def api-class-form-src\n"
       "  \"(fn [n] (let [f (fn [x] (+ x n))] (f 5)))\")\n\n"
       "(defn api-result []\n"
       "  ((c/compile-form (read-string api-value-form-src)) 20))\n\n"
       "(defn api-classes []\n"
       "  (c/compile-classes (read-string api-class-form-src)))\n"))

(defn- generated-class?
  [x]
  (.startsWith (.getName (class x)) "pnix.clj_meta.gen."))

(defn- resolved-var-value
  [ns-sym sym]
  (let [v (ns-resolve ns-sym sym)]
    (when-not v
      (throw (ex-info "missing smoke var" {:namespace ns-sym :symbol sym})))
    @v))

(defn- api-smoke-observation
  [ns-sym]
  (let [result-fn  (resolved-var-value ns-sym 'api-result)
        classes-fn (resolved-var-value ns-sym 'api-classes)
        [result classes] (binding [*ns* (the-ns target-ns)
                                   *file* "compiled_api_payload.clj"]
                           [(result-fn) (classes-fn)])]
    {:namespace ns-sym
     :api-result-class (.getName (class result-fn))
     :api-classes-class (.getName (class classes-fn))
     :api-result-generated? (generated-class? result-fn)
     :api-classes-generated? (generated-class? classes-fn)
     :result result
     :class-count (count classes)
     :class-names (vec (keys (class-fingerprint classes)))
     :class-digests (class-digests classes)
     :class-fingerprint (class-fingerprint classes)}))

(defn- public-api-smoke-observation
  [obs]
  (dissoc obs :class-fingerprint))

(defn- load-host-api-smoke
  []
  (let [src (compiled-api-smoke-source api-host-smoke-ns)]
    (remove-ns api-host-smoke-ns)
    (binding [*ns* (the-ns 'pnix.clj-meta.selfaudit)
              *file* "compiled_api_host_smoke.clj"]
      (load-string src))
    (api-smoke-observation api-host-smoke-ns)))

(defn- load-compiled-api-smoke
  []
  (let [src (compiled-api-smoke-source api-compiled-smoke-ns)]
    (remove-ns api-compiled-smoke-ns)
    (let [artifact (binding [*ns* (the-ns 'pnix.clj-meta.selfaudit)]
                     (comp/compile-ns src {:file "compiled_api_artifact_smoke.clj"}))
          loaded   (comp/load-compiled-ns artifact)
          obs      (api-smoke-observation api-compiled-smoke-ns)]
      (assoc obs
             :artifact (select-keys artifact [:namespace :form-count :body-count :loadable])
             :loaded (select-keys loaded [:namespace :loaded])
             ;; thunk 은 이제 load-compiled-ns 가 incremental 로 컴파일하므로 loaded 에서 읽는다.
             :thunk-count (count (:thunks loaded))
             :thunks-generated? (every? generated-class? (:thunks loaded))))))

(defn- compiled-api-smoke
  []
  (try
    (let [host     (load-host-api-smoke)
          compiled (load-compiled-api-smoke)
          same-result? (= (:result host) (:result compiled) 42)
          same-bytecode? (= (:class-fingerprint host)
                            (:class-fingerprint compiled))
          artifact-ok? (and (:thunks-generated? compiled)
                            (:api-result-generated? compiled)
                            (:api-classes-generated? compiled))
          ok?      (and same-result? same-bytecode? artifact-ok?)]
      {:desc "compiled compiler API via isolated compile-ns artifact"
       :host (public-api-smoke-observation host)
       :compiled (public-api-smoke-observation compiled)
       :same-result? same-result?
       :same-bytecode? same-bytecode?
       :artifact-generated? artifact-ok?
       :want 42
       :got (:result compiled)
       :ok ok?})
    (catch Throwable t
      {:desc "compiled compiler API via isolated compile-ns artifact"
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- retarget-ns-form
  [ns-form ns-sym]
  (with-meta
    (apply list 'ns ns-sym (nnext ns-form))
    (meta ns-form)))

(defn- simple-var-symbol
  [sym]
  (when (symbol? sym)
    (symbol (name sym))))

(defn- defn-return-tag
  [form]
  (some (fn [x]
          (when (vector? x)
            (:tag (meta x))))
        (drop 2 form)))

(defn- defn-var-meta
  [head form]
  (cond-> (or (meta (second form)) {})
    (= 'defn- head) (assoc :private true)
    (= 'defmacro head) (assoc :macro true)
    (defn-return-tag form) (assoc :tag (defn-return-tag form))))

(defn- top-level-var-decls
  [form]
  (if-not (seq? form)
    []
    (let [head (first form)]
      (cond
        (= 'declare head)
        (mapv (fn [sym] {:sym sym})
              (keep simple-var-symbol (rest form)))

        (= 'def head)
        (let [raw-sym (second form)]
          (if-let [sym (simple-var-symbol raw-sym)]
            [{:sym sym :meta (meta raw-sym)}]
            []))

        (contains? '#{defn defn- defmacro} head)
        (if-let [sym (simple-var-symbol (second form))]
          [{:sym sym :meta (defn-var-meta head form)}]
          [])

        :else
        []))))

(defn- preintern-var!
  [ns-obj {:keys [sym meta]}]
  (let [v (intern ns-obj sym)]
    (when (seq meta)
      (alter-meta! v merge meta))
    (when (:dynamic meta)
      (.setDynamic ^clojure.lang.Var v))
    v))

(defn- preintern-top-level-vars!
  [ns-obj forms]
  (doseq [decl (mapcat top-level-var-decls forms)]
    (preintern-var! ns-obj decl)))

(defn- prepare-compiled-impl-ns!
  [forms ns-sym]
  (remove-ns ns-sym)
  (binding [*ns* (the-ns 'pnix.clj-meta.selfaudit)
                *file* source-path]
    ;; H5b 해소: ns form 을 host clojure.lang.Compiler(eval) 대신 우리 backend 로 직접
    ;; 컴파일·실행한다(run-ns-form-strict = compile-fn-strict, host Compiler 0회). 이전엔
    ;; 부트스트랩이 ns form 만 host eval 하면서 receipt 는 backend-compiled 라 주장하는
    ;; 불일치가 있었다. 이제 *살아있는 부트스트랩 경로*가 backend-compiled 다.
    (comp/run-ns-form-strict (retarget-ns-form (first forms) ns-sym)))
  (let [ns-obj (the-ns ns-sym)]
    (preintern-top-level-vars! ns-obj (rest forms))
    ns-obj))

(defn- stage-target-form-from-indices
  [forms indices marker]
  (list 'fn []
        (cons 'do
              (conj (mapv #(nth forms %) indices)
                    marker))))

(defn- compiler-api-payload-observation
  [compile-form-fn compile-classes-fn]
  (let [value-form (read-string "(fn [n] (let [x (+ n 1)] (* x 2)))")
        class-form (read-string "(fn [n] (let [f (fn [x] (+ x n))] (f 5)))")
        [result classes] (binding [*ns* (the-ns target-ns)
                                   *file* "compiled_impl_payload.clj"]
                           [((compile-form-fn value-form) 20)
                            (compile-classes-fn class-form)])]
    {:result result
     :class-count (count classes)
     :class-names (vec (keys (class-fingerprint classes)))
     :class-digests (class-digests classes)
     :class-fingerprint (class-fingerprint classes)}))

(defn- load-compiled-compiler-impl!
  [stage]
  (let [forms   (vec (read-source-forms source-path))
        indices (get-in stage [:target :form-indices])
        ns-obj  (prepare-compiled-impl-ns! forms compiled-impl-smoke-ns)
        target  (stage-target-form-from-indices forms indices :compiled-impl-loaded)
        loader  (binding [*ns* ns-obj
                          *file* source-path]
                  (comp/compile-form target))
        got     (binding [*ns* ns-obj
                          *file* source-path]
                  (loader))]
    {:namespace compiled-impl-smoke-ns
     :artifact {:schema "pnix.clj-meta.self-source-full-namespace-artifact.v1"
                :source source-path
                :source-namespace target-ns
                :namespace compiled-impl-smoke-ns
                :host-side-effect-forms 1
                :host-side-effect-head (form-head (first forms))
                :bytecode-forms (count indices)
                :full-source-forms (inc (count indices))
                :marker :compiled-impl-loaded}
     :loader-class (.getName (class loader))
     :loader-generated? (generated-class? loader)
     :top-level-forms (count indices)
     :got got}))

(defn- generated-fn-counter
  [class-name]
  (when-let [s (second (re-find #"^pnix\.clj_meta\.gen\.Fn__(?:[0-9a-f]{12}__)?(\d+)$"
                                class-name))]
    (Long/parseLong s)))

(defn- generated-root-fn-class-name
  [class-names]
  (first (filter #(= 0 (generated-fn-counter %))
                 (sort class-names))))

(defn- instantiate-generated-fn
  [classes]
  (let [dcl     (DynamicClassLoader. (.getContextClassLoader (Thread/currentThread)))
        defined (into {}
                      (map (fn [[cname bytes]]
                             [cname (.defineClass dcl cname bytes nil)]))
                      classes)
        root-name (generated-root-fn-class-name (keys classes))
        root    (get defined root-name)]
    (when-not root
      (throw (ex-info "missing generated root fn class"
                      {:class-names (vec (keys classes))
                       :root-name root-name})))
    {:instance (.newInstance ^Class root)
     :defined-classes defined}))

(defn- compile-target-with
  [compile-classes-fn ns-obj target]
  (binding [*ns* ns-obj
            *file* source-path]
    (compile-classes-fn target)))

(defn- disk-reloaded-stage-bundle-smoke
  [forms target jar-path]
  (try
    (let [ns-obj             (prepare-compiled-impl-ns! forms compiled-impl-smoke-ns)
          classes            (read-class-jar jar-path)
          {:keys [instance defined-classes]} (instantiate-generated-fn classes)
          target-got         (binding [*ns* ns-obj
                                       *file* source-path]
                               (instance))
          compile-form-fn    (resolved-var-value compiled-impl-smoke-ns 'compile-form)
          compile-classes-fn (resolved-var-value compiled-impl-smoke-ns 'compile-classes)
          self-classes       (compile-target-with compile-classes-fn ns-obj target)
          stage8-bundle      (write-stage-bundle! "stage8-from-disk" self-classes)
          stage7-digest      (jarproof/stable-jar-digest jar-path)
          host               (compiler-api-payload-observation
                              comp/compile-form comp/compile-classes)
          compiled           (compiler-api-payload-observation
                              compile-form-fn compile-classes-fn)
          same-result?       (= (:result host) (:result compiled) 42)
          same-bytecode?     (= (:class-fingerprint host)
                                (:class-fingerprint compiled))
          self-source-compiled? (= (count classes) (count self-classes))
          self-source-match? (= (class-digests classes)
                                (class-digests self-classes))
          stage8-digest-match? (= stage7-digest
                                  (:stable-jar-digest stage8-bundle))
          generated?         (and (generated-class? instance)
                                  (generated-class? compile-form-fn)
                                  (generated-class? compile-classes-fn))
          ok?                (and (= :compiled-impl-loaded target-got)
                                  same-result?
                                  same-bytecode?
                                  self-source-compiled?
                                  self-source-match?
                                  stage8-digest-match?
                                  generated?)]
      {:desc "on-disk generated stage bundle reload"
       :jar jar-path
       :classes (count classes)
       :defined-classes (count defined-classes)
       :loader-class (.getName (class instance))
       :loader-generated? (generated-class? instance)
       :api {:compile-form-class (.getName (class compile-form-fn))
             :compile-classes-class (.getName (class compile-classes-fn))
             :compile-form-generated? (generated-class? compile-form-fn)
             :compile-classes-generated? (generated-class? compile-classes-fn)}
       :target-got target-got
       :host (dissoc host :class-fingerprint)
       :compiled (dissoc compiled :class-fingerprint)
       :self-source-recompile {:classes (count self-classes)
                               :compiled? self-source-compiled?
                               :matches-disk-bundle? self-source-match?
                               :diff-sample (class-digest-diff-sample
                                             classes self-classes)
                               :exact-match-required? true}
       :fresh-stage8 {:stage7-jar jar-path
                      :stage7-stable-jar-digest stage7-digest
                      :stage8-bundle stage8-bundle
                      :matches-stage7? stage8-digest-match?}
       :same-result? same-result?
       :same-bytecode? same-bytecode?
       :generated? generated?
       :fresh-stage8-exact? stage8-digest-match?
       :got (:result compiled)
       :ok ok?})
    (catch Throwable t
      {:desc "on-disk generated stage bundle reload"
       :jar jar-path
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- clj-meta-src-path
  "Absolute path to clj-meta source roots. Gate/primary runs from pnix-clj
  root (`clj-meta/src`); some local workflows run with cwd=clj-meta (`src`)."
  []
  (let [candidates ["clj-meta/src" "src"]
        hit (first (filter #(.isDirectory (io/file %)) candidates))]
    (.getCanonicalPath (io/file (or hit "clj-meta/src")))))

(defn- clean-replay-clojure-command
  "Child process for stage9/stage10. Always use -Sdeps (not -M:audit-self-source):
  pnix-clj root has no deps.edn; the alias only lives under clj-meta/deps.edn.
  Paths and deps match the primary gate invocation."
  [stage8-jar receipt-path]
  ["clojure" "-Srepro"
   "-Sdeps" (pr-str {:paths [(clj-meta-src-path)]
                     :deps {'org.clojure/tools.analyzer.jvm
                            {:mvn/version "1.3.4"}}})
   "-M" "-m" "pnix.clj-meta.selfaudit"
   "clean-replay"
   (str stage8-jar)
   (str receipt-path)])

(defn- clean-process-environment
  []
  (into (sorted-map)
        {:java-version (System/getProperty "java.version")
         :java-vendor (System/getProperty "java.vendor")
         :java-home (System/getProperty "java.home")
         :clojure-version (clojure-version)
         :user-dir (System/getProperty "user.dir")
         :timezone (System/getProperty "user.timezone")
         :locale (str (java.util.Locale/getDefault))
         :pid (.pid (java.lang.ProcessHandle/current))}))

(defn- stage9-canonical-observation
  [target-got payload stage9-bundle generated?]
  [[:schema "pnix.clj-meta.stage9.clean-process.canonical.v1"]
   [:target-got target-got]
   [:payload-result (:result payload)]
   [:payload-class-digests (:class-digests payload)]
   [:stage9-classes (:classes stage9-bundle)]
   [:artifact-equivalence :jarproof-normalized-compare]
   [:generated? generated?]])

(defn- stage9-clean-process-replay-report
  [stage8-jar]
  (try
    (let [base               (report)
          stage              (stage-target-report base 7)
          forms              (vec (read-source-forms source-path))
          target             (stage-target-form-from-indices
                              forms (get-in stage [:target :form-indices])
                              :compiled-impl-loaded)
          ns-obj             (prepare-compiled-impl-ns! forms compiled-impl-smoke-ns)
          classes            (read-class-jar stage8-jar)
          stage8-digest      (jarproof/stable-jar-digest stage8-jar)
          {:keys [instance defined-classes]} (instantiate-generated-fn classes)
          target-got         (binding [*ns* ns-obj
                                       *file* source-path]
                               (instance))
          compile-form-fn    (resolved-var-value compiled-impl-smoke-ns 'compile-form)
          compile-classes-fn (resolved-var-value compiled-impl-smoke-ns 'compile-classes)
          self-classes       (compile-target-with compile-classes-fn ns-obj target)
          stage9-bundle      (write-stage-bundle! "stage9-clean-process" self-classes)
          host               (compiler-api-payload-observation
                              comp/compile-form comp/compile-classes)
          compiled           (compiler-api-payload-observation
                              compile-form-fn compile-classes-fn)
          same-result?       (= (:result host) (:result compiled) 42)
          same-bytecode?     (= (:class-fingerprint host)
                                (:class-fingerprint compiled))
          self-source-match? (= (class-digests classes)
                                (class-digests self-classes))
          self-source-compiled? (= (count classes) (count self-classes))
          artifact-match?    (= stage8-digest (:stable-jar-digest stage9-bundle))
          generated?         (and (generated-class? instance)
                                  (generated-class? compile-form-fn)
                                  (generated-class? compile-classes-fn))
          canonical          (stage9-canonical-observation
                              target-got compiled stage9-bundle generated?)
          canonical-digest   (sha256-string (pr-str canonical))
          ok?                (and (:ok base)
                                  (:ok stage)
                                  (= :compiled-impl-loaded target-got)
                                  same-result?
                                  same-bytecode?
                                  self-source-compiled?
                                  generated?)]
      {:schema "pnix.clj-meta.stage9.clean-process-replay.v1"
       :desc "stage9 clean process compiler-runtime replay"
       :stage 9
       :input-artifact {:jar stage8-jar
                        :stable-jar-digest stage8-digest
                        :classes (count classes)}
       :environment (clean-process-environment)
       :loader {:class (.getName (class instance))
                :generated? (generated-class? instance)
                :defined-classes (count defined-classes)
                :target-got target-got}
       :api {:compile-form-class (.getName (class compile-form-fn))
             :compile-classes-class (.getName (class compile-classes-fn))
             :compile-form-generated? (generated-class? compile-form-fn)
             :compile-classes-generated? (generated-class? compile-classes-fn)}
       :payload {:host (dissoc host :class-fingerprint)
                 :compiled (dissoc compiled :class-fingerprint)
                 :same-result? same-result?
                 :same-bytecode? same-bytecode?}
       :self-source-recompile {:classes (count self-classes)
                               :compiled? self-source-compiled?
                               :matches-input-artifact? self-source-match?
                               :exact-match-required? false
                               :diff-sample (class-digest-diff-sample
                                             classes self-classes)}
       :stage9-artifact stage9-bundle
       :artifact-match? artifact-match?
       :canonical-receipt canonical
       :canonical-receipt-digest canonical-digest
       :ok ok?})
    (catch Throwable t
      {:schema "pnix.clj-meta.stage9.clean-process-replay.v1"
       :desc "stage9 clean process compiler-runtime replay"
       :stage 9
       :input-artifact {:jar stage8-jar}
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- write-stage9-child-receipt!
  [receipt-path r]
  (io/make-parents receipt-path)
  (spit receipt-path (with-out-str (pp/pprint r)))
  r)

(defn- stage9-clean-process-replay-cli!
  [stage8-jar receipt-path]
  (let [r (write-stage9-child-receipt!
           receipt-path
           (stage9-clean-process-replay-report stage8-jar))]
    (println (str "stage9 clean process replay: "
                  (if (:ok r) "OK" "FAILED")
                  " receipt=" receipt-path))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))

(defn- read-edn-file
  [path]
  (edn/read-string (slurp path)))

(defn- stage9-clean-process-proof!
  [disk-proof]
  (try
    (if-not (:ok disk-proof)
      {:schema "pnix.clj-meta.stage9.clean-process-proof.v1"
       :desc "stage9 clean process compiler-runtime replay"
       :skipped :stage8-disk-proof-failed
       :ok false}
      (let [stage8-bundle (peek (:digests disk-proof))
            stage8-jar    (:jar stage8-bundle)
            _             (io/delete-file stage9-clean-process-child-receipt-path true)
            child-cmd     (clean-replay-clojure-command
                           stage8-jar
                           stage9-clean-process-child-receipt-path)
            child         (apply shell/sh (concat child-cmd [:dir "."]))
            child-exit    (:exit child)
            child-receipt (when (.exists (io/file stage9-clean-process-child-receipt-path))
                            (read-edn-file stage9-clean-process-child-receipt-path))
            stage9-jar    (get-in child-receipt [:stage9-artifact :jar])
            compare-ok?   (and (zero? child-exit)
                               (:ok child-receipt)
                               (jarproof/compare-stage!
                                stage9-clean-process-compare-path
                                [["stage8-stage9-clean-process"
                                  stage8-jar
                                  stage9-jar]]))
            artifact-match? (= (:stable-jar-digest stage8-bundle)
                               (get-in child-receipt
                                       [:stage9-artifact :stable-jar-digest]))
            ok?          (and compare-ok?
                              (:ok child-receipt))]
        {:schema "pnix.clj-meta.stage9.clean-process-proof.v1"
         :desc "stage9 clean process compiler-runtime replay"
         :stage 9
         :child-command child-cmd
         :child-exit child-exit
         :child-receipt stage9-clean-process-child-receipt-path
         :child-stdout (:out child)
         :child-stderr (:err child)
         :compare-report stage9-clean-process-compare-path
         :compare-ok? compare-ok?
         :stage8-artifact stage8-bundle
         :stage9-artifact (:stage9-artifact child-receipt)
         :artifact-match? artifact-match?
         :canonical-receipt-digest (:canonical-receipt-digest child-receipt)
         :payload-result (get-in child-receipt [:payload :compiled :result])
         :payload-same-bytecode? (get-in child-receipt [:payload :same-bytecode?])
         :target-got (get-in child-receipt [:loader :target-got])
         :environment (:environment child-receipt)
         :ok ok?}))
    (catch Throwable t
      {:schema "pnix.clj-meta.stage9.clean-process-proof.v1"
       :desc "stage9 clean process compiler-runtime replay"
       :stage 9
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- process-env
  [extra]
  (merge (into {} (System/getenv))
         extra))

(defn- stage10-matrix-variants
  []
  [{:label "baseline"
    :cwd :project-root
    :classpath-policy :project-deps
    :env {}}
   {:label "utc-c-locale"
    :cwd :project-root
    :classpath-policy :project-deps
    :env {"TZ" "UTC"
          "LANG" "C"
          "LC_ALL" "C"}}
   {:label "seoul-ko-locale"
    :cwd :project-root
    :classpath-policy :project-deps
    :env {"TZ" "Asia/Seoul"
          "LANG" "ko_KR.UTF-8"
          "LC_ALL" "ko_KR.UTF-8"}}
   {:label "sandbox-cwd"
    :cwd :sandbox
    :classpath-policy :absolute-sdeps
    :env {"TZ" "UTC"
          "LANG" "C"
          "LC_ALL" "C"}}])

(defn- stage10-child-cwd!
  [root variant-dir variant]
  (case (:cwd variant)
    :project-root root
    :sandbox (let [cwd (str variant-dir "/cwd")]
               (.mkdirs (io/file cwd))
               cwd)))

(defn- stage10-child-command
  "Same clean-replay entry for every matrix row. Classpath always pins
  clj-meta/src via -Sdeps (project-deps and absolute-sdeps converge: the
  former used to require a root deps.edn alias that does not exist)."
  [root stage8-jar receipt variant]
  (let [_ root _ variant]
    (clean-replay-clojure-command stage8-jar receipt)))

(defn- stage10-canonical-row
  [row]
  (select-keys row
               [:label
                :cwd-kind
                :classpath-policy
                :canonical-receipt-digest
                :canonical-matches-baseline?
                :artifact-digest
                :artifact-matches-baseline?
                :artifact-match?
                :payload-result
                :payload-same-bytecode?
                :target-got
                :ok]))

(defn- stage10-child-row
  [stage8-jar expected-canonical expected-artifact variant]
  (let [root       (absolute-path ".")
        label      (:label variant)
        variant-dir (absolute-path (str stage10-isolated-matrix-work-dir "/" label))
        receipt    (str variant-dir "/child.receipt.edn")
        stage-work (str variant-dir "/stage-work")
        cwd        (stage10-child-cwd! root variant-dir variant)
        command    (stage10-child-command root stage8-jar receipt variant)
        env        (process-env
                    (merge {"CLJ_META_ROOT" root
                            "CLJ_META_STAGE_WORK_DIR" stage-work}
                           (:env variant)))
        _          (io/make-parents receipt)
        _          (io/delete-file receipt true)
        child      (apply shell/sh (concat command [:dir cwd :env env]))
        child-receipt (when (.exists (io/file receipt))
                        (read-edn-file receipt))
        stage10-jar (get-in child-receipt [:stage9-artifact :jar])
        canonical   (:canonical-receipt-digest child-receipt)
        artifact    (get-in child-receipt [:stage9-artifact :stable-jar-digest])
        row-ok?     (and (zero? (:exit child))
                         (:ok child-receipt)
                         (= expected-canonical canonical)
                         (get-in child-receipt [:payload :same-bytecode?])
                         (= :compiled-impl-loaded
                            (get-in child-receipt [:loader :target-got])))]
    {:label label
     :cwd-kind (:cwd variant)
     :cwd cwd
     :classpath-policy (:classpath-policy variant)
     :child-command command
     :child-exit (:exit child)
     :child-receipt receipt
     :stage-work-dir stage-work
     :root-binding root
     :stage10-jar stage10-jar
     :env (into (sorted-map) (:env variant))
     :environment (:environment child-receipt)
     :canonical-receipt-digest canonical
     :canonical-matches-baseline? (= expected-canonical canonical)
     :artifact-digest artifact
     :artifact-matches-baseline? (= expected-artifact artifact)
     :artifact-match? (:artifact-match? child-receipt)
     :artifact-equivalence :jarproof-normalized-compare
     :payload-result (get-in child-receipt [:payload :compiled :result])
     :payload-same-bytecode? (get-in child-receipt [:payload :same-bytecode?])
     :target-got (get-in child-receipt [:loader :target-got])
     :stdout (:out child)
     :stderr (:err child)
     :ok row-ok?}))

(defn- stage10-isolated-matrix-proof!
  [stage9-proof]
  (try
    (if-not (:ok stage9-proof)
      {:schema "pnix.clj-meta.stage10.isolated-matrix-proof.v1"
       :desc "stage10 isolated classpath/session/sandbox compiler closure"
       :skipped :stage9-clean-process-failed
       :ok false}
      (let [stage8-jar        (absolute-path (get-in stage9-proof [:stage8-artifact :jar]))
            expected-canonical (:canonical-receipt-digest stage9-proof)
            expected-artifact  (get-in stage9-proof [:stage8-artifact :stable-jar-digest])
            variants          (stage10-matrix-variants)
            rows              (mapv #(stage10-child-row stage8-jar
                                                        expected-canonical
                                                        expected-artifact
                                                        %)
                                    variants)
            triples           (mapv (fn [{:keys [label stage10-jar]}]
                                      [(str "stage8-" label) stage8-jar stage10-jar])
                                    rows)
            compare-ok?       (and (every? :ok rows)
                                   (jarproof/compare-stage!
                                    stage10-isolated-matrix-compare-path
                                    triples))
            canonical-fixed?  (every? #(= expected-canonical
                                          (:canonical-receipt-digest %))
                                      rows)
            artifact-fixed?   (boolean compare-ok?)
            cwd-relocated?    (boolean (some #(and (= :sandbox (:cwd-kind %))
                                                   (:ok %)
                                                   (not= (:root-binding %) (:cwd %)))
                                             rows))
            invariant         (sorted-map
                               :artifact-fixed artifact-fixed?
                               :canonical-fixed canonical-fixed?
                               :compare-ok compare-ok?
                               :cwd-relocated cwd-relocated?
                               :rows-ok (every? :ok rows))
            canonical         (mapv stage10-canonical-row rows)
            canonical-digest  (sha256-string (pr-str canonical))
            ok?               (and compare-ok?
                                   canonical-fixed?
                                   cwd-relocated?
                                   (every? :ok rows))]
        {:schema "pnix.clj-meta.stage10.isolated-matrix-proof.v1"
         :desc "stage10 isolated classpath/session/sandbox/cwd compiler closure"
         :stage 10
         :matrix [:baseline :timezone :locale :work-dir :fresh-jvm
                  :clean-namespace :cwd-relocation]
         :cwd-policy :root-bound-cwd-independent
         :root (absolute-path ".")
         :compare-report stage10-isolated-matrix-compare-path
         :expected-canonical-receipt-digest expected-canonical
         :expected-artifact-digest expected-artifact
         :rows rows
         :invariants invariant
         :canonical-receipt canonical
         :canonical-receipt-digest canonical-digest
         :compare-ok? compare-ok?
         :canonical-fixed? canonical-fixed?
         :artifact-fixed? artifact-fixed?
         :cwd-relocated? cwd-relocated?
         :ok ok?}))
    (catch Throwable t
      {:schema "pnix.clj-meta.stage10.isolated-matrix-proof.v1"
       :desc "stage10 isolated classpath/session/sandbox/cwd compiler closure"
       :stage 10
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- load-generated-target!
  [ns-obj classes]
  (let [{:keys [instance defined-classes]} (instantiate-generated-fn classes)
        got (binding [*ns* ns-obj
                      *file* source-path]
              (instance))]
    {:loader-class (.getName (class instance))
     :loader-generated? (generated-class? instance)
     :defined-classes (count defined-classes)
     :got got}))

(defn- generated-stage-chain-smoke
  [stage]
  (try
    (let [bootstrap (load-compiled-compiler-impl! stage)
          ns-obj    (the-ns compiled-impl-smoke-ns)
          forms     (vec (read-source-forms source-path))
          target    (stage-target-form-from-indices
                     forms (get-in stage [:target :form-indices]) :compiled-impl-loaded)
          receipt-target (stage-target-form-from-indices
                          forms (get-in stage [:target :form-indices])
                          :self-source-stage-target-ok)
          receipt-driver (resolved-var-value compiled-impl-smoke-ns 'compile-classes)
          receipt-classes (compile-target-with receipt-driver (the-ns target-ns) receipt-target)
          receipt-loaded (load-generated-target! (the-ns target-ns) receipt-classes)
          receipt-match? (and (= (count receipt-classes) (:classes stage))
                              (= :self-source-stage-target-ok (:got receipt-loaded))
                              (:loader-generated? receipt-loaded))
          host-classes (compile-target-with comp/compile-classes ns-obj target)
          host-fp      (class-fingerprint host-classes)
          host-bundle  (write-stage-bundle! "host-reference" host-classes)
          rows      (loop [i 1
                           acc []]
                      (if (> i 7)
                        acc
                        (let [driver  (resolved-var-value compiled-impl-smoke-ns 'compile-classes)
                              classes (compile-target-with driver ns-obj target)
                              bundle  (write-stage-bundle! (str "stage" i) classes)
                              loaded  (load-generated-target! ns-obj classes)]
                          (recur (inc i)
                                 (conj acc
                                       {:stage i
                                        :driver-class (.getName (class driver))
                                        :driver-generated? (generated-class? driver)
                                        :classes (count classes)
                                        :class-digests (class-digests classes)
                                        :class-fingerprint (class-fingerprint classes)
                                        :disk-bundle bundle
                                        :loaded loaded})))))]
      (let [fps         (mapv :class-fingerprint rows)
            fixed?      (every? #(= (first fps) %) fps)
            host-match? (every? #(= host-fp %) fps)
            bundle-digests (mapv #(get-in % [:disk-bundle :stable-jar-digest]) rows)
            disk-fixed? (every? #(= (first bundle-digests) %) bundle-digests)
            disk-host-match? (every? #(= (:stable-jar-digest host-bundle) %)
                                     bundle-digests)
            disk-reload (disk-reloaded-stage-bundle-smoke
                         forms target (get-in (peek rows) [:disk-bundle :jar]))
            loaded-ok?  (every? #(= :compiled-impl-loaded (get-in % [:loaded :got])) rows)
            generated?  (and (:loader-generated? bootstrap)
                             (every? :driver-generated? rows)
                             (every? #(get-in % [:loaded :loader-generated?]) rows))
            public-rows (mapv #(-> %
                                   (dissoc :class-fingerprint)
                                   (update :loaded dissoc :defined-classes))
                               rows)
            ok?         (and fixed?
                              host-match?
                              disk-fixed?
                              disk-host-match?
                              (:ok disk-reload)
                              receipt-match?
                              loaded-ok?
                              generated?)]
        {:desc "generated compiler var stage1..7 self-source driver"
         :bootstrap (select-keys bootstrap
                                 [:namespace :loader-class :loader-generated?
                                  :top-level-forms :got])
         :stages 7
         :classes (count host-classes)
         :stage-receipt-match? receipt-match?
         :stage-receipt-classes (count receipt-classes)
         :stage-receipt-loaded (dissoc receipt-loaded :defined-classes)
         :host-reference-digests (class-digests host-classes)
         :disk-bundles {:root generated-stage-chain-work-dir
                        :host-reference host-bundle
                        :stages (mapv #(select-keys (:disk-bundle %)
                                                     [:label
                                                      :jar
                                                      :classes
                                                      :stable-jar-digest])
                                      rows)
                        :fixed-point disk-fixed?
                        :matches-host-reference? disk-host-match?
                        :jarproof-compatible true}
         :disk-reload disk-reload
         :rows public-rows
         :fixed-point fixed?
         :matches-host-reference? host-match?
         :disk-fixed-point disk-fixed?
         :disk-matches-host-reference? disk-host-match?
         :disk-reload-ok? (:ok disk-reload)
         :fresh-stage8-exact? (:fresh-stage8-exact? disk-reload)
         :loaded-ok? loaded-ok?
         :generated? generated?
         :got (if ok? :stage1..7-fixed-point :failed)
         :ok ok?}))
    (catch Throwable t
      {:desc "generated compiler var stage1..7 self-source driver"
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- compiled-full-namespace-artifact-smoke
  [stage]
  (try
    (let [impl               (load-compiled-compiler-impl! stage)
          compile-form-fn    (resolved-var-value compiled-impl-smoke-ns 'compile-form)
          compile-classes-fn (resolved-var-value compiled-impl-smoke-ns 'compile-classes)
          host               (compiler-api-payload-observation
                              comp/compile-form comp/compile-classes)
          compiled           (compiler-api-payload-observation
                              compile-form-fn compile-classes-fn)
          artifact           (:artifact impl)
          artifact-ok?       (and (= (:full-source-forms stage)
                                     (:full-source-forms artifact))
                                  (= (:host-side-effect-forms stage)
                                     (:host-side-effect-forms artifact))
                                  (= (:stage-target-safe-forms stage)
                                     (:bytecode-forms artifact))
                                  (= :compiled-impl-loaded (:got impl)))
          same-result?       (= (:result host) (:result compiled) 42)
          same-bytecode?     (= (:class-fingerprint host)
                                (:class-fingerprint compiled))
          generated?         (and (:loader-generated? impl)
                                  (generated-class? compile-form-fn)
                                  (generated-class? compile-classes-fn))
          ok?                (and artifact-ok? same-result? same-bytecode? generated?)]
      {:desc "compiled full namespace artifact/load"
       :artifact artifact
       :loader (select-keys impl [:loader-class :loader-generated? :got])
       :api {:compile-form-class (.getName (class compile-form-fn))
             :compile-classes-class (.getName (class compile-classes-fn))
             :compile-form-generated? (generated-class? compile-form-fn)
             :compile-classes-generated? (generated-class? compile-classes-fn)}
       :host (dissoc host :class-fingerprint)
       :compiled (dissoc compiled :class-fingerprint)
       :same-result? same-result?
       :same-bytecode? same-bytecode?
       :generated? generated?
       :got (:result compiled)
       :ok ok?})
    (catch Throwable t
      {:desc "compiled full namespace artifact/load"
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn- compiled-compiler-impl-smoke
  [stage]
  (try
    (let [impl               (load-compiled-compiler-impl! stage)
          compile-form-fn    (resolved-var-value compiled-impl-smoke-ns 'compile-form)
          compile-classes-fn (resolved-var-value compiled-impl-smoke-ns 'compile-classes)
          host               (compiler-api-payload-observation
                              comp/compile-form comp/compile-classes)
          compiled           (compiler-api-payload-observation
                              compile-form-fn compile-classes-fn)
          same-result?       (= (:result host) (:result compiled) 42)
          same-bytecode?     (= (:class-fingerprint host)
                                (:class-fingerprint compiled))
          generated-vars?    (and (:loader-generated? impl)
                                  (generated-class? compile-form-fn)
                                  (generated-class? compile-classes-fn))
          ok?                (and same-result?
                                  same-bytecode?
                                  generated-vars?
                                  (= :compiled-impl-loaded (:got impl)))]
      {:desc "compiled self-source compiler implementation vars"
       :impl (assoc impl
                    :compile-form-class (.getName (class compile-form-fn))
                    :compile-classes-class (.getName (class compile-classes-fn))
                    :compile-form-generated? (generated-class? compile-form-fn)
                    :compile-classes-generated? (generated-class? compile-classes-fn))
       :host (dissoc host :class-fingerprint)
       :compiled (dissoc compiled :class-fingerprint)
       :same-result? same-result?
       :same-bytecode? same-bytecode?
       :generated-vars? generated-vars?
       :want 42
       :got (:result compiled)
       :ok ok?})
    (catch Throwable t
      {:desc "compiled self-source compiler implementation vars"
       :error {:class (.getName (class t))
               :message (.getMessage t)}
       :ok false})))

(defn write-receipt!
  [r]
  (io/make-parents receipt-path)
  (spit receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stage-receipt!
  [r]
  (io/make-parents stage-receipt-path)
  (spit stage-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-generated-stage-chain-receipt!
  [r]
  (io/make-parents generated-stage-chain-receipt-path)
  (spit generated-stage-chain-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-generated-stage-chain-disk-receipt!
  [r]
  (io/make-parents generated-stage-chain-disk-receipt-path)
  (spit generated-stage-chain-disk-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stage9-clean-process-receipt!
  [r]
  (io/make-parents stage9-clean-process-receipt-path)
  (spit stage9-clean-process-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn write-stage10-isolated-matrix-receipt!
  [r]
  (io/make-parents stage10-isolated-matrix-receipt-path)
  (spit stage10-isolated-matrix-receipt-path (with-out-str (pp/pprint r)))
  r)

(defn run
  []
  (let [base  (report)
        stage (stage-target-report base 7)
        proxy (proxy-super-smoke)
        api   (compiled-api-smoke)
        impl  (compiled-compiler-impl-smoke stage)
        chain (generated-stage-chain-smoke stage)
        disk  (generated-stage-chain-disk-proof! chain)
        stage9 (stage9-clean-process-proof! disk)
        stage10 (stage10-isolated-matrix-proof! stage9)
        full  (compiled-full-namespace-artifact-smoke stage)]
    (-> base
        (assoc :bytecode-stage-target stage)
        (assoc :generated-stage-chain chain)
        (assoc :generated-stage-chain-disk-proof disk)
        (assoc :stage9-clean-process stage9)
        (assoc :stage10-isolated-matrix stage10)
        (assoc :compiled-execution-smokes [proxy api impl chain full])
        (assoc :ok (and (:ok base)
                        (:ok stage)
                        (:ok disk)
                        (:ok stage9)
                        (:ok stage10)
                        (every? :ok [proxy api impl chain full]))))))

(defn -main
  [& args]
  (if (= "clean-replay" (first args))
    (let [[_ stage8-jar receipt-path] args]
      (stage9-clean-process-replay-cli! stage8-jar receipt-path))
    (let [r      (write-receipt! (run))
          stage  (write-stage-receipt! (:bytecode-stage-target r))
          chain  (write-generated-stage-chain-receipt! (:generated-stage-chain r))
          disk   (write-generated-stage-chain-disk-receipt!
                  (:generated-stage-chain-disk-proof r))
          stage9 (write-stage9-clean-process-receipt!
                  (:stage9-clean-process r))
          stage10 (write-stage10-isolated-matrix-receipt!
                   (:stage10-isolated-matrix r))]
    (println (str "self-source audit: " (if (:ok r) "OK" "FAILED")))
    (println (str "  source=" (:source r)
                  " forms=" (:top-level-forms r)
                  " analyzed=" (:analyzed-forms r)
                  " ops=" (count (:op-census r))))
    (println (str "  unknown-ops=" (pr-str (:unknown-ops r))))
    (println (str "  host-maintained-present=" (pr-str (:host-maintained-present r))))
    (println (str "  direct-subset top-level="
                  (get-in r [:top-level :direct-subset-forms])
                  "/" (get-in r [:top-level :total])))
    (println (str "  bytecode stage target="
                  (:stage-target-safe-forms stage)
                  "/" (:direct-subset-candidates stage)
                  " forms, classes=" (:classes stage)
                  ", stage1..7 fixed-point="
                  (if (:fixed-point stage) "OK" "FAILED")))
    (println (str "  full-source accounted="
                  (:full-source-accounted-forms stage)
                  "/" (:full-source-forms stage)
                  ", host-side-effect-forms="
                  (:host-side-effect-forms stage)))
    (doseq [{:keys [desc ok got error]} (:compiled-execution-smokes r)]
      (println (str "  execution smoke " (if ok "OK" "FAILED")
                    ": " desc
                    (if error
                      (str " err=" (pr-str error))
                      (str " got=" (pr-str got))))))
    (println (str "  receipt: " receipt-path))
    (println (str "  stage receipt: " stage-receipt-path))
    (println (str "  generated stage-chain receipt: "
                  generated-stage-chain-receipt-path
                  " fixed-point=" (if (:fixed-point chain) "OK" "FAILED")
                  ", host-match=" (if (:matches-host-reference? chain) "OK" "FAILED")
                  ", disk-fixed-point=" (if (:disk-fixed-point chain) "OK" "FAILED")
                  ", disk-host-match=" (if (:disk-matches-host-reference? chain)
                                         "OK"
                                         "FAILED")
                  ", disk-reload=" (if (:disk-reload-ok? chain) "OK" "FAILED")
                  ", fresh-stage8-exact=" (if (:fresh-stage8-exact? chain)
                                            "OK"
                                            "FAILED")))
    (println (str "  generated stage-chain disk proof: "
                  generated-stage-chain-disk-receipt-path
                  " compare-report=" generated-stage-chain-compare-path
                  " ok=" (if (:ok disk) "OK" "FAILED")))
    (println (str "  stage9 clean-process proof: "
                  stage9-clean-process-receipt-path
                  " compare-report=" stage9-clean-process-compare-path
                  " ok=" (if (:ok stage9) "OK" "FAILED")
                  ", canonical-digest="
                  (:canonical-receipt-digest stage9)))
    (println (str "  stage10 isolated-matrix proof: "
                  stage10-isolated-matrix-receipt-path
                  " compare-report=" stage10-isolated-matrix-compare-path
                  " ok=" (if (:ok stage10) "OK" "FAILED")
                  ", variants=" (count (:rows stage10))))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1)))))
