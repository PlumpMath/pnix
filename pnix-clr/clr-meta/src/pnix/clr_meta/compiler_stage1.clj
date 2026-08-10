(ns pnix.clr-meta.compiler-stage1
  "Compiler-implementation-source-hidden Stage1 backend.

  The bootstrap compiler may AOT-seed this namespace and its pure lowering
  core once.  In target mode this code reads the closed profile/plan/source,
  lowers forms itself, and writes a managed executable directly through BCL
  metadata/Reflection.Emit APIs.  It never delegates a target form to the
  ClojureCLR Compiler or evaluator."
  (:require [clojure.edn :as edn]
            [clojure.string :as str]
            [pnix.clr-meta.compiler-stage1-core :as core]))

;; ClojureCLR resolves CLR type names against loaded assemblies while compiling
;; subsequent top-level forms.  This is a low-level metadata writer, not a
;; language compiler or semantic fallback.
(System.Reflection.Assembly/Load "System.Reflection.Metadata")

(def plan-schema :pnix.clr-meta.compiler-stage1-plan.v1)
(def target-manifest-schema "pnix.clr-meta.compiler-stage1-target.v1")
(def target-framework "net10.0")
(def target-assembly "ClrMeta.Stage1.Program")
(def target-type "ClrMeta.Stage1.Program")
(def target-method "Run")
(def target-entry "Main")
(def fallback-counter-keys
  #{"compile" "eval" "load" "load_file" "load_string"})

(def expected-plan
  {:schema plan-schema
   :profile core/profile-id
   :assembly target-assembly
   :type target-type
   :method target-method})

(def ^:private utf8 (System.Text.UTF8Encoding. false true))

(defn- fail!
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.compiler-stage1-error.v1
                          :phase :compiler-stage1-backend
                          :class class}
                         evidence))))

(defn- reparse-point?
  [path]
  (not (zero? (bit-and (int (System.IO.File/GetAttributes path))
                       (int System.IO.FileAttributes/ReparsePoint)))))

(defn- reparse-ancestor
  [path]
  (loop [candidate (System.IO.Path/GetFullPath (str path))]
    (when candidate
      (if (and (or (System.IO.File/Exists candidate)
                   (System.IO.Directory/Exists candidate))
               (reparse-point? candidate))
        candidate
        (recur (System.IO.Path/GetDirectoryName candidate))))))

(defn- require-regular-file!
  [path kind]
  (let [path (System.IO.Path/GetFullPath (str path))]
    (when-not (System.IO.File/Exists path)
      (fail! (keyword (str "missing-" (name kind)))
             "Stage1 input file is unavailable"
             {:kind kind :path path}))
    (when (reparse-point? path)
      (fail! (keyword (str (name kind) "-symlink"))
             "Stage1 input itself must not be a symlink/reparse point"
             {:kind kind :path path}))
    path))

(defn- validate-reader-depth!
  [source]
  (loop [index 0 depth 0 in-string false escaped false in-comment false]
    (when (< index (.Length source))
      (let [character (.get_Chars source index)]
        (cond
          in-comment
          (recur (inc index) depth in-string false
                 (not (contains? #{\newline \return} character)))

          in-string
          (cond
            escaped (recur (inc index) depth true false false)
            (= character \\) (recur (inc index) depth true true false)
            (= character \") (recur (inc index) depth false false false)
            :else (recur (inc index) depth true false false))

          (= character \;)
          (recur (inc index) depth false false true)

          (= character \")
          (recur (inc index) depth true false false)

          (= character \\)
          ;; Skip the first character of an EDN character literal so `\[` is
          ;; not mistaken for structural nesting. Such literals are rejected
          ;; later by the admitted value-domain check.
          (recur (min (.Length source) (+ index 2)) depth false false false)

          (contains? #{\( \[ \{} character)
          (let [next-depth (inc depth)]
            (when (> next-depth core/max-reader-depth)
              (fail! :reader-depth-budget-exceeded
                     "Stage1 source exceeds the pre-reader nesting budget"
                     {:max-reader-depth core/max-reader-depth}))
            (recur (inc index) next-depth false false false))

          (contains? #{\) \] \}} character)
          (recur (inc index) (max 0 (dec depth)) false false false)

          :else
          (recur (inc index) depth false false false))))))

(defn- read-bounded-text
  [path kind]
  (let [bytes (System.IO.File/ReadAllBytes path)
        limit (if (= kind :source) core/max-source-bytes 16384)]
    (when (> (alength bytes) limit)
      (fail! (keyword (str (name kind) "-byte-budget-exceeded"))
             "Stage1 input exceeds its pre-reader byte budget"
             {:kind kind :max-bytes limit :actual-bytes (alength bytes)}))
    (try
      (let [source (.GetString utf8 bytes)]
        (validate-reader-depth! source)
        source)
      (catch System.Text.DecoderFallbackException cause
        (fail! (keyword (str "invalid-" (name kind) "-utf8"))
               "Stage1 input is not strict UTF-8"
               {:kind kind :cause-type (str (type cause))})))))

(defn- strict-read-edn
  [path kind]
  (let [path (require-regular-file! path kind)
        source (read-bounded-text path kind)
        eof (Object.)]
    (try
      (with-open [reader (clojure.lang.LineNumberingTextReader.
                          (System.IO.StringReader. source))]
        (let [value (edn/read {:eof eof} reader)
              trailing (edn/read {:eof eof} reader)]
          (when (identical? value eof)
            (fail! (keyword (str "empty-" (name kind)))
                   "Stage1 input must contain one EDN value"
                   {:kind kind :path path}))
          (when-not (identical? trailing eof)
            (fail! (keyword (str "trailing-" (name kind)))
                   "Stage1 input must contain exactly one EDN value"
                   {:kind kind :path path}))
          value))
      (catch clojure.lang.ExceptionInfo cause
        (throw cause))
      (catch System.Exception cause
        (fail! (keyword (str "invalid-" (name kind)))
               "Stage1 input is not valid strict EDN"
               {:kind kind :path path :cause-type (str (type cause))})))))

(defn- sha256-bytes
  [bytes]
  (with-open [digest (System.Security.Cryptography.SHA256/Create)]
    (-> (System.Convert/ToHexString (.ComputeHash digest bytes))
        (.ToLowerInvariant))))

(defn sha256-string
  [value]
  (sha256-bytes (.GetBytes utf8 (str value))))

(defn sha256-file
  [path]
  (let [path (require-regular-file! path :hash-input)]
    (with-open [stream (System.IO.File/OpenRead path)
                digest (System.Security.Cryptography.SHA256/Create)]
      (-> (System.Convert/ToHexString (.ComputeHash digest stream))
          (.ToLowerInvariant)))))

(defn closure-hash
  [rows]
  (sha256-string
   (apply str
          (map (fn [row]
                 (str (get row "sha256") "  " (get row "path") "\n"))
               rows))))

(defn- json-string
  [value]
  (str "\""
       (apply str
              (map (fn [character]
                     (case character
                       \" "\\\""
                       \\ "\\\\"
                       \backspace "\\b"
                       \formfeed "\\f"
                       \newline "\\n"
                       \return "\\r"
                       \tab "\\t"
                       (if (< (int character) 32)
                         (format "\\u%04x" (int character))
                         (str character))))
                   (str value)))
       "\""))

(declare json-encode)

(defn- json-map
  [value]
  (str "{"
       (str/join ","
                 (map (fn [[key item]]
                        (when-not (string? key)
                          (fail! :json-key-not-string
                                 "Stage1 JSON keys must be strings"
                                 {:key (str key)}))
                        (str (json-string key) ":" (json-encode item)))
                      (sort-by key value)))
       "}"))

(defn json-encode
  [value]
  (cond
    (nil? value) "null"
    (string? value) (json-string value)
    (true? value) "true"
    (false? value) "false"
    (integer? value) (str value)
    (map? value) (json-map value)
    (sequential? value) (str "[" (str/join "," (map json-encode value)) "]")
    :else (fail! :json-value-unsupported
                 "Stage1 manifest value is outside the JSON domain"
                 {:value-type (str (type value))})))

(defn- physical-path
  [path]
  (let [full (System.IO.Path/GetFullPath (str path))
        root (System.IO.Path/GetPathRoot full)
        relative (.Substring full (.Length root))
        parts (remove str/blank? (str/split relative #"[\\/]+"))]
    (loop [current root remaining parts]
      (if-let [part (first remaining)]
        (let [candidate (System.IO.Path/Combine current part)
              exists-file (System.IO.File/Exists candidate)
              exists-dir (System.IO.Directory/Exists candidate)
              resolved (when (and (or exists-file exists-dir)
                                  (reparse-point? candidate))
                         (.ResolveLinkTarget
                          (if exists-dir
                            (System.IO.DirectoryInfo. candidate)
                            (System.IO.FileInfo. candidate))
                          true))]
          (recur (if resolved (.FullName resolved) candidate) (next remaining)))
        (System.IO.Path/TrimEndingDirectorySeparator current)))))

(defn- normalized-path
  [path]
  (physical-path path))

(defn- path-comparison
  []
  (if (or (System.OperatingSystem/IsWindows)
          (System.OperatingSystem/IsMacOS))
    System.StringComparison/OrdinalIgnoreCase
    System.StringComparison/Ordinal))

(defn- same-path?
  [left right]
  (.Equals (normalized-path left) (normalized-path right) (path-comparison)))

(defn- below-path?
  [candidate ancestor]
  (let [ancestor (normalized-path ancestor)
        suffix (str System.IO.Path/DirectorySeparatorChar)
        prefix (if (.EndsWith ancestor suffix (path-comparison))
                 ancestor
                 (str ancestor suffix))]
    (.StartsWith (normalized-path candidate) prefix (path-comparison))))

(defn- overlap?
  [left right]
  (or (same-path? left right)
      (below-path? left right)
      (below-path? right left)))

(defn- validate-disjoint!
  [profile-path plan-path source-path output]
  (let [inputs [[:profile profile-path] [:plan plan-path] [:source source-path]]]
    (doseq [[left-kind left] inputs
            [right-kind right] inputs
            :when (neg? (compare (name left-kind) (name right-kind)))]
      (when (overlap? left right)
        (fail! :input-path-overlap
               "Stage1 input paths must be distinct"
               {:left-kind left-kind :right-kind right-kind})))
    (doseq [[kind input] inputs]
      (when (overlap? input output)
        (fail! :output-path-overlap
               "Stage1 output must be disjoint from every input"
               {:input-kind kind :input (normalized-path input)
                :output (normalized-path output)})))))

(defn- delete-tree!
  [path]
  (when (System.IO.Directory/Exists path)
    (System.IO.Directory/Delete path true)))

(defn- publish-directory!
  [temporary output]
  ;; Publication is a single same-filesystem rename. Refusing every existing
  ;; output avoids deleting unrelated caller data and avoids a rename gap.
  (when (or (System.IO.File/Exists output)
            (System.IO.Directory/Exists output))
    (fail! :output-exists
           "Stage1 publication requires an absent output path"
           {:output output}))
  (System.IO.Directory/Move temporary output))

(defn- emit-run-instruction!
  [il instruction]
  (case (first instruction)
    :ldc-i8 (.Emit il System.Reflection.Emit.OpCodes/Ldc_I8
                   (long (second instruction)))
    :ldarg-0 (.Emit il System.Reflection.Emit.OpCodes/Ldarg_0)
    :add-ovf (.Emit il System.Reflection.Emit.OpCodes/Add_Ovf)
    :sub-ovf (.Emit il System.Reflection.Emit.OpCodes/Sub_Ovf)
    :mul-ovf (.Emit il System.Reflection.Emit.OpCodes/Mul_Ovf)
    (fail! :unknown-ir-instruction
           "lowering core produced an unknown backend instruction"
           {:instruction (pr-str instruction)})))

(defn- emit-executable!
  [path instructions]
  (let [assembly-name (System.Reflection.AssemblyName. target-assembly)
        assembly (System.Reflection.Emit.PersistedAssemblyBuilder.
                  assembly-name (.Assembly (type (Object.))) nil)
        module (.DefineDynamicModule assembly target-assembly)
        program-type (.DefineType
                      module target-type
                      (enum-or System.Reflection.TypeAttributes/Public
                               System.Reflection.TypeAttributes/Abstract
                               System.Reflection.TypeAttributes/Sealed)
                      System.Object)
        run-method (.DefineMethod
                    program-type target-method
                    (enum-or System.Reflection.MethodAttributes/Public
                             System.Reflection.MethodAttributes/Static)
                    System.Int64
                    (into-array System.Type [System.Int64]))
        run-il (.GetILGenerator run-method)
        main-method (.DefineMethod
                     program-type target-entry
                     (enum-or System.Reflection.MethodAttributes/Public
                              System.Reflection.MethodAttributes/Static)
                     System.Void
                     (into-array System.Type
                                 [(type (make-array System.String 0))]))
        main-il (.GetILGenerator main-method)
        invariant-culture (.GetProperty System.Globalization.CultureInfo
                                        "InvariantCulture")
        invariant-getter (.GetGetMethod invariant-culture)
        parse-method (.GetMethod System.Int64 "Parse"
                                 (into-array System.Type
                                             [System.String System.IFormatProvider]))
        format-i64 (.GetMethod System.Convert "ToString"
                               (into-array System.Type
                                           [System.Int64 System.IFormatProvider]))
        write-line (.GetMethod System.Console "WriteLine"
                               (into-array System.Type [System.String]))
        argument-error (.GetConstructor
                        System.ArgumentException
                        (into-array System.Type [System.String]))
        argument-count-ok (.DefineLabel main-il)]
    (doseq [instruction instructions]
      (emit-run-instruction! run-il instruction))
    (.Emit run-il System.Reflection.Emit.OpCodes/Ret)

    ;; static void Main(string[] argv) {
    ;;   if (argv.Length != 1) throw new ArgumentException(...);
    ;;   Console.WriteLine(Run(Int64.Parse(argv[0], InvariantCulture)));
    ;; }
    (.Emit main-il System.Reflection.Emit.OpCodes/Ldarg_0)
    (.Emit main-il System.Reflection.Emit.OpCodes/Ldlen)
    (.Emit main-il System.Reflection.Emit.OpCodes/Conv_I4)
    (.Emit main-il System.Reflection.Emit.OpCodes/Ldc_I4_1)
    (.Emit main-il System.Reflection.Emit.OpCodes/Beq argument-count-ok)
    (.Emit main-il System.Reflection.Emit.OpCodes/Ldstr
           "expected exactly one System.Int64 argument")
    (.Emit main-il System.Reflection.Emit.OpCodes/Newobj argument-error)
    (.Emit main-il System.Reflection.Emit.OpCodes/Throw)
    (.MarkLabel main-il argument-count-ok)
    (.Emit main-il System.Reflection.Emit.OpCodes/Ldarg_0)
    (.Emit main-il System.Reflection.Emit.OpCodes/Ldc_I4_0)
    (.Emit main-il System.Reflection.Emit.OpCodes/Ldelem_Ref)
    (.Emit main-il System.Reflection.Emit.OpCodes/Call invariant-getter)
    (.Emit main-il System.Reflection.Emit.OpCodes/Call parse-method)
    (.Emit main-il System.Reflection.Emit.OpCodes/Call run-method)
    (.Emit main-il System.Reflection.Emit.OpCodes/Call invariant-getter)
    (.Emit main-il System.Reflection.Emit.OpCodes/Call format-i64)
    (.Emit main-il System.Reflection.Emit.OpCodes/Call write-line)
    (.Emit main-il System.Reflection.Emit.OpCodes/Ret)
    (.CreateType program-type)

    (let [il-stream nil
          field-data nil
          metadata (.GenerateMetadata assembly
                                      (by-ref il-stream)
                                      (by-ref field-data))
          header (System.Reflection.PortableExecutable.PEHeaderBuilder/CreateExecutableHeader)
          metadata-root (System.Reflection.Metadata.Ecma335.MetadataRootBuilder.
                         metadata nil false)
          entry-point (System.Reflection.Metadata.Ecma335.MetadataTokens/MethodDefinitionHandle
                       (.MetadataToken main-method))
          pe-builder (System.Reflection.PortableExecutable.ManagedPEBuilder.
                      header metadata-root il-stream field-data
                      nil nil nil 0 entry-point
                      System.Reflection.PortableExecutable.CorFlags/ILOnly
                      nil)
          pe-blob (System.Reflection.Metadata.BlobBuilder. 0)]
      (.Serialize pe-builder pe-blob)
      (with-open [output (System.IO.FileStream.
                         path System.IO.FileMode/Create System.IO.FileAccess/Write)]
        (.WriteContentTo pe-blob output)))))

(defn- runtime-config-json
  []
  (str "{\"runtimeOptions\":{\"tfm\":\"net10.0\","
       "\"framework\":{\"name\":\"Microsoft.NETCore.App\","
       "\"version\":\"10.0.0\"}}}\n"))

(defn- verify-managed-target!
  [dll-path]
  (let [identity (System.Reflection.AssemblyName/GetAssemblyName dll-path)]
    (when-not (= target-assembly (.Name identity))
      (fail! :target-assembly-name
             "generated target assembly identity is incorrect"
             {:actual (.Name identity)})))
  (let [context (System.Runtime.Loader.AssemblyLoadContext.
                 (str "clr-meta-stage1-verify-"
                      (.ToString (System.Guid/NewGuid) "N"))
                 true)]
    (try
      (let [assembly (.LoadFromAssemblyPath context
                                            (System.IO.Path/GetFullPath dll-path))
            references (->> (.GetReferencedAssemblies assembly)
                            (map #(.Name %))
                            sort
                            vec)
            expected-references ["System.Console" "System.Private.CoreLib"]
            resources (vec (.GetManifestResourceNames assembly))
            program (.GetType assembly target-type false false)
            run (when program (.GetMethod program target-method))
            entry (.EntryPoint assembly)]
        (when-not (= expected-references references)
          (fail! :target-reference-set
                 "generated target has an unexpected assembly reference"
                 {:expected expected-references :actual references}))
        (when-not (empty? resources)
          (fail! :target-resource-set
                 "generated target must not embed resources"
                 {:resources resources}))
        (when-not (and program run entry
                       (= target-entry (.Name entry))
                       (= System.Void (.ReturnType entry))
                       (= [(type (make-array System.String 0))]
                          (mapv #(.ParameterType %) (.GetParameters entry)))
                       (.IsPublic entry)
                       (.IsStatic entry)
                       (= System.Int64 (.ReturnType run))
                       (= [System.Int64]
                          (mapv #(.ParameterType %) (.GetParameters run)))
                       (.IsPublic run)
                       (.IsStatic run))
          (fail! :target-entry-contract
                 "generated target entry/callable ABI is incorrect"
                 {}))
        ;; Force the CLR to parse and JIT both method bodies without executing
        ;; source semantics. Invalid IL fails before publication.
        (System.Runtime.CompilerServices.RuntimeHelpers/PrepareMethod
         (.MethodHandle run))
        (System.Runtime.CompilerServices.RuntimeHelpers/PrepareMethod
         (.MethodHandle entry))
        {:references references :resources resources})
      (finally
        (.Unload context)))))

(defn build!
  "Compile one snapshotted Stage1 source into a fail-preserving executable."
  [profile-path plan-path source-path output-dir fallback-counts]
  (let [profile-path (require-regular-file! profile-path :profile)
        plan-path (require-regular-file! plan-path :plan)
        source-path (require-regular-file! source-path :source)
        output (normalized-path output-dir)
        parent (System.IO.Path/GetDirectoryName output)
        nonce (.ToString (System.Guid/NewGuid) "N")
        temporary (str output ".building." nonce)
        snapshot-root (System.IO.Path/Combine
                       (or parent (System.IO.Path/GetTempPath))
                       (str ".clr-meta-stage1-input." nonce))]
    (validate-disjoint! profile-path plan-path source-path output)
    (when (or (nil? parent) (= "" parent))
      (fail! :output-parent-missing
             "Stage1 output must have a parent directory"
             {:output output}))
    (when (or (System.IO.File/Exists output)
              (System.IO.Directory/Exists output))
      (fail! :output-exists
             "Stage1 build refuses to replace an existing output"
             {:output output}))
    (System.IO.Directory/CreateDirectory parent)
    (delete-tree! temporary)
    (delete-tree! snapshot-root)
    (System.IO.Directory/CreateDirectory snapshot-root)
    (try
      (let [profile-snapshot (System.IO.Path/Combine snapshot-root "profile.edn")
            plan-snapshot (System.IO.Path/Combine snapshot-root "plan.edn")
            source-snapshot (System.IO.Path/Combine snapshot-root "source.clj")]
        ;; Parse, lower, and hash the same private byte snapshots. A caller
        ;; replacing a live input cannot split semantics from the receipt.
        (System.IO.File/Copy profile-path profile-snapshot true)
        (System.IO.File/Copy plan-path plan-snapshot true)
        (System.IO.File/Copy source-path source-snapshot true)
        (let [profile (strict-read-edn profile-snapshot :profile)
              plan (strict-read-edn plan-snapshot :plan)
              form (strict-read-edn source-snapshot :source)]
          (core/validate-profile profile)
          (when-not (= expected-plan plan)
            (fail! :plan-contract-mismatch
                   "Stage1 plan does not equal the admitted exact plan"
                   {:expected expected-plan :actual plan}))
          (let [ir (core/lower profile form)]
            (System.IO.Directory/CreateDirectory temporary)
            (try
              (let [dll-path (System.IO.Path/Combine temporary "program.dll")
                    runtime-path (System.IO.Path/Combine
                                  temporary "program.runtimeconfig.json")]
                (emit-executable! dll-path (:instructions ir))
                (System.IO.File/WriteAllText runtime-path (runtime-config-json) utf8)
                (let [verification (verify-managed-target! dll-path)
                      outputs [{"path" "program.dll"
                                "sha256" (sha256-file dll-path)}
                               {"path" "program.runtimeconfig.json"
                                "sha256" (sha256-file runtime-path)}]
                      accounting (:accounting ir)
                      counts (if (instance? clojure.lang.IDeref fallback-counts)
                               @fallback-counts
                               fallback-counts)]
                  (when-not (and (map? counts)
                                 (= fallback-counter-keys (set (keys counts)))
                                 (every? zero? (vals counts)))
                    (fail! :fallback-counter-nonzero
                           "target-form host fallback counter is nonzero"
                           {:counts counts}))
                  (let [manifest
                        {"backend" "direct-system-reflection-emit"
                         "target_form_fallback_calls" counts
                         "compiler_self_reproduction" false
                         "compiler_stage" 1
                         "compiler_stage2" false
                         "entry" "ClrMeta.Stage1.Program/Main"
                         "il_fixed_point" false
                         "input_kind" "strict-edn-one-form"
                         "ir_sha256" (sha256-string
                                      (str (pr-str (:instructions ir)) "\n"))
                         "node_accounting"
                         {"directly_lowered" (:directly-lowered accounting)
                          "max_depth" (:max-depth accounting)
                          "nodes" (:nodes accounting)
                          "runtime_primitive_nodes" 0
                          "unknown_nodes" 0}
                         "output_closure_sha256" (closure-hash outputs)
                         "outputs" outputs
                         "plan_sha256" (sha256-file plan-snapshot)
                         "profile_id" (name core/profile-id)
                         "profile_sha256" (sha256-file profile-snapshot)
                         "raw_artifact_reproducibility" false
                         "schema" target-manifest-schema
                         "source_sha256" (sha256-file source-snapshot)
                         "source_text_compilation" true
                         "target" target-framework
                         "target_assembly_references" (:references verification)
                         "target_resources" (:resources verification)}]
                    (System.IO.File/WriteAllText
                     (System.IO.Path/Combine temporary "manifest.json")
                     (str (json-encode manifest) "\n") utf8)
                    (publish-directory! temporary output)
                    manifest)))
              (finally
                (delete-tree! temporary))))))
      (finally
        (delete-tree! snapshot-root)))))

(defn- forbidden-fallback
  [counts key]
  (fn [& _]
    (swap! counts update key inc)
    (fail! :host-fallback-called
           "target compilation attempted a forbidden host fallback"
           {:fallback key})))

(defn -main
  [& args]
  (when-not (= 4 (count args))
    (binding [*out* *err*]
      (println "usage: compiler-stage1 PROFILE PLAN SOURCE OUTPUT"))
    (System.Environment/Exit 2))
  (let [counts (atom {"compile" 0
                      "eval" 0
                      "load" 0
                      "load_file" 0
                      "load_string" 0})
        forbidden (fn [key] (forbidden-fallback counts key))
        manifest (with-redefs [clojure.core/compile (forbidden "compile")
                               clojure.core/eval (forbidden "eval")
                               clojure.core/load (forbidden "load")
                               clojure.core/load-file (forbidden "load_file")
                               clojure.core/load-string (forbidden "load_string")]
                   (apply build! (concat args [counts])))]
    (when-not (every? zero? (vals @counts))
      (fail! :fallback-counter-nonzero
             "target-form host fallback counter changed after publication"
             {:counts @counts}))
    (prn manifest)
    (flush)))
