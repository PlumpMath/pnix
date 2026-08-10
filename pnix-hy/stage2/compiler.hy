"Hy implementation of the stage compiler protocol."

(import ast sys types)
(import pathlib [Path])
(import hy.reader [read-many])
(import hy.compiler [hy-compile])

(setv DEFAULT-MODULE "hy_meta_stage2.session")
(setv DIRECT-KERNEL None)
(setv DIRECT-KERNEL-ENABLED True)
(setv DIRECT-KERNEL-STRICT False)
(setv DIRECT-KERNEL-HITS 0)
(setv DIRECT-KERNEL-FALLBACKS 0)
(setv DIRECT-KERNEL-LAST-ERROR None)


(defn make-module [[name DEFAULT-MODULE] [filename None]]
  (setv module (types.ModuleType name))
  (setv module.__file__ (or filename (+ "<" name ">")))
  (setv module.__package__ (. (name.rpartition ".") [0]))
  (setv module.hy hy)
  (.setdefault module.__dict__ "_hy_macros" {})
  (.setdefault module.__dict__ "_hy_reader_macros" {})
  (setv (get sys.modules name) module)
  module)


(defn read-source [source [filename "<string>"]]
  (read-many source :filename filename))


(defn compile-model-to-ast [model [module None] [filename "<string>"]
                            [source None] [import-stdlib True]]
  (setv module (or module (make-module)))
  (ast.fix-missing-locations
    (hy-compile model module
                :filename filename
                :source source
                :import-stdlib import-stdlib)))


(defn compile-source-to-ast-upstream [source [module None] [filename "<string>"]
                                      [import-stdlib True]]
  (compile-model-to-ast
    (read-source source filename)
    (or module (make-module))
    filename
    source
    import-stdlib))


(defn direct-kernel-path []
  (setv compiler-path (Path __file__))
  (.with-name compiler-path "kernel.hy"))


(defn load-direct-kernel []
  (global DIRECT-KERNEL DIRECT-KERNEL-ENABLED)
  (when (is DIRECT-KERNEL None)
    (setv kernel-module-name (+ __name__ "._direct_kernel"))
    (setv cached-kernel (.get sys.modules kernel-module-name))
    (if cached-kernel
        (setv DIRECT-KERNEL cached-kernel)
        (do
          (setv previous-enabled DIRECT-KERNEL-ENABLED)
          (setv DIRECT-KERNEL-ENABLED False)
          (try
            (setv DIRECT-KERNEL
                  (load-hy-file (direct-kernel-path)
                                kernel-module-name))
            (finally
              (setv DIRECT-KERNEL-ENABLED previous-enabled))))))
  DIRECT-KERNEL)


(defn compile-source-to-ast-direct [source module filename]
  (setv kernel (load-direct-kernel))
  (ast.fix-missing-locations
    (kernel.compile-source-to-module source
                                     filename
                                     None
                                     module.__name__
                                     module.__package__)))


(defn compile-source-to-ast [source [module None] [filename "<string>"]
                             [import-stdlib True]]
  (global DIRECT-KERNEL-HITS DIRECT-KERNEL-FALLBACKS DIRECT-KERNEL-LAST-ERROR)
  (setv module (or module (make-module)))
  (if (and DIRECT-KERNEL-ENABLED import-stdlib)
      (try
        (do
          (setv tree (compile-source-to-ast-direct source module filename))
          (setv DIRECT-KERNEL-HITS (+ DIRECT-KERNEL-HITS 1))
          (setv DIRECT-KERNEL-LAST-ERROR None)
          tree)
        (except [e Exception]
          (setv DIRECT-KERNEL-FALLBACKS (+ DIRECT-KERNEL-FALLBACKS 1))
          (setv DIRECT-KERNEL-LAST-ERROR (repr e))
          (when DIRECT-KERNEL-STRICT
            (raise e))
          (compile-source-to-ast-upstream source module filename import-stdlib)))
      (compile-source-to-ast-upstream source module filename import-stdlib)))


(defn compile-source-to-code [source [module None] [filename "<string>"]
                              [import-stdlib True]]
  (compile
    (compile-source-to-ast source
                           (or module (make-module))
                           filename
                           import-stdlib)
    filename
    "exec"))


(defn python-source [source [module None] [filename "<string>"]
                    [import-stdlib True]]
  (ast.unparse
    (compile-source-to-ast source
                           (or module (make-module))
                           filename
                           import-stdlib)))


(defn exec-source [source [module None] [filename "<string>"]
                  [import-stdlib True]]
  (setv module (or module (make-module)))
  (exec (compile-source-to-code source module filename import-stdlib)
        module.__dict__)
  module)


(defn assign-eval-result [tree result-name]
  (setv body tree.body)
  (if (and (> (len body) 0)
           (isinstance (get body -1) ast.Expr))
      (do
        (setv final-expr (get body -1))
        (setv (get body -1)
              (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                          :value final-expr.value)))
      (.append body
               (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                           :value (ast.Constant :value None))))
  (ast.fix-missing-locations tree))


(defn eval-source [source [module None] [filename "<string>"]
                  [import-stdlib True]]
  (setv module (or module (make-module)))
  (setv result-name "__hy_meta_stage2_eval_result__")
  (setv tree (assign-eval-result
               (compile-source-to-ast source module filename import-stdlib)
               result-name))
  (exec (compile tree filename "exec") module.__dict__)
  (get module.__dict__ result-name))


(defn direct-kernel-stats []
  {"enabled" DIRECT-KERNEL-ENABLED
   "strict" DIRECT-KERNEL-STRICT
   "loaded" (not (is DIRECT-KERNEL None))
   "hits" DIRECT-KERNEL-HITS
   "fallbacks" DIRECT-KERNEL-FALLBACKS
   "last_error" DIRECT-KERNEL-LAST-ERROR})


(defn set-direct-kernel-strict [enabled]
  (global DIRECT-KERNEL-STRICT)
  (setv DIRECT-KERNEL-STRICT enabled)
  DIRECT-KERNEL-STRICT)


(defn reset-direct-kernel-stats []
  (global DIRECT-KERNEL-HITS DIRECT-KERNEL-FALLBACKS DIRECT-KERNEL-LAST-ERROR)
  (setv DIRECT-KERNEL-HITS 0)
  (setv DIRECT-KERNEL-FALLBACKS 0)
  (setv DIRECT-KERNEL-LAST-ERROR None)
  (direct-kernel-stats))


(defn load-hy-file [path [module-name None] [import-stdlib True]]
  (setv path (Path path))
  (setv name (or module-name path.stem))
  (setv module (make-module name (str path)))
  (exec-source (.read-text path) module (str path) import-stdlib)
  module)


(defn eval-file [path [module-name "hy_meta_stage2.file"] [import-stdlib True]]
  (setv path (Path path))
  (eval-source (.read-text path)
               (make-module module-name (str path))
               (str path)
               import-stdlib))


(defn hy-meta-check [kernel-path factorial-path features-path loop-path
                     [kernel-module-name "hy_meta_stage2_prime.kernel"]]
  (setv kernel (load-hy-file kernel-path kernel-module-name))
  (setv factorial-path (Path factorial-path))
  (setv features-path (Path features-path))
  (setv loop-path (Path loop-path))
  (setv factorial-source (.read-text factorial-path))
  (setv features-source (.read-text features-path))
  (setv loop-source (.read-text loop-path))
  {"kernel_self_check" (kernel.self-check)
   "factorial" (kernel.eval-source factorial-source None (str factorial-path))
   "features" (kernel.eval-source features-source None (str features-path))
   "loop" (kernel.eval-source loop-source None (str loop-path))
   "factorial_python_contains_fact" (in "def fact"
                                        (kernel.python-source factorial-source
                                                              (str factorial-path)))
   "kernel_module" kernel.__name__})


(defn self-check []
  (setv module (make-module "hy_meta_stage2.self_check"))
  (and
    (= (eval-source "(+ 1 2 3)" module "<stage2:self-check>") 6)
    (do
      (exec-source "(defn f [x] (+ x 10))" module "<stage2:self-check>")
      (= ((get module.__dict__ "f") 32) 42))))
