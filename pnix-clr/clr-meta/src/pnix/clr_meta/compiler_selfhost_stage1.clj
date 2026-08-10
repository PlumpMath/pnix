(ns pnix.clr-meta.compiler-selfhost-stage1
  "Explicit pinned-ClojureCLR B0 adapter for the selfhost compiler family.

  This namespace is bootstrap TCB only. It loads the exact canonical kernel
  through the host ClojureCLR Compiler, installs the frozen low-level ABI as
  Vars backed by CompilerSupport, and invokes the kernel to emit Compiler
  Stage1. Generated artifacts do not reference this namespace or ClojureCLR."
  )

(def stage1-schema "pnix.clr-meta.compiler-selfhost-stage1-bootstrap.v1")
(def kernel-namespace 'pnix.clr-meta.compiler-kernel.v1)
(def kernel-entry 'compile-source)
(def generated-type "Pnix.ClrMeta.Generated.CompilerKernel")

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

(def support-spec
  {'pnix.clr-meta.compiler-support.reader.v1
   {'read-all ["Pnix.ClrMeta.CompilerSupport.ReaderAbi" "ReadAll" 2]}

   'pnix.clr-meta.compiler-support.data.v1
   {'kind-is? ["Pnix.ClrMeta.CompilerSupport.DataAbi" "KindIs" 2]
    'count ["Pnix.ClrMeta.CompilerSupport.DataAbi" "Count" 1]
    'nth ["Pnix.ClrMeta.CompilerSupport.DataAbi" "Nth" 2]
    'symbol-name ["Pnix.ClrMeta.CompilerSupport.DataAbi" "SymbolName" 1]
    'string-equal? ["Pnix.ClrMeta.CompilerSupport.DataAbi" "StringEqual" 2]
    'env-new ["Pnix.ClrMeta.CompilerSupport.DataAbi" "EnvNew" 0]
    'env-bind ["Pnix.ClrMeta.CompilerSupport.DataAbi" "EnvBind" 5]
    'env-lookup ["Pnix.ClrMeta.CompilerSupport.DataAbi" "EnvLookup" 2]
    'reject ["Pnix.ClrMeta.CompilerSupport.DataAbi" "Reject" 3]}

   'pnix.clr-meta.compiler-support.pesink.v1
   {'begin ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "Begin" 5]
    'define-constant ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "DefineConstant" 2]
    'define-method ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "DefineMethod" 3]
    'begin-initializer ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "BeginInitializer" 1]
    'end-initializer ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EndInitializer" 1]
    'begin-method ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "BeginMethod" 3]
    'end-method ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EndMethod" 1]
    'allocate-local ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "AllocateLocal" 1]
    'new-label ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "NewLabel" 1]
    'mark-label ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "MarkLabel" 2]
    'emit-literal ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitLiteral" 3]
    'emit-load-arg ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitLoadArg" 2]
    'emit-load-local ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitLoadLocal" 2]
    'emit-load-field ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitLoadField" 2]
    'emit-store-local ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitStoreLocal" 2]
    'emit-store-field ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitStoreField" 2]
    'emit-call ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitCall" 3]
    'emit-opcode ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitOpcode" 2]
    'emit-branch-false ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitBranchFalse" 2]
    'emit-branch ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitBranch" 2]
    'emit-pop ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitPop" 1]
    'emit-ret ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "EmitRet" 1]
    'finish ["Pnix.ClrMeta.CompilerSupport.PeSinkAbi" "Finish" 1]}})

(defn- fail!
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.compiler-selfhost-stage1-error.v1
                          :phase :bootstrap-b0
                          :class class}
                         evidence))))

(defn- require-regular-file!
  [path kind]
  (let [full (System.IO.Path/GetFullPath (str path))]
    (when-not (System.IO.File/Exists full)
      (fail! (keyword (str "missing-" (name kind)))
             "B0 input file is unavailable"
             {:kind kind :path full}))
    (when-not (zero? (bit-and (int (System.IO.File/GetAttributes full))
                              (int System.IO.FileAttributes/ReparsePoint)))
      (fail! (keyword (str (name kind) "-reparse-point"))
             "B0 input file must not be a symlink/reparse point"
             {:kind kind :path full}))
    full))

(defn- method-by-name!
  [type method-name arity]
  (let [matches (->> (.GetMethods type)
                     (filter #(and (= method-name (.Name %))
                                   (.IsStatic %)
                                   (= arity (alength (.GetParameters %)))))
                     vec)]
    (when-not (= 1 (count matches))
      (fail! :support-method-shape
             "support assembly does not expose one exact ABI method"
             {:type (.FullName type)
              :method method-name
              :arity arity
              :matches (count matches)}))
    (first matches)))

(defn- invoke-static
  [method arguments]
  (try
    (.Invoke method nil (object-array arguments))
    (catch System.Reflection.TargetInvocationException cause
      (throw (.InnerException cause)))))

(defn- fixed-arity-wrapper
  [method arity]
  (case arity
    0 (fn [] (invoke-static method []))
    1 (fn [a] (invoke-static method [a]))
    2 (fn [a b] (invoke-static method [a b]))
    3 (fn [a b c] (invoke-static method [a b c]))
    4 (fn [a b c d] (invoke-static method [a b c d]))
    5 (fn [a b c d e] (invoke-static method [a b c d e]))
    (fail! :support-arity
           "support ABI arity is outside the fixed bootstrap wrapper set"
           {:arity arity})))

(defn- install-support!
  [assembly]
  (doseq [[namespace entries] support-spec]
    (let [target-ns (or (find-ns namespace) (create-ns namespace))]
      (doseq [[symbol [type-name method-name arity]] entries]
        (let [type (.GetType assembly type-name true false)
              method (method-by-name! type method-name arity)]
          (intern target-ns symbol (fixed-arity-wrapper method arity))))))
  true)

(defn- construct-sink!
  [assembly output]
  (let [type (.GetType assembly "Pnix.ClrMeta.CompilerSupport.PeSink" true false)
        constructor (.GetConstructor type (into-array System.Type [System.String]))]
    (when-not constructor
      (fail! :support-sink-constructor
             "support assembly is missing the exact PeSink(String) constructor"
             {}))
    (try
      (.Invoke constructor (object-array [output]))
      (catch System.Reflection.TargetInvocationException cause
        (throw (.InnerException cause))))))

(defn- strict-source-text
  [assembly source-path]
  (let [type (.GetType assembly "Pnix.ClrMeta.CompilerSupport.ReaderAbi" true false)
        method (method-by-name! type "ReadStrictUtf8File" 1)]
    (invoke-static method [source-path])))

(defn seed-stage1!
  "Execute one explicit B0 seed step. The caller owns exact-input admission."
  [support-path source-path output-path]
  (let [support-path (require-regular-file! support-path :support)
        source-path (require-regular-file! source-path :source)
        output-path (System.IO.Path/GetFullPath (str output-path))
        output-parent (System.IO.Path/GetDirectoryName output-path)]
    (when-not (System.IO.Directory/Exists output-parent)
      (fail! :output-parent-missing
             "B0 output parent directory is unavailable"
             {:output output-path}))
    (when (or (System.IO.File/Exists output-path)
              (System.IO.Directory/Exists output-path))
      (fail! :output-exists
             "B0 output path must be absent"
             {:output output-path}))
    (let [assembly (System.Reflection.Assembly/LoadFrom support-path)
          _ (install-support! assembly)
          source-text (strict-source-text assembly source-path)
          sink (construct-sink! assembly output-path)]
      ;; Explicit bootstrap-only host Compiler boundary. The generated PE has
      ;; no dependency on this source, this namespace, or ClojureCLR.
      (load-file source-path)
      (let [entry (ns-resolve kernel-namespace kernel-entry)]
        (when-not entry
          (fail! :kernel-entry-missing
                 "loaded canonical source did not define the compiler entry"
                 {:namespace kernel-namespace :entry kernel-entry}))
        (let [descriptor (entry source-text sink)
              descriptor-type (.GetType descriptor)
              path-property (.GetProperty descriptor-type "Path")
              actual-path (when path-property (.GetValue path-property descriptor nil))]
          (when-not (and (= output-path actual-path)
                         (System.IO.File/Exists output-path))
            (fail! :artifact-publication
                   "kernel did not publish exactly the requested Stage1 artifact"
                   {:expected output-path :actual actual-path}))
          descriptor)))))

(defn -main
  [& arguments]
  (when-not (= 3 (count arguments))
    (fail! :argument-count
           "expected: <CompilerSupport.dll> <compiler_kernel.clj> <output.dll>"
           {:count (count arguments)}))
  (let [[support source output] arguments
        descriptor (seed-stage1! support source output)]
    (println
     (str "{\"schema\":" (json-string stage1-schema)
          ",\"output\":"
          (json-string
           (.GetValue (.GetProperty (.GetType descriptor) "Path")
                      descriptor nil))
          "}"))))
