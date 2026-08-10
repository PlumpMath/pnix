"A small Hy-written compiler kernel for the meta-circular path."

(import ast builtins hy importlib sys textwrap traceback types warnings)
(import hy.reader [mangle read-many unmangle])
(import hy.reader.mangling [slashes2dots])
(import hy.models [as-model Bytes Complex Dict Expression FComponent Float FString Integer Keyword List Set String Symbol Tuple])

(setv RESULT-NAME "__hy_meta_result__")
(setv PENDING-STATEMENTS [])
(setv GENERATED-NAME-INDEX 0)
(setv LOCAL-MACRO-INDEX 0)
(setv FUNCTION-SCOPE-DEPTH 0)
(setv MODULE-BINDING-NAMES [])
(setv FUNCTION-BINDING-STACK [])
(setv LET-PROTECTED-BINDING-STACK [])
(setv BINDING-CONSTANT-NAMES ["None" "True" "False"])
(setv REQUIRE-PACKAGE None)
(setv WARN-ON-CORE-SHADOW True)


(defn model-list [models]
  (list models))


(defn symbol-named? [model name]
  (and (isinstance model Symbol) (= (str model) name)))


(defn keyword-named? [model name]
  (and (isinstance model Keyword) (= model.name name)))


(defn expression-head [model]
  (if (and (isinstance model Expression) (> (len model) 0))
      (get model 0)
      None))


(defn statement-head? [head]
  (or (symbol-named? head "setv")
      (symbol-named? head "annotate")
      (symbol-named? head "defn")
      (symbol-named? head "defclass")
      (symbol-named? head "deftype")
      (symbol-named? head "import")
      (symbol-named? head "pys")
      (symbol-named? head "defmacro")
      (symbol-named? head "while")
      (symbol-named? head "for")
      (symbol-named? head "with")
      (symbol-named? head "try")
      (symbol-named? head "raise")
      (symbol-named? head "pass")
      (symbol-named? head "assert")
      (symbol-named? head "global")
      (symbol-named? head "nonlocal")
      (symbol-named? head "del")
      (symbol-named? head "+=")
      (symbol-named? head "-=")
      (symbol-named? head "*=")
      (symbol-named? head "/=")
      (symbol-named? head "return")
      (symbol-named? head "yield")
      (symbol-named? head "break")
      (symbol-named? head "continue")
      (and (isinstance head Symbol) (in (str head) AUGOPS))))


(defn cond-result-forms [model]
  (setv results [])
  (for [index (range 2 (len model) 2)]
    (.append results (get model index)))
  results)


(defn statement-form? [model]
  (setv head (expression-head model))
  (or (statement-head? head)
      (wrapped-annotation-form? model)
      (and (symbol-named? head "if")
           (any (map statement-form? (cut model 1 None))))
      (and (symbol-named? head "when")
           (any (map statement-form? (cut model 2 None))))
      (and (symbol-named? head "cond")
           (any (map statement-form? (cond-result-forms model))))
      (and (symbol-named? head "do")
           (any (map statement-form? (cut model 1 None))))))


(defn expression-valued-statement-form? [model]
  (setv head (expression-head model))
  (or (symbol-named? head "try")
      (symbol-named? head "with")
      (symbol-named? head "match")))


(defn value-preserving-statement-form? [model]
  (setv head (expression-head model))
  (or (expression-valued-statement-form? model)
      (symbol-named? head "if")
      (symbol-named? head "when")
      (symbol-named? head "cond")
      (and (symbol-named? head "do")
           (or (= (len model) 1)
               (not (statement-form? (get model -1)))
               (value-preserving-statement-form? (get model -1))))))


(defn none-expression-statement-head? [head]
  (or (symbol-named? head "defn")
      (symbol-named? head "defclass")
      (symbol-named? head "deftype")
      (symbol-named? head "while")
      (symbol-named? head "for")
      (symbol-named? head "assert")
      (symbol-named? head "pass")
      (symbol-named? head "del")
      (and (isinstance head Symbol) (in (str head) AUGOPS))))


(defn async-keyword-in-list? [model]
  (and (isinstance model List)
       (any (map (fn [item] (keyword-named? item "async")) model))))


(defn async-comprehension-form? [model]
  (and (isinstance model Expression)
       (> (len model) 0)
       (isinstance (get model 0) Symbol)
       (in (str (get model 0)) ["lfor" "sfor" "dfor" "gfor"])
       (any (map (fn [item] (keyword-named? item "async"))
                 (cut model 1 None)))))


(defn async-helper-form? [model]
  (cond
    (isinstance model Expression)
      (do
        (setv head (expression-head model))
        (or
          (symbol-named? head "await")
          (and (symbol-named? head "with")
               (> (len model) 1)
               (async-keyword-in-list? (get model 1)))
          (and (symbol-named? head "for")
               (> (len model) 1)
               (async-keyword-in-list? (get model 1)))
          (async-comprehension-form? model)
          (any (map async-helper-form? model))))
    (or (isinstance model List)
        (isinstance model Tuple)
        (isinstance model Set))
      (any (map async-helper-form? model))
    (isinstance model Dict)
      (any (map async-helper-form? (model-list model)))
    True False))


(defn yield-form? [model]
  (cond
    (isinstance model Expression)
      (do
        (setv head (expression-head model))
        (cond
          (symbol-named? head "yield") True
          (or (symbol-named? head "fn")
              (symbol-named? head "defn")
              (symbol-named? head "defclass")
              (symbol-named? head "quote")
              (symbol-named? head "quasiquote")) False
          True (any (map yield-form? model))))
    (or (isinstance model List)
        (isinstance model Tuple)
        (isinstance model Set))
      (any (map yield-form? model))
    (isinstance model Dict)
      (any (map yield-form? (model-list model)))
    True False))


(defn control-flow-form? [model]
  (cond
    (isinstance model Expression)
      (do
        (setv head (expression-head model))
        (cond
          (or (symbol-named? head "break")
              (symbol-named? head "continue")
              (symbol-named? head "return")
              (symbol-named? head "yield"))
            True
          (or (symbol-named? head "fn")
              (symbol-named? head "defn")
              (symbol-named? head "defclass")
              (symbol-named? head "quote")
              (symbol-named? head "quasiquote"))
            False
          True
            (any (map control-flow-form? model))))
    (or (isinstance model List)
        (isinstance model Tuple)
        (isinstance model Set))
      (any (map control-flow-form? model))
    (isinstance model Dict)
      (any (map control-flow-form? (model-list model)))
    True False))


(defn scope-declaration-form? [model]
  (cond
    (isinstance model Expression)
      (do
        (setv head (expression-head model))
        (cond
          (or (symbol-named? head "global")
              (symbol-named? head "nonlocal"))
            True
          (or (symbol-named? head "fn")
              (symbol-named? head "defn")
              (symbol-named? head "defclass")
              (symbol-named? head "quote")
              (symbol-named? head "quasiquote"))
            False
          True
            (any (map scope-declaration-form? model))))
    (or (isinstance model List)
        (isinstance model Tuple)
        (isinstance model Set))
      (any (map scope-declaration-form? model))
    (isinstance model Dict)
      (any (map scope-declaration-form? (model-list model)))
    True False))


(defn defmacro-form? [model]
  (symbol-named? (expression-head model) "defmacro"))


(defn require-form? [model]
  (symbol-named? (expression-head model) "require"))


(defn defreader-form? [model]
  (symbol-named? (expression-head model) "defreader"))


(defn require-reader-form? [model]
  (and (require-form? model)
       (any (map (fn [item] (keyword-named? item "readers"))
                 (cut model 1 None)))))


(defn reader-affecting-form? [model]
  (cond
    (or (defreader-form? model)
        (require-reader-form? model))
      True
    (or (special-form? model "quote")
        (special-form? model "quasiquote"))
      False
    (isinstance model Expression)
      (any (map reader-affecting-form? model))
    (or (isinstance model List)
        (isinstance model Tuple)
        (isinstance model Set))
      (any (map reader-affecting-form? model))
    (isinstance model Dict)
      (any (map reader-affecting-form? (model-list model)))
    True False))


(defn eval-and-compile-form? [model]
  (symbol-named? (expression-head model) "eval-and-compile"))


(defn eval-when-compile-form? [model]
  (symbol-named? (expression-head model) "eval-when-compile"))


(defn pragma-form? [model]
  (symbol-named? (expression-head model) "pragma"))


(defn special-form? [model name]
  (and (isinstance model Expression)
       (= (len model) 2)
       (symbol-named? (get model 0) name)))


(defn annotation-form? [model]
  (and (isinstance model Expression)
       (= (len model) 3)
       (symbol-named? (get model 0) "annotate")))


(defn wrapped-annotation-form? [model]
  (and (isinstance model Expression)
       (= (len model) 1)
       (annotation-form? (get model 0))))


(defn annotation-target [model]
  (get model 1))


(defn annotation-type [model]
  (get model 2))


(defn unpack-iterable-form? [model]
  (special-form? model "unpack-iterable"))


(defn unpack-mapping-form? [model]
  (special-form? model "unpack-mapping"))


(defn placeholder-special-form-name [head]
  (if (isinstance head Symbol)
      (do
        (setv name (unmangle (str head)))
        (if (in name ["unquote" "unquote-splice" "except" "except*"])
            name
            None))
      None))


(defn reject-placeholder-special-form [head]
  (setv name (placeholder-special-form-name head))
  (when name
    (raise (SyntaxError (+ "`" name "` is not allowed here")))))


(defn fresh-generated-name [prefix]
  (global GENERATED-NAME-INDEX)
  (setv name (+ prefix (str GENERATED-NAME-INDEX)))
  (setv GENERATED-NAME-INDEX (+ GENERATED-NAME-INDEX 1))
  name)


(defn append-pending-statement [statement]
  (global PENDING-STATEMENTS)
  (.append PENDING-STATEMENTS statement))


(defn model-positioned? [model]
  (and (hasattr model "start_line")
       (not (is model.start_line None))
       (hasattr model "start_column")
       (not (is model.start_column None))))


(defn apply-model-location [node model]
  (when (and (isinstance node ast.AST)
             (model-positioned? model)
             (in "lineno" node._attributes))
    (setv node.lineno model.start_line)
    (setv node.col_offset (max 0 (- model.start_column 1)))
    (setv node.end_lineno (if (and (hasattr model "end_line")
                                   (not (is model.end_line None)))
                              model.end_line
                              model.start_line))
    (setv node.end_col_offset (if (and (hasattr model "end_column")
                                       (not (is model.end_column None)))
                                  model.end_column
                                  (max 0 (- model.start_column 1)))))
  node)


(defn apply-model-location-to-statements [statements model]
  (for [statement statements]
    (apply-model-location statement model))
  statements)


(defn drain-pending-statements []
  (global PENDING-STATEMENTS)
  (setv statements PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  statements)


(defn illegal-binding-symbol? [target]
  (and (isinstance target Symbol)
       (in (str target) BINDING-CONSTANT-NAMES)))


(defn validate-binding-symbol [target form-name]
  (when (illegal-binding-symbol? target)
    (raise (SyntaxError (+ "kernel cannot bind constant " (str target)
                           " in " form-name)))))


(defn store-target [target]
  (cond
    (isinstance target Symbol)
      (do
        (validate-binding-symbol target "assignment")
        (ast.Name :id (mangle (str target)) :ctx (ast.Store)))
    (and (isinstance target Expression)
         (symbol-named? (expression-head target) "."))
      (store-attribute-target (cut target 1 None))
    (and (isinstance target Expression)
         (symbol-named? (expression-head target) "get"))
      (store-subscript-target (cut target 1 None))
    (and (isinstance target Expression)
         (symbol-named? (expression-head target) "cut"))
      (store-slice-target (cut target 1 None))
    (isinstance target List)
      (ast.List :elts (store-sequence-targets target) :ctx (ast.Store))
    (isinstance target Tuple)
      (ast.Tuple :elts (store-sequence-targets target) :ctx (ast.Store))
    True
      (raise (SyntaxError (+ "kernel can only assign to symbols, attributes, or sequences, not "
                             (repr target))))))


(defn store-sequence-targets [items]
  (setv targets [])
  (setv starred-count 0)
  (for [item items]
    (if (unpack-iterable-form? item)
        (do
          (when (not (= (len item) 2))
            (raise (SyntaxError "kernel starred assignment target needs one expression")))
          (setv starred-count (+ starred-count 1))
          (when (> starred-count 1)
            (raise (SyntaxError "kernel assignment target only supports one starred target")))
          (.append targets
                   (ast.Starred :value (store-target (get item 1))
                                :ctx (ast.Store))))
        (.append targets (store-target item))))
  targets)


(defn del-target [target]
  (cond
    (isinstance target Symbol)
      (do
        (validate-binding-symbol target "deletion")
        (ast.Name :id (mangle (str target)) :ctx (ast.Del)))
    (and (isinstance target Expression)
         (symbol-named? (expression-head target) "."))
      (del-attribute-target (cut target 1 None))
    (and (isinstance target Expression)
         (symbol-named? (expression-head target) "get"))
      (del-subscript-target (cut target 1 None))
    (and (isinstance target Expression)
         (symbol-named? (expression-head target) "cut"))
      (del-slice-target (cut target 1 None))
    (isinstance target List)
      (ast.List :elts (list (map del-target target)) :ctx (ast.Del))
    (isinstance target Tuple)
      (ast.Tuple :elts (list (map del-target target)) :ctx (ast.Del))
    True
      (raise (SyntaxError (+ "kernel can only delete symbols, attributes, subscripts, or sequences, not "
                             (repr target))))))


(defn attribute-target [args ctx]
  (when (< (len args) 2)
    (raise (SyntaxError "kernel attribute target needs an object and at least one name")))
  (compile-dot-chain args ctx False))


(defn store-attribute-target [args]
  (attribute-target args (ast.Store)))


(defn del-attribute-target [args]
  (attribute-target args (ast.Del)))


(defn subscript-target [args ctx]
  (when (< (len args) 2)
    (raise (SyntaxError "kernel subscript target needs a collection and at least one index")))
  (setv node (compile-expr (get args 0)))
  (for [offset (range 1 (len args))]
    (setv node (ast.Subscript :value node
                              :slice (compile-expr (get args offset))
                              :ctx (if (= offset (- (len args) 1))
                                       ctx
                                       (ast.Load)))))
  node)


(defn store-subscript-target [args]
  (subscript-target args (ast.Store)))


(defn del-subscript-target [args]
  (subscript-target args (ast.Del)))


(defn slice-target [args ctx]
  (when (not (in (len args) [1 2 3 4]))
    (raise (SyntaxError "kernel slice target needs a collection, optional start, optional stop, and optional step")))
  (ast.Subscript :value (compile-expr (get args 0))
                 :slice (cut-slice args)
                 :ctx ctx))


(defn store-slice-target [args]
  (slice-target args (ast.Store)))


(defn del-slice-target [args]
  (slice-target args (ast.Del)))


(defn load-symbol [symbol]
  (setv name (str symbol))
  (cond
    (in name ["None" "True" "False"])
      (ast.Constant :value (eval name))
    (= name "...")
      (ast.Constant :value Ellipsis)
    True
      (ast.Name :id (mangle name) :ctx (ast.Load))))


(defn hy-model-class [name]
  (ast.Attribute
    :value (ast.Attribute
             :value (ast.Name :id "hy" :ctx (ast.Load))
             :attr "models"
             :ctx (ast.Load))
    :attr name
    :ctx (ast.Load)))


(defn hy-model-call [name args]
  (ast.Call :func (hy-model-class name)
            :args args
            :keywords []))


(defn hy-model-keyword [name value]
  (ast.keyword :arg name :value value))


(defn hy-model-call-kwargs [name args keywords]
  (ast.Call :func (hy-model-class name)
            :args args
            :keywords keywords))


(defn pending-method-call [target-name method args]
  (ast.Expr
    :value (ast.Call
             :func (ast.Attribute
                     :value (ast.Name :id target-name :ctx (ast.Load))
                     :attr method
                     :ctx (ast.Load))
             :args args
             :keywords [])))


(defn make-sequence-node [node-class elts]
  ;; ast.Set has no `ctx` field (unlike ast.List / ast.Tuple); passing one is a
  ;; DeprecationWarning on 3.13/3.14 and an error on 3.15.
  (if (is node-class ast.Set)
      (node-class :elts elts)
      (node-class :elts elts :ctx (ast.Load))))


(defn compile-sequence-with-pending [compiled node-class]
  (for [entry compiled]
    (for [statement (get entry 0)]
      (append-pending-statement statement)))
  (make-sequence-node
    node-class
    (list (map (fn [entry]
                 (if (get entry 2)
                     (ast.Starred :value (get entry 1)
                                  :ctx (ast.Load))
                     (get entry 1)))
               compiled))))


(defn compile-expr-isolated-pending [model]
  (global PENDING-STATEMENTS)
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (try
    (do
      (setv value (compile-expr model))
      [(drain-pending-statements) value])
    (finally
      (setv PENDING-STATEMENTS outer-pending))))


(defn compile-sequence-entry [item]
  (if (unpack-iterable-form? item)
      (do
        (when (not (= (len item) 2))
          (raise (SyntaxError "kernel #* literal unpacking needs one expression")))
        (setv compiled (compile-expr-isolated-pending (get item 1)))
        [(get compiled 0) (get compiled 1) True])
      (do
        (setv compiled (compile-expr-isolated-pending item))
        [(get compiled 0) (get compiled 1) False])))


(defn compile-sequence [items node-class]
  (setv compiled [])
  (setv has-pending False)
  (for [item items]
    (setv entry (compile-sequence-entry item))
    (when (get entry 0)
      (setv has-pending True))
    (.append compiled entry))
  (if has-pending
      (compile-sequence-with-pending compiled node-class)
      (make-sequence-node
        node-class
        (list (map (fn [entry]
                     (if (get entry 2)
                         (ast.Starred :value (get entry 1)
                                      :ctx (ast.Load))
                         (get entry 1)))
                   compiled)))))


(defn compile-dict-with-pending [compiled]
  (for [entry compiled]
    (if (get entry 0)
        (for [statement (get entry 2)]
          (append-pending-statement statement))
        (do
          (for [statement (get entry 2)]
            (append-pending-statement statement))
          (for [statement (get entry 4)]
            (append-pending-statement statement)))))
  (ast.Dict :keys (list (map (fn [entry]
                               (if (get entry 0)
                                   None
                                   (get entry 3)))
                             compiled))
            :values (list (map (fn [entry]
                                 (if (get entry 0)
                                     (get entry 3)
                                     (get entry 5)))
                               compiled))))


(defn compile-dict [model]
  (setv keys [])
  (setv values [])
  (setv compiled [])
  (setv has-pending False)
  (setv items (model-list model))
  (setv index 0)
  (while (< index (len items))
    (setv key (get items index))
    (if (unpack-mapping-form? key)
        (do
          (when (not (= (len key) 2))
            (raise (SyntaxError "kernel #** dict unpacking needs one expression")))
          (setv value-compiled (compile-expr-isolated-pending (get key 1)))
          (setv pending (get value-compiled 0))
          (setv value (get value-compiled 1))
          (when pending
            (setv has-pending True))
          (.append compiled [True None pending value None None])
          (.append keys None)
          (.append values value)
          (setv index (+ index 1)))
        (do
          (when (>= (+ index 1) (len items))
            (raise (SyntaxError "kernel dict literals need key/value pairs or #** mappings")))
          (setv key-compiled (compile-expr-isolated-pending key))
          (setv key-pending (get key-compiled 0))
          (setv key-node (get key-compiled 1))
          (setv value-compiled (compile-expr-isolated-pending (get items (+ index 1))))
          (setv value-pending (get value-compiled 0))
          (setv value-node (get value-compiled 1))
          (when (or key-pending value-pending)
            (setv has-pending True))
          (.append compiled [False None key-pending key-node value-pending value-node])
          (.append keys key-node)
          (.append values value-node)
          (setv index (+ index 2)))))
  (if has-pending
      (compile-dict-with-pending compiled)
      (ast.Dict :keys keys :values values)))


(defn fstring-conversion [component]
  (if component.conversion
      (do
        (when (not (in component.conversion ["s" "r" "a"]))
          (raise (SyntaxError (+ "kernel invalid f-string conversion "
                                 (repr component.conversion)))))
        (ord component.conversion))
      -1))


(defn compile-fcomponent [component]
  (when component.is_tstring
    (raise (SyntaxError "kernel template strings are outside the current direct-kernel lane")))
  (when (= (len component) 0)
    (raise (SyntaxError "kernel f-string components need a value")))
  (setv value-node (compile-expr (get component 0)))
  (setv format-values [])
  (for [part (cut component 1 None)]
    (.append format-values
             (cond
               (isinstance part String)
                 (ast.Constant :value (str part))
               (isinstance part FComponent)
                 (compile-fcomponent part)
               True
                 (compile-expr part))))
  (ast.FormattedValue
    :value value-node
    :conversion (fstring-conversion component)
    :format_spec (if format-values
                     (ast.JoinedStr :values format-values)
                     None)))


(defn compile-fstring [model]
  (when model.is_tstring
    (raise (SyntaxError "kernel template strings are outside the current direct-kernel lane")))
  (setv values [])
  (for [part model]
    (.append values
             (cond
               (isinstance part String)
                 (ast.Constant :value (str part))
               (isinstance part FComponent)
                 (compile-fcomponent part)
               True
                 (compile-expr part))))
  (ast.JoinedStr :values values))


(defn inline-python-code [args form-name]
  (when (!= (len args) 1)
    (raise (SyntaxError (+ "kernel " form-name " needs exactly one string"))))
  (setv code (get args 0))
  (when (not (isinstance code String))
    (raise (SyntaxError (+ "kernel " form-name " needs a string"))))
  (str code))


(defn compile-py-expr [args]
  (setv code (inline-python-code args "py"))
  (try
    (do
      (setv parsed (ast.parse (+ "(" code "\n)") "<kernel:py>" "eval"))
      parsed.body)
    (except [e [SyntaxError ValueError]]
      (raise (SyntaxError (+ "Python parse error in 'py': " (str e)))))))


(defn compile-pys [args]
  (setv code (inline-python-code args "pys"))
  (try
    (do
      (setv parsed (ast.parse (textwrap.dedent code) "<kernel:pys>" "exec"))
      parsed.body)
    (except [e [SyntaxError ValueError]]
      (raise (SyntaxError (+ "Python parse error in 'pys': " (str e)))))))


(defn quote-items [model]
  (ast.List :elts (list (map quote-to-ast model))
            :ctx (ast.Load)))


(defn quote-string-call [model]
  (setv keywords [])
  (when (not (is model.brackets None))
    (.append keywords (hy-model-keyword "brackets"
                                        (ast.Constant :value model.brackets))))
  (hy-model-call-kwargs "String"
                        [(ast.Constant :value (str model))]
                        keywords))


(defn quote-fstring-call [model]
  (setv keywords [])
  (when (not (is model.brackets None))
    (.append keywords (hy-model-keyword "brackets"
                                        (ast.Constant :value model.brackets))))
  (when model.is_tstring
    (.append keywords (hy-model-keyword "is_tstring"
                                        (ast.Constant :value True))))
  (hy-model-call-kwargs "FString" [(quote-items model)] keywords))


(defn quote-fcomponent-call [model]
  (hy-model-call-kwargs
    "FComponent"
    [(quote-items model)]
    [(hy-model-keyword "conversion" (ast.Constant :value model.conversion))
     (hy-model-keyword "expression" (ast.Constant :value model.expression))
     (hy-model-keyword "is_tstring" (ast.Constant :value model.is_tstring))]))


(defn quasiquote-fstring-call [model depth]
  (setv keywords [])
  (when (not (is model.brackets None))
    (.append keywords (hy-model-keyword "brackets"
                                        (ast.Constant :value model.brackets))))
  (when model.is_tstring
    (.append keywords (hy-model-keyword "is_tstring"
                                        (ast.Constant :value True))))
  (hy-model-call-kwargs "FString" [(quasiquote-items model depth)] keywords))


(defn quasiquote-fcomponent-call [model depth]
  (hy-model-call-kwargs
    "FComponent"
    [(quasiquote-items model depth)]
    [(hy-model-keyword "conversion" (ast.Constant :value model.conversion))
     (hy-model-keyword "expression" (ast.Constant :value model.expression))
     (hy-model-keyword "is_tstring" (ast.Constant :value model.is_tstring))]))


(defn quote-to-ast [model]
  (cond
    (isinstance model Integer)
      (hy-model-call "Integer" [(ast.Constant :value (int model))])
    (isinstance model Float)
      (hy-model-call "Float" [(ast.Constant :value (float model))])
    (isinstance model Complex)
      (hy-model-call "Complex" [(ast.Constant :value (complex model))])
    (isinstance model String)
      (quote-string-call model)
    (isinstance model Bytes)
      (hy-model-call "Bytes" [(ast.Constant :value (bytes model))])
    (isinstance model FString)
      (quote-fstring-call model)
    (isinstance model FComponent)
      (quote-fcomponent-call model)
    (isinstance model Keyword)
      (hy-model-call "Keyword" [(ast.Constant :value model.name)])
    (isinstance model Symbol)
      (hy-model-call "Symbol" [(ast.Constant :value (str model))])
    (isinstance model List)
      (hy-model-call "List" [(quote-items model)])
    (isinstance model Tuple)
      (hy-model-call "Tuple" [(quote-items model)])
    (isinstance model Set)
      (hy-model-call "Set" [(quote-items model)])
    (isinstance model Dict)
      (hy-model-call "Dict" [(quote-items model)])
    (isinstance model Expression)
      (hy-model-call "Expression" [(quote-items model)])
    True (raise (SyntaxError (+ "kernel cannot quote " (repr model))))))


(defn one-item-list-ast [node]
  (ast.List :elts [node] :ctx (ast.Load)))


(defn list-call-ast [node]
  (ast.Call :func (ast.Name :id "list" :ctx (ast.Load))
            :args [node]
            :keywords []))


(defn splice-list-call-ast [node]
  (list-call-ast
    (ast.BoolOp :op (ast.Or)
                :values [node (ast.List :elts [] :ctx (ast.Load))])))


(defn concat-list-asts [parts]
  (when (= (len parts) 0)
    (return (ast.List :elts [] :ctx (ast.Load))))
  (setv node (get parts 0))
  (for [part (cut parts 1 None)]
    (setv node (ast.BinOp :left node :op (ast.Add) :right part)))
  node)


(defn quasiquote-special-form-ast [name value]
  (hy-model-call
    "Expression"
    [(ast.List :elts [(quote-to-ast (Symbol name)) value]
               :ctx (ast.Load))]))


(defn quasiquote-items [model [depth 1]]
  (setv parts [])
  (for [item model]
    (cond
      (and (special-form? item "unquote-splice")
           (= depth 1))
        (.append parts (splice-list-call-ast (compile-expr (get item 1))))
      (special-form? item "unquote-splice")
        (.append parts
                 (one-item-list-ast
                   (quasiquote-special-form-ast
                     "unquote-splice"
                     (quasiquote-to-ast (get item 1) (- depth 1)))))
      True
        (.append parts (one-item-list-ast (quasiquote-to-ast item depth)))))
  (concat-list-asts parts))


(defn quasiquote-dict-items [model [depth 1]]
  (setv parts [])
  (for [item model]
    (when (and (special-form? item "unquote-splice")
               (= depth 1))
      (raise (SyntaxError "kernel quasiquote does not support unquote-splice in dicts")))
    (.append parts (one-item-list-ast (quasiquote-to-ast item depth))))
  (concat-list-asts parts))


(defn quasiquote-to-ast [model [depth 1]]
  (cond
    (special-form? model "quasiquote")
      (hy-model-call "Expression" [(quasiquote-items model (+ depth 1))])
    (and (special-form? model "unquote") (= depth 1))
      (compile-expr (get model 1))
    (special-form? model "unquote")
      (hy-model-call "Expression" [(quasiquote-items model (- depth 1))])
    (and (special-form? model "unquote-splice") (= depth 1))
      (compile-expr (get model 1))
    (special-form? model "unquote-splice")
      (hy-model-call "Expression" [(quasiquote-items model (- depth 1))])
    (isinstance model FString)
      (quasiquote-fstring-call model depth)
    (isinstance model FComponent)
      (quasiquote-fcomponent-call model depth)
    (isinstance model List)
      (hy-model-call "List" [(quasiquote-items model depth)])
    (isinstance model Tuple)
      (hy-model-call "Tuple" [(quasiquote-items model depth)])
    (isinstance model Set)
      (hy-model-call "Set" [(quasiquote-items model depth)])
    (isinstance model Dict)
      (hy-model-call "Dict" [(quasiquote-dict-items model depth)])
    (isinstance model Expression)
      (hy-model-call "Expression" [(quasiquote-items model depth)])
    True (quote-to-ast model)))


(defn macro-module [[macros None] [local-macros None] [reader-macros None]]
  (setv module (types.ModuleType "hy_meta_kernel.macros"))
  (setv module.hy hy)
  (setv (get module.__dict__ "_hy_macros")
        (if (is macros None) {} macros))
  (setv (get module.__dict__ "_hy_local_macros")
        (if (is local-macros None) {} local-macros))
  (setv (get module.__dict__ "_hy_reader_macros")
        (if (is reader-macros None) {} reader-macros))
  (for [name ["Bytes" "Complex" "Dict" "Expression" "FComponent" "Float"
              "FString" "Integer" "Keyword" "List" "Set" "String" "Symbol"
              "Tuple"]]
    (setv (get module.__dict__ name) (get hy.models.__dict__ name)))
  module)


(defn validate-macro-params [params]
  (when (not (isinstance params List))
    (raise (SyntaxError "kernel defmacro parameters must be a vector")))
  (for [index (range (len params))]
    (setv param (get params index))
    (cond
      (symbol-named? param "*")
        (raise (SyntaxError "kernel defmacro does not support keyword-only parameters"))
      (unpack-mapping-form? param)
        (raise (SyntaxError "kernel defmacro does not support #** kwargs"))
      (and (unpack-iterable-form? param)
           (!= index (- (len params) 1)))
        (raise (SyntaxError "kernel defmacro #* parameter must be last")))))


(defn core-macro-name? [name]
  (and (hasattr builtins "_hy_macros")
       (in (mangle name) builtins._hy_macros)))


(defn warn-on-core-shadow [name]
  (when (and WARN-ON-CORE-SHADOW (core-macro-name? name))
    (warnings.warn (+ "New macro `" name "` will shadow the core macro of the same name")
                   RuntimeWarning)))


(defn pragma-version-parts [version]
  (setv parts [])
  (for [part (.split version ".")]
    (when (not (.isdigit part))
      (raise (SyntaxError "kernel pragma :hy needs a dot-separated integer version")))
    (try
      (.append parts (int part))
      (except [ValueError]
        (raise (SyntaxError "kernel pragma :hy needs a dot-separated integer version")))))
  parts)


(defn version-at-least? [have need]
  (setv index 0)
  (while (or (< index (len have)) (< index (len need)))
    (setv have-part (if (< index (len have)) (get have index) 0))
    (setv need-part (if (< index (len need)) (get need index) 0))
    (when (> have-part need-part)
      (return True))
    (when (< have-part need-part)
      (return False))
    (setv index (+ index 1)))
  True)


(defn apply-hy-version-pragma [value [module None]]
  (setv version-string None)
  (cond
    (isinstance value String)
      (setv version-string (str value))
    (not (is module None))
      (do
        (setv evaluated (hy.eval value :module module))
        (when (not (isinstance evaluated str))
          (raise (SyntaxError "kernel pragma :hy needs a version string")))
        (setv version-string evaluated))
    True
      (raise (SyntaxError "kernel pragma :hy needs a version string")))
  (setv required (pragma-version-parts version-string))
  (setv current (pragma-version-parts hy.__version__))
  (when (not (version-at-least? current required))
    (raise (SyntaxError (+ "Hy version " version-string " or later required")))))


(defn pragma-truthy? [value]
  (not (symbol-named? value "False")))


(defn apply-bracketed-templates-pragma [form value]
  (setv reader (getattr form "reader" None))
  (when reader
    (setv reader.bracketed_templates (pragma-truthy? value))))


(defn apply-pragma [form [module None]]
  (global WARN-ON-CORE-SHADOW)
  (when (= (len form) 1)
    (return None))
  (when (!= (% (- (len form) 1) 2) 0)
    (raise (SyntaxError "kernel pragma needs keyword/value pairs")))
  (setv index 1)
  (while (< index (len form))
    (setv key (get form index))
    (setv value (get form (+ index 1)))
    (when (not (isinstance key Keyword))
      (raise (SyntaxError "kernel pragma keys must be keywords")))
    (cond
      (keyword-named? key "warn-on-core-shadow")
        (setv WARN-ON-CORE-SHADOW (pragma-truthy? value))
      (keyword-named? key "hy")
        (apply-hy-version-pragma value module)
      (keyword-named? key "bracketed-templates")
        (apply-bracketed-templates-pragma form value)
      True
        (raise (SyntaxError (+ "Unknown pragma `" (str key) "`"))))
    (setv index (+ index 2))))


(defn make-macro-function [form module]
  (when (< (len form) 3)
    (raise (SyntaxError "kernel defmacro needs a name, parameter vector, and body")))
  (setv name (get form 1))
  (when (not (isinstance name Symbol))
    (raise (SyntaxError "kernel defmacro name must be a symbol")))
  (validate-macro-params (get form 2))
  (setv fn-ast
        (get (compile-defn (+ [name (get form 2)] (list (cut form 3 None)))) 0))
  (exec (compile (ast.fix-missing-locations
                   (ast.Module :body [(ast.Import :names [(ast.alias :name "hy" :asname None)])
                                      fn-ast]
                               :type_ignores []))
                 "<kernel:defmacro>"
                 "exec")
        module.__dict__)
  [(str name) (mangle (str name)) (get module.__dict__ (mangle (str name)))])


(defn install-macro [form macros module]
  (setv made (make-macro-function form module))
  (warn-on-core-shadow (get made 0))
  (setv (get macros (get made 1)) (get made 2)))


(defn dotted-macro-head? [head]
  (and (isinstance head Expression)
       (> (len head) 1)
       (symbol-named? (get head 0) ".")
       (all (map (fn [part] (isinstance part Symbol)) (cut head 1 None)))))


(defn macro-head-names [head]
  (cond
    (isinstance head Symbol)
      [(str head) (mangle (str head))]
    (dotted-macro-head? head)
      (do
        (setv parts (list (map (fn [part] (mangle (str part)))
                               (cut head 1 None))))
        [(.join "." parts)])
    True []))


(defn one-shot-require-macro [name]
  (when (not (.startswith name "hy.R."))
    (return None))
  (setv pieces (.partition (cut name (len "hy.R.") None) "."))
  (setv module-name (slashes2dots (get pieces 0)))
  (setv macro-name (get pieces 2))
  (when (= macro-name "")
    (raise (hy.errors.HyRequireError (+ "Could not require name from " module-name))))
  (try
    (setv source-module (importlib.import_module module-name))
    (except [e ImportError]
      (raise (hy.errors.HyRequireError (get e.args 0)))))
  (setv source-macros (getattr source-module "_hy_macros" {}))
  (setv source-name (mangle macro-name))
  (when (not (in source-name source-macros))
    (raise (hy.errors.HyRequireError (+ "Could not require name "
                                        source-name
                                        " from "
                                        module-name))))
  (setv macro-value (get source-macros source-name))
  (if (macro-needs-compiler? macro-value)
      (make-require-alias-macro source-name)
      macro-value))


(defn get-macro-lookup-name [arg]
  (cond
    (isinstance arg String)
      (mangle (str arg))
    (isinstance arg Symbol)
      (mangle (str arg))
    (dotted-macro-head? arg)
      (get (macro-head-names arg) 0)
    True
      (raise (NameError (+ "no such macro: " (repr arg))))))


(defn get-macro-target-form [namespace name]
  (Expression [(Symbol "get") namespace (String name)]))


(defn local-stack-list [local-stack]
  (if (is local-stack None) [] local-stack))


(defn find-local-macro-record [name local-stack]
  (for [local-macros (reversed (local-stack-list local-stack))]
    (when (in name local-macros)
      (return (get local-macros name))))
  None)


(defn make-get-macro-macro [macros [local-stack None] [reader-macros None]]
  (fn [arg1 [arg2 None]]
    (setv reader? (keyword-named? arg1 "reader"))
    (setv name (if reader? (str arg2) (get-macro-lookup-name arg1)))
    (setv local-record (if reader? None (find-local-macro-record name local-stack)))
    (cond
      local-record
        (Symbol (get local-record 1))
      (and reader? (in name (if (is reader-macros None) {} reader-macros)))
        (get-macro-target-form (Symbol "_hy_reader_macros") name)
      (and reader?
           (hasattr builtins "_hy_reader_macros")
           (in name builtins._hy_reader_macros))
        (get-macro-target-form
          (Expression [(Symbol ".")
                       (Expression [(Symbol "__import__") (String "builtins")])
                       (Symbol "_hy_reader_macros")])
          name)
      (and (not reader?) (in name macros))
        (get-macro-target-form (Symbol "_hy_macros") name)
      (and (not reader?)
           (hasattr builtins "_hy_macros")
           (in name builtins._hy_macros))
        (get-macro-target-form
          (Expression [(Symbol ".")
                       (Expression [(Symbol "__import__") (String "builtins")])
                       (Symbol "_hy_macros")])
          name)
      True
        (raise (NameError (+ "no such "
                             (if reader? "reader macro: " "macro: ")
                             (repr name)))))))


(defn find-macro [head macros [local-stack None] [reader-macros None]]
  (for [name (macro-head-names head)]
    (when (= name "get_macro")
      (return (make-get-macro-macro macros local-stack reader-macros)))
    (setv local-record (find-local-macro-record name local-stack))
    (when local-record
      (return (get local-record 0)))
    (when (in name macros)
      (return (get macros name)))
    (setv one-shot (one-shot-require-macro name))
    (when one-shot
      (return one-shot)))
  None)


(defn macro-needs-compiler? [macro]
  (and (hasattr macro "__code__")
       (> (len macro.__code__.co_varnames) 0)
       (= (get macro.__code__.co_varnames 0) "_hy_compiler")))


(defn make-require-alias-macro [target-name]
  (fn [#* args]
    (Expression (+ [(Symbol (unmangle target-name))]
                   (list args)))))


(defn local-macro-name [original]
  (+ "_hy_local_macro__"
     (.replace (.replace (mangle original) "D" "DN") "." "DD")))


(defn fresh-local-macro-storage-key [runtime-name]
  (global LOCAL-MACRO-INDEX)
  (setv key (+ runtime-name ":" (str LOCAL-MACRO-INDEX)))
  (setv LOCAL-MACRO-INDEX (+ LOCAL-MACRO-INDEX 1))
  key)


(defn local-macro-storage [module]
  (.setdefault module.__dict__ "_hy_local_macros" {}))


(defn local-macro-runtime-setv [runtime-name storage-key]
  (Expression [(Symbol "setv")
               (Symbol runtime-name)
               (Expression [(Symbol "get")
                            (Symbol "_hy_local_macros")
                            (String storage-key)])]))


(defn install-local-macro-value [name macro-value local-macros module]
  (warn-on-core-shadow name)
  (setv runtime-name (local-macro-name name))
  (setv storage-key (fresh-local-macro-storage-key runtime-name))
  (setv (get (local-macro-storage module) storage-key) macro-value)
  (setv (get local-macros (mangle name)) [macro-value runtime-name storage-key])
  (local-macro-runtime-setv runtime-name storage-key))


(defn install-local-macro [form local-macros module]
  (setv made (make-macro-function form module))
  (install-local-macro-value (get made 0) (get made 2) local-macros module))


(defn require-module-name [module]
  (setv target (compile-import-target module))
  (setv module-name (get target 0))
  (setv import-level (get target 1))
  (if (> import-level 0)
      (do
        (setv dots "")
        (for [_ (range import-level)]
          (+= dots "."))
        (+ dots (or module-name "")))
      module-name))


(defn require-assignment-list [items]
  (setv assignments [])
  (setv index 0)
  (while (< index (len items))
    (setv name (get items index))
    (when (not (isinstance name Symbol))
      (raise (SyntaxError "kernel require names must be symbols")))
    (setv alias name)
    (when (and (< (+ index 2) (len items))
               (keyword-named? (get items (+ index 1)) "as"))
      (setv alias (get items (+ index 2)))
      (when (not (isinstance alias Symbol))
        (raise (SyntaxError "kernel require aliases must be symbols")))
      (setv index (+ index 2)))
    (.append assignments [(str name) (str alias)])
    (setv index (+ index 1)))
  assignments)


(defn require-assignment-shape [module-name rest]
  (setv prefix "")
  (setv assignments "EXPORTS")
  (cond
    (is rest None)
      (setv prefix module-name)
    (symbol-named? rest "*")
      None
    (and (isinstance rest List)
         (> (len rest) 0)
         (keyword-named? (get rest 0) "as"))
      (do
        (when (!= (len rest) 2)
          (raise (SyntaxError "kernel require :as needs exactly one alias")))
        (when (not (isinstance (get rest 1) Symbol))
          (raise (SyntaxError "kernel require aliases must be symbols")))
        (setv prefix (mangle (str (get rest 1)))))
    (isinstance rest List)
      (setv assignments (require-assignment-list rest))
    True
      (raise (SyntaxError "kernel require needs *, :as alias, or a name vector")))
  [prefix assignments])


(defn require-macro-brackets-redefined? [args index]
  (and (< index (len args))
       (or (isinstance (get args index) List)
           (symbol-named? (get args index) "*")
           (keyword-named? (get args index) "macros"))))


(defn raise-require-bracket-redefinition [kind]
  (raise (hy.errors.HySyntaxError
           (+ "redefinition of ':" kind "' brackets."))))


(defn require-export-pairs [source-macros source-module assignments]
  (setv pairs [])
  (setv source-exports
        (getattr source-module
                 "_hy_export_macros"
                 (list (filter (fn [key] (not (.startswith key "_")))
                               (.keys source-macros)))))
  (for [key (.keys source-macros)]
    (when (or (= assignments "ALL")
              (in key source-exports))
      (.append pairs [key key])))
  pairs)


(defn required-macro-pairs [module-name assignments [prefix ""]]
  (setv source-module (if (= module-name "builtins")
                          builtins
                          (importlib.import_module module-name REQUIRE-PACKAGE)))
  (setv source-macros (.setdefault source-module.__dict__ "_hy_macros" {}))
  (setv out [])
  (if (not source-macros)
      (do
        (when (not (in assignments ["ALL" "EXPORTS"]))
          (for [pair assignments]
            (.extend out
                     (required-macro-pairs
                       (+ source-module.__name__ "." (mangle (get pair 0)))
                       "ALL"
                       (get pair 1))))))
      (do
        (setv pairs (if (in assignments ["ALL" "EXPORTS"])
                        (require-export-pairs source-macros source-module assignments)
                        assignments))
        (setv prefix-text (if prefix (+ prefix ".") ""))
        (for [pair pairs]
          (setv source-name (mangle (get pair 0)))
          (setv alias-name (mangle (+ prefix-text (get pair 1))))
          (when (not (in source-name source-macros))
            (raise (SyntaxError (+ "kernel require could not find macro "
                                   source-name
                                   " in "
                                   module-name))))
          (setv macro-value (get source-macros source-name))
          (.append out
                   [alias-name
                    (if (macro-needs-compiler? macro-value)
                        (make-require-alias-macro source-name)
                        macro-value)]))))
  out)


(defn install-required-macros [module-name macros assignments [prefix ""]]
  (for [pair (required-macro-pairs module-name assignments prefix)]
    (warn-on-core-shadow (get pair 0))
    (setv (get macros (get pair 0)) (get pair 1))))


(defn install-require [form macros]
  (setv args (cut form 1 None))
  (setv index 0)
  (while (< index (len args))
    (setv module (get args index))
    (setv module-name (require-module-name module))
    (setv index (+ index 1))
    (when (and (< index (len args))
               (keyword-named? (get args index) "macros"))
      (setv index (+ index 1)))
    (when (and (< index (len args))
               (keyword-named? (get args index) "readers"))
      (raise (SyntaxError "kernel require :readers is not supported yet")))
    (setv rest None)
    (when (< index (len args))
      (setv candidate (get args index))
      (cond
        (symbol-named? candidate "*")
          (do
            (setv rest candidate)
            (setv index (+ index 1)))
        (keyword-named? candidate "as")
          (do
            (when (>= (+ index 1) (len args))
              (raise (SyntaxError "kernel require :as needs an alias")))
            (setv rest (List [(Keyword "as") (get args (+ index 1))]))
            (setv index (+ index 2)))
        (isinstance candidate List)
          (do
            (setv rest candidate)
            (setv index (+ index 1)))))
    (setv shape (require-assignment-shape module-name rest))
    (when (require-macro-brackets-redefined? args index)
      (raise-require-bracket-redefinition "macros"))
    (install-required-macros module-name macros (get shape 1) (get shape 0))))


(defn defn-body-start [form]
  (setv offset 1)
  (when (and (> (len form) offset)
             (keyword-named? (get form offset) "async"))
    (setv offset (+ offset 1)))
  (when (and (> (len form) offset)
             (isinstance (get form offset) List))
    (setv offset (+ offset 1)))
  (+ offset 2))


(defn defclass-body-start [form]
  (setv offset 1)
  (when (and (> (len form) offset)
             (keyword-named? (get form offset) "tp"))
    (return (len form)))
  (when (and (> (len form) offset)
             (isinstance (get form offset) List))
    (setv offset (+ offset 1)))
  (setv body-offset (+ offset 1))
  (if (and (> (len form) body-offset)
           (isinstance (get form body-offset) List))
      (+ body-offset 1)
      (len form)))


(defn install-local-require [form local-macros module]
  (setv runtime-forms [])
  (setv args (cut form 1 None))
  (setv index 0)
  (while (< index (len args))
    (setv require-module (get args index))
    (setv module-name (require-module-name require-module))
    (setv index (+ index 1))
    (when (and (< index (len args))
               (keyword-named? (get args index) "macros"))
      (setv index (+ index 1)))
    (when (and (< index (len args))
               (keyword-named? (get args index) "readers"))
      (raise (SyntaxError "kernel local require :readers is not supported yet")))
    (setv rest None)
    (when (< index (len args))
      (setv candidate (get args index))
      (cond
        (symbol-named? candidate "*")
          (do
            (setv rest candidate)
            (setv index (+ index 1)))
        (keyword-named? candidate "as")
          (do
            (when (>= (+ index 1) (len args))
              (raise (SyntaxError "kernel require :as needs an alias")))
            (setv rest (List [(Keyword "as") (get args (+ index 1))]))
            (setv index (+ index 2)))
        (isinstance candidate List)
          (do
            (setv rest candidate)
            (setv index (+ index 1)))))
    (setv shape (require-assignment-shape module-name rest))
    (when (require-macro-brackets-redefined? args index)
      (raise-require-bracket-redefinition "macros"))
    (for [pair (required-macro-pairs module-name (get shape 1) (get shape 0))]
      (.append runtime-forms
               (install-local-macro-value (get pair 0)
                                          (get pair 1)
                                          local-macros
                                          module))))
  runtime-forms)


(defn expand-body-forms [forms macros local-stack module]
  (global WARN-ON-CORE-SHADOW)
  (setv saved-warn-on-core-shadow WARN-ON-CORE-SHADOW)
  (setv local-macros {})
  (setv body-stack (+ (local-stack-list local-stack) [local-macros]))
  (setv expanded [])
  (try
    (for [form forms]
      (cond
        (pragma-form? form)
          (apply-pragma form module)
        (defmacro-form? form)
          (.append expanded (install-local-macro form local-macros module))
        (require-form? form)
          (.extend expanded (install-local-require form local-macros module))
        (eval-when-compile-form? form)
          (compile-time-eval-forms (cut form 1 None) macros module "<kernel:body>")
        (eval-and-compile-form? form)
          (do
            (setv ewc-body (cut form 1 None))
            (compile-time-eval-forms ewc-body macros module "<kernel:body>")
            (for [body-form ewc-body]
              (.append expanded (expand-form body-form macros body-stack module))))
        True
          (.append expanded (expand-form form macros body-stack module))))
    (finally
      (setv WARN-ON-CORE-SHADOW saved-warn-on-core-shadow)))
  expanded)


(defn expand-do-forms [forms macros local-stack module]
  "Expand the forms of a `do`, which is transparent for local macros.

  A `defmacro`/`require`/`pragma`/`eval-when-compile` inside a `do` affects the
  enclosing scope (the innermost local-macro table, or the module macros at top
  level), not a fresh nested scope, matching upstream Hy where `do` leaks local
  macros to siblings and the enclosing body."
  (setv stack-list (local-stack-list local-stack))
  (setv at-top (= (len stack-list) 0))
  (setv install-macros (if at-top macros (get stack-list -1)))
  (setv expanded [])
  (for [form forms]
    (cond
      (pragma-form? form)
        (apply-pragma form module)
      (defmacro-form? form)
        (if at-top
            (install-macro form macros module)
            (.append expanded (install-local-macro form install-macros module)))
      (require-form? form)
        (if at-top
            (install-require form macros)
            (.extend expanded (install-local-require form install-macros module)))
      (eval-when-compile-form? form)
        (compile-time-eval-forms (cut form 1 None) macros module "<kernel:do>")
      (eval-and-compile-form? form)
        (do
          (setv ewc-body (cut form 1 None))
          (compile-time-eval-forms ewc-body macros module "<kernel:do>")
          (for [body-form ewc-body]
            (.append expanded (expand-form body-form macros local-stack module))))
      True
        (.append expanded (expand-form form macros local-stack module))))
  expanded)


(defn expand-defn-form [form macros local-stack module]
  (setv body-start (defn-body-start form))
  (when (> body-start (len form))
    (return form))
  (Expression (+ (list (cut form 0 body-start))
                 (expand-body-forms (cut form body-start None)
                                    macros
                                    local-stack
                                    module))))


(defn expand-defclass-form [form macros local-stack module]
  (setv body-start (defclass-body-start form))
  (setv head-items
        (list (map (fn [item] (expand-form item macros local-stack module))
                   (cut form 0 body-start))))
  (if (>= body-start (len form))
      (Expression head-items)
      (Expression (+ head-items
                     (expand-body-forms (cut form body-start None)
                                        macros
                                        local-stack
                                        module)))))


(defn expand-container [model macros container-class [local-stack None] [module None]]
  (container-class
    (list (map (fn [item] (expand-form item macros local-stack module)) model))))


(defn raise-macro-expansion-error [form error]
  (setv exc-msg (.join "" (traceback.format_exception_only (type error) error)))
  (raise (hy.errors.HyMacroExpansionError
           (+ "expanding macro " (str (get form 0)) "\n  " exc-msg)
           form)))


(defn expand-macro-call [form macro macros local-stack module]
  (try
    (setv result (as-model (macro #* (cut form 1 None))))
    (except [error Exception]
      (if (isinstance error hy.errors.HyLanguageError)
          (raise)
          (raise-macro-expansion-error form error))))
  (when (or (defmacro-form? result)
            (require-form? result)
            (pragma-form? result))
    (setv result (Expression [(Symbol "do") result])))
  (expand-form result macros local-stack module))


(defn expand-comprehension-form [form macros local-stack module]
  "Expand a comprehension, consuming `:do (defmacro ...)` as a local macro.

  A `defmacro` in a `:do` clause defines a compile-time-only local macro visible
  to later clauses and the body; it is installed into a fresh local-macro scope
  and dropped from the runtime comprehension. Comprehensions without such a
  clause expand item-by-item exactly as the generic expansion path does."
  (setv local-macros {})
  (setv body-stack (+ (local-stack-list local-stack) [local-macros]))
  (setv items (list (cut form 1 None)))
  (setv out [(get form 0)])
  (setv index 0)
  (while (< index (len items))
    (setv item (get items index))
    (if (and (keyword-named? item "do")
             (< (+ index 1) (len items))
             (defmacro-form? (get items (+ index 1))))
        (do
          (install-local-macro (get items (+ index 1)) local-macros module)
          (setv index (+ index 2)))
        (do
          (.append out (expand-form item macros body-stack module))
          (setv index (+ index 1)))))
  (Expression out))


(defn expand-form [form macros [local-stack None] [module None]]
  (cond
    (isinstance form Expression)
      (do
        (if (= (len form) 0)
            form
            (do
              (setv head (get form 0))
              (setv macro (find-macro head macros local-stack
                                      (getattr module "_hy_reader_macros" {})))
              (cond
                (symbol-named? head "quote") form
                (symbol-named? head "quasiquote") form
                (symbol-named? head "defn")
                  (expand-defn-form form macros local-stack module)
                (symbol-named? head "defclass")
                  (expand-defclass-form form macros local-stack module)
                (symbol-named? head "do")
                  (Expression (+ [(get form 0)]
                                 (expand-do-forms (cut form 1 None)
                                                  macros
                                                  local-stack
                                                  module)))
                (or (symbol-named? head "lfor")
                    (symbol-named? head "sfor")
                    (symbol-named? head "dfor")
                    (symbol-named? head "gfor"))
                  (expand-comprehension-form form macros local-stack module)
                macro
                  (expand-macro-call form macro macros local-stack module)
                True
                  (Expression
                    (list (map (fn [item] (expand-form item macros local-stack module))
                               form)))))))
    (isinstance form List) (expand-container form macros List local-stack module)
    (isinstance form Tuple) (expand-container form macros Tuple local-stack module)
    (isinstance form Set) (expand-container form macros Set local-stack module)
    (isinstance form Dict) (expand-container form macros Dict local-stack module)
    True form))


(defn compile-time-eval-forms [forms macros module filename]
  (global PENDING-STATEMENTS GENERATED-NAME-INDEX LOCAL-MACRO-INDEX FUNCTION-SCOPE-DEPTH
          WARN-ON-CORE-SHADOW
          MODULE-BINDING-NAMES FUNCTION-BINDING-STACK)
  (setv saved-pending PENDING-STATEMENTS)
  (setv saved-generated GENERATED-NAME-INDEX)
  (setv saved-local-macro-index LOCAL-MACRO-INDEX)
  (setv saved-function-depth FUNCTION-SCOPE-DEPTH)
  (setv saved-warn-on-core-shadow WARN-ON-CORE-SHADOW)
  (setv saved-module-bindings MODULE-BINDING-NAMES)
  (setv saved-function-stack FUNCTION-BINDING-STACK)
  (try
    (setv PENDING-STATEMENTS [])
    (setv GENERATED-NAME-INDEX 0)
    (setv LOCAL-MACRO-INDEX 0)
    (setv FUNCTION-SCOPE-DEPTH 0)
    (setv WARN-ON-CORE-SHADOW True)
    (setv MODULE-BINDING-NAMES [])
    (setv FUNCTION-BINDING-STACK [])
    (setv body [])
    (for [form forms]
      (.extend body (compile-form-with-pending-as-statements
                      (preserve-expanded-location
                        (expand-form form macros None module)
                        form))))
    (exec (compile (ast.fix-missing-locations
                     (ast.Module :body body :type_ignores []))
                   filename
                   "exec")
          module.__dict__)
    (finally
      (setv PENDING-STATEMENTS saved-pending)
      (setv GENERATED-NAME-INDEX saved-generated)
      (setv LOCAL-MACRO-INDEX saved-local-macro-index)
      (setv FUNCTION-SCOPE-DEPTH saved-function-depth)
      (setv WARN-ON-CORE-SHADOW saved-warn-on-core-shadow)
      (setv MODULE-BINDING-NAMES saved-module-bindings)
      (setv FUNCTION-BINDING-STACK saved-function-stack))))


(defn target-macro-dict [target-module]
  (if target-module
      (.setdefault target-module.__dict__ "_hy_macros" {})
      {}))


(defn preserve-expanded-location [expanded form]
  (if (and (model-positioned? form)
           (hasattr expanded "replace"))
      (.replace expanded form)
      expanded))


(defn prepare-forms [forms [target-module None] [filename "<kernel>"]]
  (when target-module
    (setv target-module.hy hy)
    (.setdefault target-module.__dict__ "_hy_local_macros" {})
    (.setdefault target-module.__dict__ "_hy_reader_macros" {}))
  (setv macros (target-macro-dict target-module))
  (setv local-storage (if target-module
                          (.setdefault target-module.__dict__ "_hy_local_macros" {})
                          {}))
  (setv reader-storage (if target-module
                           (.setdefault target-module.__dict__ "_hy_reader_macros" {})
                           {}))
  (setv module (macro-module macros local-storage reader-storage))
  (setv prepared [])
  (for [form forms]
    (cond
      (reader-affecting-form? form)
        (hy.eval form :module target-module)
      (pragma-form? form)
        (apply-pragma form module)
      (defmacro-form? form)
        (install-macro form macros module)
      (require-form? form)
        (install-require form macros)
      (eval-when-compile-form? form)
        (compile-time-eval-forms (cut form 1 None) macros module filename)
      (eval-and-compile-form? form)
        (do
          (setv body (cut form 1 None))
          (compile-time-eval-forms body macros module filename)
          (for [body-form body]
            (.append prepared
                     (preserve-expanded-location
                       (expand-form body-form macros None module)
                       body-form))))
      True
        (.append prepared
                 (preserve-expanded-location
                   (expand-form form macros None module)
                   form))))
  prepared)


(setv BINOPS {"+" ast.Add
              "-" ast.Sub
              "*" ast.Mult
              "/" ast.Div
              "//" ast.FloorDiv
              "%" ast.Mod
              "**" ast.Pow
              "@" ast.MatMult
              "<<" ast.LShift
              ">>" ast.RShift
              "&" ast.BitAnd
              "|" ast.BitOr
              "^" ast.BitXor})

(setv AUGOPS {"+=" ast.Add
              "-=" ast.Sub
              "*=" ast.Mult
              "/=" ast.Div
              "//=" ast.FloorDiv
              "%=" ast.Mod
              "**=" ast.Pow
              "@=" ast.MatMult
              "<<=" ast.LShift
              ">>=" ast.RShift
              "&=" ast.BitAnd
              "|=" ast.BitOr
              "^=" ast.BitXor})

(setv AUG-VALUE-OPS {"+=" "+"
                     "-=" "+"
                     "*=" "*"
                     "/=" "*"
                     "//=" "*"
                     "%=" "*"
                     "**=" "**"
                     "@=" "@"
                     "<<=" "+"
                     ">>=" "+"
                     "&=" "&"
                     "|=" "|"
                     "^=" "^"})

(setv CMPOPS {"=" ast.Eq
              "!=" ast.NotEq
              "<" ast.Lt
              "<=" ast.LtE
              ">" ast.Gt
              ">=" ast.GtE
              "in" ast.In
              "not-in" ast.NotIn
              "not_in" ast.NotIn
              "is" ast.Is
              "is-not" ast.IsNot
              "is_not" ast.IsNot})

(setv UNARY-COMPARISONS ["=" "<" "<=" ">" ">=" "is"])


(defn has-iterable-unpack? [args]
  (any (map unpack-iterable-form? args)))


(defn operator-values-list [args]
  (setv elts [])
  (for [arg args]
    (.append elts
             (if (unpack-iterable-form? arg)
                 (ast.Starred :value (compile-expr (get arg 1))
                              :ctx (ast.Load))
                 (compile-expr arg))))
  (ast.List :elts elts :ctx (ast.Load)))


(defn raise-type-error-statement [message]
  (ast.Raise
    :exc (ast.Call :func (ast.Name :id "TypeError" :ctx (ast.Load))
                   :args [(ast.Constant :value message)]
                   :keywords [])
    :cause None))


(defn values-empty-test []
  (ast.UnaryOp :op (ast.Not)
               :operand (ast.Name :id "values" :ctx (ast.Load))))


(defn values-single-test []
  (ast.Compare
    :left (ast.Call :func (ast.Name :id "len" :ctx (ast.Load))
                    :args [(ast.Name :id "values" :ctx (ast.Load))]
                    :keywords [])
    :ops [(ast.Eq)]
    :comparators [(ast.Constant :value 1)]))


(defn values-first []
  (ast.Subscript :value (ast.Name :id "values" :ctx (ast.Load))
                 :slice (ast.Constant :value 0)
                 :ctx (ast.Load)))


(defn compile-starred-binop-helper [op]
  (setv result-name "result")
  (setv item-name "value")
  (setv body [])
  (cond
    (= op "+")
      (.append body
               (ast.If :test (values-empty-test)
                       :body [(ast.Return :value (ast.Constant :value 0))]
                       :orelse []))
    (= op "*")
      (.append body
               (ast.If :test (values-empty-test)
                       :body [(ast.Return :value (ast.Constant :value 1))]
                       :orelse []))
    (= op "|")
      (.append body
               (ast.If :test (values-empty-test)
                       :body [(ast.Return :value (ast.Constant :value 0))]
                       :orelse []))
    (in op ["-" "/" "//" "%" "**" "<<" ">>" "^"])
      (do
        (.append body
                 (ast.If :test (values-empty-test)
                         :body [(raise-type-error-statement
                                  (+ "kernel operator " op " needs at least one argument"))]
                         :orelse []))
        (cond
          (= op "-")
            (.append body
                     (ast.If :test (values-single-test)
                             :body [(ast.Return
                                      :value (ast.UnaryOp :op (ast.USub)
                                                          :operand (values-first)))]
                             :orelse []))
          (= op "/")
            (.append body
                     (ast.If :test (values-single-test)
                             :body [(ast.Return
                                      :value (ast.BinOp :left (ast.Constant :value 1)
                                                        :op (ast.Div)
                                                        :right (values-first)))]
                             :orelse []))
          True
            (.append body
                     (ast.If :test (values-single-test)
                             :body [(raise-type-error-statement
                                      (+ "kernel operator " op " needs at least two arguments"))]
                             :orelse []))))
    True
      (.append body
               (ast.If :test (values-empty-test)
                       :body [(raise-type-error-statement
                                (+ "kernel operator " op " needs at least one argument"))]
                       :orelse [])))
  (if (= op "+")
      (do
        (.append body
                 (ast.If :test (values-single-test)
                         :body [(ast.Return
                                  :value (ast.UnaryOp :op (ast.UAdd)
                                                      :operand (values-first)))]
                         :orelse []))
        (.append body
                 (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                             :value (values-first)))
        (.append body
                 (ast.For
                   :target (ast.Name :id item-name :ctx (ast.Store))
                   :iter (ast.Subscript :value (ast.Name :id "values" :ctx (ast.Load))
                                        :slice (ast.Slice :lower (ast.Constant :value 1)
                                                          :upper None
                                                          :step None)
                                        :ctx (ast.Load))
                   :body [(ast.Assign
                            :targets [(ast.Name :id result-name :ctx (ast.Store))]
                            :value (ast.BinOp
                                     :left (ast.Name :id result-name :ctx (ast.Load))
                                     :op (ast.Add)
                                     :right (ast.Name :id item-name :ctx (ast.Load))))]
                   :orelse [])))
      (if (= op "**")
          (do
            (.append body
                     (ast.Assign
                       :targets [(ast.Name :id result-name :ctx (ast.Store))]
                       :value (ast.Subscript
                                :value (ast.Name :id "values" :ctx (ast.Load))
                                :slice (ast.Constant :value -1)
                                :ctx (ast.Load))))
            (.append body
                     (ast.For
                       :target (ast.Name :id item-name :ctx (ast.Store))
                       :iter (ast.Call
                               :func (ast.Name :id "reversed" :ctx (ast.Load))
                               :args [(ast.Subscript
                                        :value (ast.Name :id "values" :ctx (ast.Load))
                                        :slice (ast.Slice :lower None
                                                          :upper (ast.Constant :value -1)
                                                          :step None)
                                        :ctx (ast.Load))]
                               :keywords [])
                       :body [(ast.Assign
                                :targets [(ast.Name :id result-name :ctx (ast.Store))]
                                :value (ast.BinOp
                                         :left (ast.Name :id item-name :ctx (ast.Load))
                                         :op (ast.Pow)
                                         :right (ast.Name :id result-name :ctx (ast.Load))))]
                       :orelse [])))
          (do
            (.append body
                     (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                                 :value (values-first)))
            (.append body
                     (ast.For
                       :target (ast.Name :id item-name :ctx (ast.Store))
                       :iter (ast.Subscript :value (ast.Name :id "values" :ctx (ast.Load))
                                            :slice (ast.Slice :lower (ast.Constant :value 1)
                                                              :upper None
                                                              :step None)
                                            :ctx (ast.Load))
                       :body [(ast.Assign
                                :targets [(ast.Name :id result-name :ctx (ast.Store))]
                                :value (ast.BinOp
                                         :left (ast.Name :id result-name :ctx (ast.Load))
                                         :op ((get BINOPS op))
                                         :right (ast.Name :id item-name :ctx (ast.Load))))]
                       :orelse [])))))
  (.append body (ast.Return :value (ast.Name :id result-name :ctx (ast.Load))))
  body)


(defn compile-starred-operator-call [args helper-prefix helper-body]
  (setv values-name (fresh-generated-name "__hy_meta_operator_values_"))
  (setv helper-name (fresh-generated-name helper-prefix))
  (setv values-node (operator-values-list args))
  (for [statement (drain-pending-statements)]
    (append-pending-statement statement))
  (append-pending-statement
    (ast.Assign :targets [(ast.Name :id values-name :ctx (ast.Store))]
                :value values-node))
  (append-pending-statement
    (ast.FunctionDef
      :name helper-name
      :args (ast.arguments :posonlyargs []
                           :args [(ast.arg :arg "values" :annotation None)]
                           :vararg None
                           :kwonlyargs []
                           :kw_defaults []
                           :kwarg None
                           :defaults [])
      :body helper-body
      :decorator_list []
      :returns None
      :type_comment None))
  (ast.Call :func (ast.Name :id helper-name :ctx (ast.Load))
            :args [(ast.Name :id values-name :ctx (ast.Load))]
            :keywords []))


(defn compile-starred-binop [op args]
  (compile-starred-operator-call
    args
    "__hy_meta_binop_"
    (compile-starred-binop-helper op)))


(defn compile-binop [op args]
  (when (and (= (len args) 0) (not (in op ["+" "*" "|"])))
    (raise (SyntaxError (+ "kernel operator " op " needs at least one argument"))))
  (when (and (= (len args) 1)
             (not (has-iterable-unpack? args))
             (in op ["//" "%" "**" "<<" ">>" "^"]))
    (raise (SyntaxError (+ "kernel operator " op " needs at least two arguments"))))
  (when (has-iterable-unpack? args)
    (return (compile-starred-binop op args)))
  (when (and (= op "+") (= (len args) 0))
    (return (ast.Constant :value 0)))
  (when (and (= op "*") (= (len args) 0))
    (return (ast.Constant :value 1)))
  (when (and (= op "|") (= (len args) 0))
    (return (ast.Constant :value 0)))
  (when (and (= op "+") (= (len args) 1))
    (return (ast.UnaryOp :op (ast.UAdd) :operand (compile-expr (get args 0)))))
  (when (and (= op "-") (= (len args) 1))
    (return (ast.UnaryOp :op (ast.USub) :operand (compile-expr (get args 0)))))
  (when (and (= op "/") (= (len args) 1))
    (return
      (ast.BinOp :left (ast.Constant :value 1)
                 :op (ast.Div)
                 :right (compile-expr (get args 0)))))
  (when (and (= op "**") (> (len args) 1))
    (setv values [])
    (for [arg args]
      (.append values (compile-expr arg)))
    (setv node (get values (- (len values) 1)))
    (for [arg (reversed (cut values 0 -1))]
      (setv node (ast.BinOp :left arg
                            :op (ast.Pow)
                            :right node)))
    (return node))
  (setv node (compile-expr (get args 0)))
  (for [arg (cut args 1 None)]
    (setv node
          (ast.BinOp :left node
                     :op ((get BINOPS op))
                     :right (compile-expr arg))))
  node)


(defn compile-starred-compare-helper [op]
  (setv body [])
  (.append body
           (ast.If :test (values-empty-test)
                   :body [(raise-type-error-statement
                            (+ "kernel comparison " op " needs at least one argument"))]
                   :orelse []))
  (.append body
           (ast.If :test (values-single-test)
                   :body [(if (in op UNARY-COMPARISONS)
                              (ast.Return :value (ast.Constant :value True))
                              (raise-type-error-statement
                                (+ "kernel comparison " op " needs at least two arguments")))]
                   :orelse []))
  (.append body
           (ast.Assign :targets [(ast.Name :id "left" :ctx (ast.Store))]
                       :value (values-first)))
  (.append body
           (ast.For
             :target (ast.Name :id "right" :ctx (ast.Store))
             :iter (ast.Subscript :value (ast.Name :id "values" :ctx (ast.Load))
                                  :slice (ast.Slice :lower (ast.Constant :value 1)
                                                    :upper None
                                                    :step None)
                                  :ctx (ast.Load))
             :body [(ast.If
                      :test (ast.UnaryOp
                              :op (ast.Not)
                              :operand (ast.Compare
                                         :left (ast.Name :id "left" :ctx (ast.Load))
                                         :ops [((get CMPOPS op))]
                                         :comparators [(ast.Name :id "right" :ctx (ast.Load))]))
                      :body [(ast.Return :value (ast.Constant :value False))]
                      :orelse [])
                    (ast.Assign
                      :targets [(ast.Name :id "left" :ctx (ast.Store))]
                      :value (ast.Name :id "right" :ctx (ast.Load)))]
             :orelse []))
  (.append body (ast.Return :value (ast.Constant :value True)))
  body)


(defn compile-compare [op args]
  (when (has-iterable-unpack? args)
    (return
      (compile-starred-operator-call
        args
        "__hy_meta_compare_"
        (compile-starred-compare-helper op))))
  (when (= (len args) 0)
    (raise (SyntaxError (+ "kernel comparison " op " needs at least one argument"))))
  (when (= (len args) 1)
    (if (in op UNARY-COMPARISONS)
        (do
          (setv value (compile-expr (get args 0)))
          (for [statement (drain-pending-statements)]
            (append-pending-statement statement))
          (append-pending-statement (ast.Expr :value value))
          (return (ast.Constant :value True)))
        (raise (SyntaxError (+ "kernel comparison " op " needs at least two arguments")))))
  (setv left (compile-expr (get args 0)))
  (setv ops [])
  (setv comparators [])
  (for [arg (cut args 1 None)]
    (.append ops ((get CMPOPS op)))
    (.append comparators (compile-expr arg)))
  (ast.Compare :left left
               :ops ops
               :comparators comparators))


(defn chainc-op [model]
  (when (not (isinstance model Symbol))
    (raise (SyntaxError "kernel chainc operators must be symbols")))
  (setv op (str model))
  (when (not (in op CMPOPS))
    (raise (SyntaxError (+ "kernel chainc does not support operator " op))))
  ((get CMPOPS op)))


(defn compile-chainc-expr [args]
  (global PENDING-STATEMENTS)
  (when (or (< (len args) 3) (= (% (len args) 2) 0))
    (raise (SyntaxError "kernel chainc needs operand/operator/operand clauses")))
  (setv result-name (fresh-generated-name "__hy_meta_chainc_result_"))
  (setv left-name (fresh-generated-name "__hy_meta_chainc_left_"))
  (setv right-name (fresh-generated-name "__hy_meta_chainc_right_"))
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv left-value (compile-expr (get args 0)))
  (setv statements (drain-pending-statements))
  (.append statements
           (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                       :value (ast.Constant :value True)))
  (.append statements
           (ast.Assign :targets [(ast.Name :id left-name :ctx (ast.Store))]
                       :value left-value))
  (setv current-body statements)
  (try
    (do
      (setv index 1)
      (while (< index (len args))
        (setv op-node (chainc-op (get args index)))
        (setv PENDING-STATEMENTS [])
        (setv right-value (compile-expr (get args (+ index 1))))
        (for [statement (drain-pending-statements)]
          (.append current-body statement))
        (.append current-body
                 (ast.Assign :targets [(ast.Name :id right-name :ctx (ast.Store))]
                             :value right-value))
        (setv branch
              (ast.If
                :test (ast.Compare
                        :left (ast.Name :id left-name :ctx (ast.Load))
                        :ops [op-node]
                        :comparators [(ast.Name :id right-name :ctx (ast.Load))])
                :body []
                :orelse [(ast.Assign
                           :targets [(ast.Name :id result-name :ctx (ast.Store))]
                           :value (ast.Constant :value False))]))
        (.append current-body branch)
        (.append branch.body
                 (ast.Assign :targets [(ast.Name :id left-name :ctx (ast.Store))]
                             :value (ast.Name :id right-name :ctx (ast.Load))))
        (setv current-body branch.body)
        (setv index (+ index 2))))
    (finally
      (setv PENDING-STATEMENTS outer-pending)))
  (for [statement statements]
    (append-pending-statement statement))
  (ast.Name :id result-name :ctx (ast.Load)))


(defn compile-starred-boolop-helper [op]
  (setv body [])
  (cond
    (= op "and")
      (do
        (.append body
                 (ast.If :test (values-empty-test)
                         :body [(ast.Return :value (ast.Constant :value True))]
                         :orelse []))
        (.append body
                 (ast.Assign :targets [(ast.Name :id "result" :ctx (ast.Store))]
                             :value (ast.Constant :value True)))
        (.append body
                 (ast.For
                   :target (ast.Name :id "value" :ctx (ast.Store))
                   :iter (ast.Name :id "values" :ctx (ast.Load))
                   :body [(ast.Assign
                            :targets [(ast.Name :id "result" :ctx (ast.Store))]
                            :value (ast.Name :id "value" :ctx (ast.Load)))
                          (ast.If
                            :test (ast.UnaryOp :op (ast.Not)
                                               :operand (ast.Name :id "value" :ctx (ast.Load)))
                            :body [(ast.Break)]
                            :orelse [])]
                   :orelse [])))
    True
      (do
        (.append body
                 (ast.If :test (values-empty-test)
                         :body [(ast.Return :value (ast.Constant :value None))]
                         :orelse []))
        (.append body
                 (ast.Assign :targets [(ast.Name :id "result" :ctx (ast.Store))]
                             :value (ast.Constant :value None)))
        (.append body
                 (ast.For
                   :target (ast.Name :id "value" :ctx (ast.Store))
                   :iter (ast.Name :id "values" :ctx (ast.Load))
                   :body [(ast.Assign
                            :targets [(ast.Name :id "result" :ctx (ast.Store))]
                            :value (ast.Name :id "value" :ctx (ast.Load)))
                          (ast.If
                            :test (ast.Name :id "value" :ctx (ast.Load))
                            :body [(ast.Break)]
                            :orelse [])]
                   :orelse []))))
  (.append body (ast.Return :value (ast.Name :id "result" :ctx (ast.Load))))
  body)


(defn compile-starred-boolop [op args]
  (compile-starred-operator-call
    args
    "__hy_meta_boolop_"
    (compile-starred-boolop-helper op)))


(defn compile-if-expr [args]
  (when (not (= (len args) 3))
    (raise (SyntaxError "kernel if expressions need test, then, and else forms")))
  (setv test (compile-expr (get args 0)))
  (setv test-pending (drain-pending-statements))
  (setv body-form (get args 1))
  (setv else-form (get args 2))
  (setv branch-has-statements False)
  (when (statement-form? body-form)
    (setv branch-has-statements True))
  (when (statement-form? else-form)
    (setv branch-has-statements True))
  (when branch-has-statements
    (setv result-name (fresh-generated-name "__hy_meta_if_result_"))
    (setv body-statements
          (compile-isolated-value-assignment-body [body-form] result-name))
    (setv else-statements
          (compile-isolated-value-assignment-body [else-form] result-name))
    (for [statement test-pending]
      (append-pending-statement statement))
    (append-pending-statement
      (ast.If
        :test test
        :body body-statements
        :orelse else-statements))
    (return (ast.Name :id result-name :ctx (ast.Load))))
  (setv body-value (compile-expr body-form))
  (setv body-pending (drain-pending-statements))
  (setv else-value (compile-expr else-form))
  (setv else-pending (drain-pending-statements))
  (setv has-pending False)
  (when test-pending
    (setv has-pending True))
  (when body-pending
    (setv has-pending True))
  (when else-pending
    (setv has-pending True))
  (when has-pending
    (setv result-name (fresh-generated-name "__hy_meta_if_result_"))
    (for [statement test-pending]
      (append-pending-statement statement))
    (append-pending-statement
      (ast.If
        :test test
        :body (+ body-pending
                 [(ast.Assign
                    :targets [(ast.Name :id result-name :ctx (ast.Store))]
                    :value body-value)])
        :orelse (+ else-pending
                   [(ast.Assign
                      :targets [(ast.Name :id result-name :ctx (ast.Store))]
                      :value else-value)])))
    (return (ast.Name :id result-name :ctx (ast.Load))))
  (ast.IfExp :test test
             :body body-value
             :orelse else-value))


(defn boolop-continue-test [op result-name]
  (setv result (ast.Name :id result-name :ctx (ast.Load)))
  (if (= op "and")
      result
      (ast.UnaryOp :op (ast.Not) :operand result)))


(defn boolop-values-expr [op values]
  (if (= (len values) 1)
      (get values 0)
      (ast.BoolOp :op (if (= op "and") (ast.And) (ast.Or))
                  :values values)))


(defn boolop-append-segment-assignment [op compiled start-index result-name body]
  (setv values [])
  (setv index start-index)
  (while (< index (len compiled))
    (setv item (get compiled index))
    (when (and (> index start-index) (get item 0))
      (break))
    (.append values (get item 1))
    (setv index (+ index 1)))
  (.append body
           (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                       :value (boolop-values-expr op values)))
  index)


(defn compile-boolop-with-pending [op compiled]
  (setv result-name (fresh-generated-name "__hy_meta_boolop_result_"))
  (setv first (get compiled 0))
  (for [statement (get first 0)]
    (append-pending-statement statement))
  (setv initial-body [])
  (setv index (boolop-append-segment-assignment op compiled 0 result-name initial-body))
  (for [statement initial-body]
    (append-pending-statement statement))
  (while (< index (len compiled))
    (setv branch
          (ast.If :test (boolop-continue-test op result-name)
                  :body []
                  :orelse []))
    (setv item (get compiled index))
    (append-pending-statement branch)
    (.extend branch.body (get item 0))
    (setv index (boolop-append-segment-assignment op compiled index
                                                  result-name branch.body)))
  (ast.Name :id result-name :ctx (ast.Load)))


(defn compile-boolop [op args]
  (when (has-iterable-unpack? args)
    (return (compile-starred-boolop op args)))
  (when (= (len args) 0)
    (return (ast.Constant :value (= op "and"))))
  (when (= (len args) 1)
    (return (compile-expr (get args 0))))
  (setv compiled [])
  (setv has-pending False)
  (for [arg args]
    (setv value (compile-expr arg))
    (setv pending (drain-pending-statements))
    (when pending
      (setv has-pending True))
    (.append compiled [pending value]))
  (if has-pending
      (compile-boolop-with-pending op compiled)
      (ast.BoolOp :op (if (= op "and") (ast.And) (ast.Or))
                  :values (list (map (fn [item] (get item 1))
                                     compiled)))))


(defn compile-not-expr [args]
  (when (not (= (len args) 1))
    (raise (SyntaxError "kernel not needs exactly one expression")))
  (ast.UnaryOp :op (ast.Not) :operand (compile-expr (get args 0))))


(defn compile-setx-expr [args]
  (when (not (= (len args) 2))
    (raise (SyntaxError "kernel setx needs a symbol and value expression")))
  (setv target (get args 0))
  (when (not (isinstance target Symbol))
    (raise (SyntaxError "kernel setx target must be a symbol")))
  (validate-binding-symbol target "setx")
  (ast.NamedExpr :target (ast.Name :id (mangle (str target)) :ctx (ast.Store))
                 :value (compile-expr (get args 1))))


(defn collect-setx-binding-names [model names]
  (cond
    (isinstance model Expression)
      (do
        (when (> (len model) 0)
          (setv head (get model 0))
          (setv head-name (if (isinstance head Symbol) (str head) None))
          (cond
            (and (symbol-named? head "setx")
                 (> (len model) 1)
                 (isinstance (get model 1) Symbol))
              (append-unique-name names (mangle (str (get model 1))))
            (symbol-named? head "setv")
              (do
                (setv index 1)
                (while (< index (len model))
                  (if (keyword-named? (get model index) "chain")
                      (do
                        (when (< (+ index 1) (len model))
                          (for [target (get model (+ index 1))]
                            (append-target-binding-names target names)))
                        (when (< (+ index 2) (len model))
                          (collect-setx-binding-names (get model (+ index 2)) names))
                        (setv index (+ index 3)))
                      (do
                        (append-target-binding-names (get model index) names)
                        (when (< (+ index 1) (len model))
                          (collect-setx-binding-names (get model (+ index 1)) names))
                        (setv index (+ index 2))))))
            (symbol-named? head "annotate")
              (do
                (when (> (len model) 1)
                  (append-target-binding-names (get model 1) names))
                (for [item (cut model 2 None)]
                  (collect-setx-binding-names item names)))
            (and head-name (in head-name AUGOPS))
              (do
                (when (> (len model) 1)
                  (append-target-binding-names (get model 1) names))
                (for [item (cut model 2 None)]
                  (collect-setx-binding-names item names)))
            (symbol-named? head "del")
              (for [item (cut model 1 None)]
                (append-target-binding-names item names))
            (or (symbol-named? head "quote")
                (symbol-named? head "quasiquote")
                (symbol-named? head "fn")
                (symbol-named? head "defn")
                (symbol-named? head "defclass"))
              None
            True
              (for [item model]
                (collect-setx-binding-names item names)))))
    (or (isinstance model List)
        (isinstance model Tuple)
        (isinstance model Set)
        (isinstance model FString)
        (isinstance model FComponent))
      (for [item model]
        (collect-setx-binding-names item names))
    (isinstance model Dict)
      (for [item (model-list model)]
        (collect-setx-binding-names item names))
    True None))


(defn compile-invert-expr [args]
  (when (not (= (len args) 1))
    (raise (SyntaxError "kernel invert needs exactly one expression")))
  (ast.UnaryOp :op (ast.Invert) :operand (compile-expr (get args 0))))


(defn compile-await-expr [args]
  (when (not (= (len args) 1))
    (raise (SyntaxError "kernel await needs exactly one expression")))
  (ast.Await :value (compile-expr (get args 0))))


(defn compile-when-expr [args]
  (when (= (len args) 0)
    (raise (SyntaxError "kernel when needs a test expression")))
  (compile-if-expr [(get args 0)
                    (Expression (+ [(Symbol "do")] (list (cut args 1 None))))
                    (Symbol "None")]))


(defn compile-cond-expr [args]
  (when (% (len args) 2)
    (raise (SyntaxError "kernel cond needs test/result pairs")))
  (setv compiled [])
  (setv has-pending False)
  (setv index 0)
  (while (< index (len args))
    (setv test (compile-expr (get args index)))
    (setv test-pending (drain-pending-statements))
    (setv value (compile-expr (get args (+ index 1))))
    (setv value-pending (drain-pending-statements))
    (when (or test-pending value-pending)
      (setv has-pending True))
    (.append compiled [test-pending test value-pending value])
    (setv index (+ index 2)))
  (when has-pending
    (setv result-name (fresh-generated-name "__hy_meta_cond_result_"))
    (append-pending-statement
      (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                  :value (ast.Constant :value None)))
    (setv tail [])
    (for [index (range (- (len compiled) 1) -1 -1)]
      (setv item (get compiled index))
      (setv tail
            (+ (get item 0)
               [(ast.If
                  :test (get item 1)
                  :body (+ (get item 2)
                           [(ast.Assign
                              :targets [(ast.Name :id result-name :ctx (ast.Store))]
                              :value (get item 3))])
                  :orelse tail)])))
    (for [statement tail]
      (append-pending-statement statement))
    (return (ast.Name :id result-name :ctx (ast.Load))))
  (setv node (ast.Constant :value None))
  (for [index (range (- (len compiled) 1) -1 -1)]
    (setv item (get compiled index))
    (setv node
          (ast.IfExp :test (get item 1)
                     :body (get item 3)
                     :orelse node)))
  node)


(defn compile-do-expr [args]
  (when (= (len args) 0)
    (return (ast.Constant :value None)))
  (when (= (len args) 1)
    (return (compile-expr (get args 0))))
  (setv body (compile-statement-body (cut args 0 -1)))
  (setv value (compile-expr (get args -1)))
  (setv value-pending (drain-pending-statements))
  (for [statement body]
    (append-pending-statement statement))
  (for [statement value-pending]
    (append-pending-statement statement))
  value)


(defn compile-quote-expr [args]
  (when (not (= (len args) 1))
    (raise (SyntaxError "kernel quote needs exactly one form")))
  (quote-to-ast (get args 0)))


(defn compile-quasiquote-expr [args]
  (when (not (= (len args) 1))
    (raise (SyntaxError "kernel quasiquote needs exactly one form")))
  (quasiquote-to-ast (get args 0)))


(defn collect-arg-annotations [arguments returns]
  (setv keys [])
  (setv values [])
  (for [arg (+ arguments.posonlyargs arguments.args arguments.kwonlyargs)]
    (when (not (is arg.annotation None))
      (.append keys (ast.Constant :value arg.arg))
      (.append values arg.annotation)
      (setv arg.annotation None)))
  (when (and arguments.vararg (not (is arguments.vararg.annotation None)))
    (.append keys (ast.Constant :value arguments.vararg.arg))
    (.append values arguments.vararg.annotation)
    (setv arguments.vararg.annotation None))
  (when (and arguments.kwarg (not (is arguments.kwarg.annotation None)))
    (.append keys (ast.Constant :value arguments.kwarg.arg))
    (.append values arguments.kwarg.annotation)
    (setv arguments.kwarg.annotation None))
  (when (not (is returns None))
    (.append keys (ast.Constant :value "return"))
    (.append values returns))
  (if keys (ast.Dict :keys keys :values values) None))


(defn annotate-function-expr [fn-expr annotations]
  (if annotations
      (ast.Call
        :func (ast.Lambda
                :args (ast.arguments
                        :posonlyargs []
                        :args [(ast.arg :arg "__hy_meta_fn" :annotation None)]
                        :vararg None
                        :kwonlyargs []
                        :kw_defaults []
                        :kwarg None
                        :defaults [])
                :body (ast.BoolOp
                        :op (ast.Or)
                        :values [(ast.Call
                                   :func (ast.Name :id "setattr" :ctx (ast.Load))
                                   :args [(ast.Name :id "__hy_meta_fn" :ctx (ast.Load))
                                          (ast.Constant :value "__annotations__")
                                          annotations]
                                   :keywords [])
                                 (ast.Name :id "__hy_meta_fn" :ctx (ast.Load))]))
        :args [fn-expr]
        :keywords [])
      fn-expr))


(defn fn-docstring-body? [body-forms]
  (and (> (len body-forms) 1)
       (isinstance (get body-forms 0) String)))


(defn unsupported-type-parameters-error [form-name]
  (raise (SyntaxError
           (+ "kernel " form-name
              " :tp type parameters are outside the current direct-kernel lane"))))


(defn compile-fn-expr [args]
  (when (< (len args) 1)
    (raise (SyntaxError "kernel fn needs a parameter vector")))
  (setv is-async False)
  (when (keyword-named? (get args 0) "async")
    (setv is-async True)
    (setv args (cut args 1 None)))
  (when (and (> (len args) 0) (keyword-named? (get args 0) "tp"))
    (unsupported-type-parameters-error "fn"))
  (when (< (len args) 1)
    (raise (SyntaxError "kernel fn needs a parameter vector")))
  (setv params (get args 0))
  (setv returns None)
  (when (annotation-form? params)
    (setv returns (compile-expr (annotation-type params)))
    (setv params (annotation-target params)))
  (setv body-forms (cut args 1 None))
  (setv prepared (prepare-arguments params))
  (setv fn-args (get prepared 0))
  (setv annotations (collect-arg-annotations fn-args returns))
  (setv local-bindings (collect-function-binding-names params body-forms))
  (when is-async
    (setv fn-name (fresh-generated-name "__hy_meta_fn_"))
    (append-pending-statement
      (ast.AsyncFunctionDef :name fn-name
                            :args fn-args
                            :body (+ (get prepared 3)
                                     (compile-function-body
                                       body-forms
                                       (not (any (map yield-form? body-forms)))
                                       local-bindings))
                            :decorator_list []
                            :returns None
                            :type_comment None))
    (return (annotate-function-expr
              (ast.Name :id fn-name :ctx (ast.Load))
              annotations)))
  (if (or (any (map statement-form? body-forms))
          (fn-docstring-body? body-forms))
      (do
        (setv fn-name (fresh-generated-name "__hy_meta_fn_"))
        (append-pending-statement
          (ast.FunctionDef :name fn-name
                           :args fn-args
                           :body (+ (get prepared 3)
                                    (compile-function-body body-forms True local-bindings))
                           :decorator_list []
                           :returns None
                           :type_comment None))
        (annotate-function-expr (ast.Name :id fn-name :ctx (ast.Load)) annotations))
      (do
        (setv body (compile-do-expr body-forms))
        (setv nested-pending (drain-pending-statements))
        (if nested-pending
            (do
              (setv fn-name (fresh-generated-name "__hy_meta_fn_"))
              (append-pending-statement
                (ast.FunctionDef :name fn-name
                                 :args fn-args
                                 :body (+ (get prepared 3)
                                          nested-pending
                                          [(ast.Return :value body)])
                                 :decorator_list []
                                 :returns None
                                 :type_comment None))
              (annotate-function-expr (ast.Name :id fn-name :ctx (ast.Load)) annotations))
            (do
              (when (get prepared 1)
                (setv body
                      (ast.Call :func (ast.Lambda :args (compile-arguments (List (get prepared 1)))
                                                  :body body)
                                :args (get prepared 2)
                                :keywords [])))
              (annotate-function-expr (ast.Lambda :args fn-args :body body) annotations))))))


(defn sequence-ref [value index]
  (ast.Subscript :value value
                 :slice (ast.Constant :value index)
                 :ctx (ast.Load)))


(defn sequence-slice [value lower upper]
  (ast.Subscript :value value
                 :slice (ast.Slice :lower (ast.Constant :value lower)
                                   :upper (if (is upper None)
                                              None
                                              (ast.Constant :value upper))
                                   :step None)
                 :ctx (ast.Load)))


(defn list-call [value]
  (ast.Call :func (ast.Name :id "list" :ctx (ast.Load))
            :args [value]
            :keywords []))


(defn collect-sequence-destructure-bindings [target value names values]
  (setv star-index None)
  (for [index (range (len target))]
    (when (unpack-iterable-form? (get target index))
      (when (not (is star-index None))
        (raise (SyntaxError "kernel destructuring only supports one starred target")))
      (setv star-index index)))
  (if (is star-index None)
      (for [index (range (len target))]
        (collect-destructure-bindings
          (get target index)
          (sequence-ref value index)
          names
          values))
      (do
        (for [index (range star-index)]
          (collect-destructure-bindings
            (get target index)
            (sequence-ref value index)
            names
            values))
        (setv after-count (- (len target) star-index 1))
        (setv star-target (get (get target star-index) 1))
        (collect-destructure-bindings
          star-target
          (list-call
            (sequence-slice value
                            star-index
                            (if (= after-count 0) None (- after-count))))
          names
          values)
        (for [offset (range after-count)]
          (collect-destructure-bindings
            (get target (+ star-index 1 offset))
            (sequence-ref value (- offset after-count))
            names
            values)))))


(defn collect-destructure-bindings [target value names values]
  (cond
    (isinstance target Symbol)
      (do
        (.append names target)
        (.append values value))
    (annotation-form? target)
      (do
        (when (not (isinstance (annotation-target target) Symbol))
          (raise (SyntaxError "kernel annotated destructuring targets need symbol names")))
        (.append names target)
        (.append values value))
    (or (isinstance target List) (isinstance target Tuple))
      (collect-sequence-destructure-bindings target value names values)
    True
      (raise (SyntaxError (+ "kernel destructuring targets must contain symbols, not "
                             (repr target))))))


(defn let-name-key [name]
  (cond
    (isinstance name Symbol)
      (mangle (str name))
    (annotation-form? name)
      (let [target (annotation-target name)]
        (when (not (isinstance target Symbol))
          (raise (SyntaxError "kernel annotated let bindings need symbol names")))
        (mangle (str target)))
    True
      (raise (SyntaxError "kernel let visible names must be symbols or annotations"))))


(defn let-load-name [name]
  (if (annotation-form? name)
      (load-symbol (annotation-target name))
      (load-symbol name)))


(defn active-name-position [keys key]
  (for [index (range (len keys))]
    (when (= (get keys index) key)
      (return index)))
  None)


(defn upsert-active-name [names keys name]
  (setv key (let-name-key name))
  (setv position (active-name-position keys key))
  (if (is position None)
      (do
        (.append keys key)
        (.append names name))
      (setv (get names position) name)))


(defn collect-let-target-names [target names]
  (cond
    (or (isinstance target Symbol)
        (annotation-form? target))
      (.append names target)
    (unpack-iterable-form? target)
      (collect-let-target-names (get target 1) names)
    (or (isinstance target List) (isinstance target Tuple))
      (for [item target]
        (collect-let-target-names item names))
    True
      (raise (SyntaxError "kernel let binding names must be symbols or sequences"))))


(defn append-unique-name [names name]
  (when (not (in name names))
    (.append names name)))


(defn append-target-binding-names [target names]
  (cond
    (isinstance target Symbol)
      (when (not (symbol-named? target "_"))
        (append-unique-name names (mangle (str target))))
    (annotation-form? target)
      (append-target-binding-names (annotation-target target) names)
    (unpack-iterable-form? target)
      (append-target-binding-names (get target 1) names)
    (or (isinstance target List) (isinstance target Tuple))
      (for [item target]
        (append-target-binding-names item names))
    True None))


(defn append-for-binding-names [bindings names]
  (when (isinstance bindings List)
    (setv index 0)
    (while (< index (len bindings))
      (when (keyword-named? (get bindings index) "async")
        (setv index (+ index 1)))
      (when (< index (len bindings))
        (append-target-binding-names (get bindings index) names))
      (setv index (+ index 2)))))


(defn append-with-binding-names [managers names]
  (when (and (isinstance managers List) (> (len managers) 1))
    (setv index 0)
    (while (< index (len managers))
      (when (keyword-named? (get managers index) "async")
        (setv index (+ index 1)))
      (when (< index (len managers))
        (append-target-binding-names (get managers index) names))
      (setv index (+ index 2)))))


(defn append-import-binding-name [names forced name]
  (append-unique-name names name)
  (append-unique-name forced name))


(defn append-import-bound-names [args names forced]
  (setv index 0)
  (while (< index (len args))
    (do
      (setv module (get args index))
      (setv target (compile-import-target module))
      (setv module-name (get target 0))
      (setv import-level (get target 1))
      (setv index (+ index 1))
      (cond
        (and (< index (len args))
             (symbol-named? (get args index) "*"))
          (setv index (+ index 1))
        (and (< index (len args))
             (keyword-named? (get args index) "as"))
          (do
            (when (< (+ index 1) (len args))
              (setv alias (get args (+ index 1)))
              (when (isinstance alias Symbol)
                (append-import-binding-name names forced (mangle (str alias)))))
            (setv index (+ index 2)))
        (and (< index (len args))
             (isinstance (get args index) List))
          (do
            (setv import-names (get args index))
            (setv item-index 0)
            (while (< item-index (len import-names))
              (do
                (setv name (get import-names item-index))
                (when (isinstance name Symbol)
                  (setv bound-name (mangle (str name)))
                  (when (and (< (+ item-index 2) (len import-names))
                             (keyword-named? (get import-names (+ item-index 1)) "as"))
                    (setv alias (get import-names (+ item-index 2)))
                    (when (isinstance alias Symbol)
                      (setv bound-name (mangle (str alias))))
                    (setv item-index (+ item-index 2)))
                  (when (not (= bound-name "*"))
                    (append-import-binding-name names forced bound-name)))
                (setv item-index (+ item-index 1))))
            (setv index (+ index 1)))
        True
          (when (and (= import-level 0) module-name)
            (append-import-binding-name names forced
                                        (get (.split module-name "." 1) 0)))))))


(defn append-defn-bound-name [args names forced]
  (do
    (setv offset 0)
    (when (and (> (len args) 0) (keyword-named? (get args 0) "async"))
      (setv offset 1))
    (when (and (> (len args) offset) (isinstance (get args offset) List))
      (setv offset (+ offset 1)))
    (when (> (len args) offset)
      (do
        (setv name (get args offset))
        (when (annotation-form? name)
          (setv name (annotation-target name)))
        (when (isinstance name Symbol)
          (append-import-binding-name names forced (mangle (str name))))))))


(defn append-defclass-bound-name [args names forced]
  (do
    (setv offset 0)
    (when (and (> (len args) 0) (isinstance (get args 0) List))
      (setv offset 1))
    (when (> (len args) offset)
      (do
        (setv name (get args offset))
        (when (isinstance name Symbol)
          (append-import-binding-name names forced (mangle (str name))))))))


(defn append-match-pattern-bound-names [pattern names]
  (cond
    (and (isinstance pattern Symbol)
         (not (symbol-named? pattern "_"))
         (not (in (str pattern) ["None" "True" "False"])))
      (append-unique-name names (mangle (str pattern)))
    (unpack-iterable-form? pattern)
      (append-match-pattern-bound-names (get pattern 1) names)
    (or (isinstance pattern List) (isinstance pattern Tuple))
      (for [item pattern]
        (append-match-pattern-bound-names item names))
    (isinstance pattern Dict)
      (do
        (setv items (model-list pattern))
        (when (% (len items) 2)
          (setv last-item (get items -1))
          (when (and (unpack-mapping-form? last-item)
                     (isinstance (get last-item 1) Symbol)
                     (not (symbol-named? (get last-item 1) "_")))
            (append-unique-name names (mangle (str (get last-item 1)))))
          (setv items (cut items 0 -1)))
        (for [index (range 1 (len items) 2)]
          (append-match-pattern-bound-names (get items index) names)))
    (isinstance pattern Expression)
      (do
        (when (> (len pattern) 0)
          (setv head (get pattern 0))
          (cond
            (symbol-named? head "|")
              (for [item (cut pattern 1 None)]
                (append-match-pattern-bound-names item names))
            (symbol-named? head "as")
              (do
                (when (> (len pattern) 1)
                  (append-match-pattern-bound-names (get pattern 1) names))
                (when (and (> (len pattern) 2)
                           (isinstance (get pattern 2) Symbol)
                           (not (symbol-named? (get pattern 2) "_")))
                  (append-unique-name names (mangle (str (get pattern 2))))))
            (symbol-named? head ".")
              None
            True
              (do
                (setv index 1)
                (while (< index (len pattern))
                  (do
                    (when (isinstance (get pattern index) Keyword)
                      (setv index (+ index 1)))
                    (when (< index (len pattern))
                      (append-match-pattern-bound-names (get pattern index) names))
                    (setv index (+ index 1))))))))
    True None))


(defn collect-let-body-binding-names [forms names forced]
  (for [form forms]
    (when (and (isinstance form Expression) (> (len form) 0))
      (setv head (get form 0))
      (setv args (cut form 1 None))
      (cond
        (symbol-named? head "setv")
          (do
            (setv index 0)
            (while (< index (len args))
              (if (keyword-named? (get args index) "chain")
                  (do
                    (when (< (+ index 1) (len args))
                      (for [target (get args (+ index 1))]
                        (append-target-binding-names target names)))
                    (setv index (+ index 3)))
                  (do
                    (append-target-binding-names (get args index) names)
                    (setv index (+ index 2))))))
        (symbol-named? head "annotate")
          (when (> (len args) 0)
            (append-target-binding-names (get args 0) names))
        (wrapped-annotation-form? form)
          (append-target-binding-names (annotation-target (get form 0)) names)
        (symbol-named? head "defn")
          (append-defn-bound-name args names forced)
        (symbol-named? head "defclass")
          (append-defclass-bound-name args names forced)
        (symbol-named? head "import")
          (append-import-bound-names args names forced)
        (symbol-named? head "for")
          (do
            (when (> (len args) 0)
              (append-for-binding-names (get args 0) names))
            (collect-let-body-binding-names (cut args 1 None) names forced))
        (symbol-named? head "with")
          (do
            (when (> (len args) 0)
              (append-with-binding-names (get args 0) names))
            (collect-let-body-binding-names (cut args 1 None) names forced))
        (symbol-named? head "match")
          (do
            (for [clause (parse-match-clauses (cut args 1 None))]
              (append-match-pattern-bound-names (get clause 0) names)
              (when (and (not (is (get clause 1) None))
                         (isinstance (get clause 1) Symbol))
                (append-unique-name names (mangle (str (get clause 1)))))
              (collect-let-body-binding-names (get clause 3) names forced)))
        (symbol-named? head "if")
          (collect-let-body-binding-names (cut args 1 None) names forced)
        (or (symbol-named? head "when")
            (symbol-named? head "cond")
            (symbol-named? head "do")
            (symbol-named? head "while"))
          (collect-let-body-binding-names args names forced)
        (symbol-named? head "try")
          (collect-try-let-body-binding-names args names forced)
        True None))))


(defn collect-parameter-binding-names [params names]
  (when (isinstance params List)
    (for [param params]
      (when (annotation-form? param)
        (setv param (annotation-target param)))
      (cond
        (default-parameter-form? param)
          (append-target-binding-names (get param 0) names)
        (and (unpack-iterable-form? param)
             (isinstance (get param 1) Symbol))
          (append-target-binding-names (get param 1) names)
        (and (unpack-mapping-form? param)
             (isinstance (get param 1) Symbol))
          (append-target-binding-names (get param 1) names)
        (or (parameter-name-symbol? param)
            (parameter-destructure-target? param))
          (append-target-binding-names param names)
        True None))))


(defn collect-function-binding-names [params body-forms]
  (setv names [])
  (setv forced [])
  (collect-parameter-binding-names params names)
  (collect-let-body-binding-names body-forms names forced)
  names)


(defn collect-try-let-body-binding-names [forms names forced]
  (for [form forms]
    (if (try-clause? form)
        (do
          (setv head (expression-head form))
          (cond
            (or (symbol-named? head "except")
                (symbol-named? head "except*"))
              (when (> (len form) 2)
                (collect-let-body-binding-names (cut form 2 None)
                                                names
                                                forced))
            (or (symbol-named? head "else")
                (symbol-named? head "finally"))
              (collect-let-body-binding-names (cut form 1 None)
                                              names
                                              forced)
            True None))
        (collect-let-body-binding-names [form] names forced))))


(defn enclosing-function-binding? [name]
  (for [scope (cut FUNCTION-BINDING-STACK 0 -1)]
    (when (in name scope)
      (return True)))
  False)


(defn declaration-names [args form-name]
  (when (= (len args) 0)
    (raise (SyntaxError (+ "kernel " form-name " needs at least one name"))))
  (setv names [])
  (for [arg args]
    (when (not (isinstance arg Symbol))
      (raise (SyntaxError (+ "kernel " form-name " names must be symbols"))))
    (.append names (mangle (str arg))))
  names)


(defn compile-nonlocal-declaration [args]
  (setv nonlocal-names [])
  (setv global-names [])
  (for [name (declaration-names args "nonlocal")]
    (cond
      (enclosing-function-binding? name)
        (.append nonlocal-names name)
      (in name MODULE-BINDING-NAMES)
        (.append global-names name)
      True
        (.append nonlocal-names name)))
  (setv statements [])
  (when global-names
    (.append statements (ast.Global :names global-names)))
  (when nonlocal-names
    (.append statements (ast.Nonlocal :names nonlocal-names)))
  statements)


(defn constant-string-list [items]
  (ast.List :elts (list (map (fn [item] (ast.Constant :value item)) items))
            :ctx (ast.Load)))


(defn let-local-bindings-dict [possible-names active-keys forced-names originals-name]
  (setv key-name "__hy_meta_local_key")
  (setv value-name "__hy_meta_local_value")
  (ast.DictComp
    :key (ast.Name :id key-name :ctx (ast.Load))
    :value (ast.Name :id value-name :ctx (ast.Load))
    :generators
      [(ast.comprehension
         :target (ast.Tuple :elts [(ast.Name :id key-name :ctx (ast.Store))
                                   (ast.Name :id value-name :ctx (ast.Store))]
                            :ctx (ast.Store))
         :iter (ast.Call
                 :func (ast.Attribute
                         :value (ast.Call :func (ast.Name :id "locals" :ctx (ast.Load))
                                          :args []
                                          :keywords [])
                         :attr "items"
                         :ctx (ast.Load))
                 :args []
                 :keywords [])
         :ifs [(ast.BoolOp
                 :op (ast.And)
                 :values
                   [(ast.Compare :left (ast.Name :id key-name :ctx (ast.Load))
                                 :ops [(ast.In)]
                                 :comparators [(constant-string-list possible-names)])
                    (ast.UnaryOp
                      :op (ast.Not)
                      :operand
                        (ast.Call
                          :func (ast.Attribute
                                  :value (ast.Name :id key-name :ctx (ast.Load))
                                  :attr "startswith"
                                  :ctx (ast.Load))
                          :args [(ast.Constant :value "__hy_meta_")]
                          :keywords []))
                    (ast.BoolOp
                      :op (ast.Or)
                      :values
                        [(ast.Compare :left (ast.Name :id key-name :ctx (ast.Load))
                                      :ops [(ast.NotIn)]
                                      :comparators [(constant-string-list active-keys)])
                         (ast.BoolOp
                           :op (ast.And)
                           :values
                             [(ast.Compare :left (ast.Name :id key-name :ctx (ast.Load))
                                           :ops [(ast.In)]
                                           :comparators [(constant-string-list forced-names)])
                              (ast.Compare
                                :left (ast.Name :id value-name :ctx (ast.Load))
                                :ops [(ast.IsNot)]
                                :comparators
                                  [(ast.Subscript
                                     :value (ast.Name :id originals-name :ctx (ast.Load))
                                     :slice (ast.Name :id key-name :ctx (ast.Load))
                                     :ctx (ast.Load))])])])])]
         :is_async 0)]))


(defn let-leak-assignment [bindings-name name]
  (ast.If
    :test (ast.Compare :left (ast.Constant :value name)
                       :ops [(ast.In)]
                       :comparators [(ast.Name :id bindings-name :ctx (ast.Load))])
    :body [(ast.Assign
             :targets [(ast.Name :id name :ctx (ast.Store))]
             :value (ast.Subscript
                      :value (ast.Name :id bindings-name :ctx (ast.Load))
                      :slice (ast.Constant :value name)
                      :ctx (ast.Load)))]
    :orelse []))


(defn wrap-let-binding [target value inner index available-names]
  (setv compiled-value (compile-expr-isolated-pending value))
  (setv value-pending (get compiled-value 0))
  (setv value-node (get compiled-value 1))
  (when value-pending
    (setv helper-name (fresh-generated-name "__hy_meta_let_value_"))
    (append-pending-statement
      (ast.FunctionDef
        :name helper-name
        :args (compile-arguments (List available-names))
        :body (+ value-pending
                 [(ast.Return :value value-node)])
        :decorator_list []
        :returns None
        :type_comment None))
    (setv value-node
          (ast.Call :func (ast.Name :id helper-name :ctx (ast.Load))
                    :args (list (map let-load-name available-names))
                    :keywords [])))
  (cond
    (or (isinstance target Symbol)
        (annotation-form? target))
      (ast.Call :func (ast.Lambda :args (compile-arguments (List [target]))
                                  :body inner)
                :args [value-node]
                :keywords [])
    (or (isinstance target List) (isinstance target Tuple))
      (do
        (setv temp-name (Symbol (+ "__hy_meta_let_unpack_" (str index))))
        (setv destructured-names [])
        (setv destructured-values [])
        (collect-destructure-bindings
          target
          (load-symbol temp-name)
          destructured-names
          destructured-values)
        (setv destructured-inner
              (ast.Call :func (ast.Lambda :args (compile-arguments (List destructured-names))
                                          :body inner)
                        :args destructured-values
                        :keywords []))
        (ast.Call :func (ast.Lambda :args (compile-arguments (List [temp-name]))
                                    :body destructured-inner)
                  :args [value-node]
                  :keywords []))
    True
      (raise (SyntaxError "kernel let binding names must be symbols or sequences"))))


(defn locals-get-original [name sentinel-name]
  (ast.Call
    :func (ast.Attribute
            :value (ast.Call :func (ast.Name :id "locals" :ctx (ast.Load))
                             :args []
                             :keywords [])
            :attr "get"
            :ctx (ast.Load))
    :args [(ast.Constant :value name)
           (ast.Name :id sentinel-name :ctx (ast.Load))]
    :keywords []))


(defn restore-inline-let-name [name original-name sentinel-name]
  (ast.If
    :test (ast.Compare
            :left (ast.Name :id original-name :ctx (ast.Load))
            :ops [(ast.Is)]
            :comparators [(ast.Name :id sentinel-name :ctx (ast.Load))])
    :body [(ast.Try
             :body [(ast.Delete :targets [(ast.Name :id name :ctx (ast.Del))])]
             :handlers [(ast.ExceptHandler
                           :type (ast.Name :id "NameError" :ctx (ast.Load))
                           :name None
                           :body [(ast.Pass)])]
             :orelse []
             :finalbody [])]
    :orelse [(ast.Assign
               :targets [(ast.Name :id name :ctx (ast.Store))]
               :value (ast.Name :id original-name :ctx (ast.Load)))]))


(defn compile-inline-let-expr [binding-targets binding-values body-forms]
  (global LET-PROTECTED-BINDING-STACK)
  (setv result-name (fresh-generated-name "__hy_meta_let_inline_result_"))
  (setv sentinel-name (fresh-generated-name "__hy_meta_let_inline_sentinel_"))
  (setv target-names [])
  (for [target binding-targets]
    (setv visible-names [])
    (collect-let-target-names target visible-names)
    (for [visible-name visible-names]
      (append-unique-name target-names (let-name-key visible-name))))
  (setv body [])
  (.append body
           (ast.Assign
             :targets [(ast.Name :id sentinel-name :ctx (ast.Store))]
             :value (ast.Call :func (ast.Name :id "object" :ctx (ast.Load))
                              :args []
                              :keywords [])))
  (setv original-names [])
  (for [name target-names]
    (setv original-name (fresh-generated-name "__hy_meta_let_inline_original_"))
    (.append original-names [name original-name])
    (.append body
             (ast.Assign
               :targets [(ast.Name :id original-name :ctx (ast.Store))]
               :value (locals-get-original name sentinel-name))))
  (setv try-body [])
  (for [index (range (len binding-targets))]
    (.extend try-body
             (compile-setv [(get binding-targets index)
                            (get binding-values index)])))
  (.append LET-PROTECTED-BINDING-STACK target-names)
  (try
    (.extend try-body (compile-value-assignment-body body-forms result-name))
    (finally
      (.pop LET-PROTECTED-BINDING-STACK)))
  (setv restore-body [])
  (for [entry (reversed original-names)]
    (.append restore-body
             (restore-inline-let-name (get entry 0) (get entry 1) sentinel-name)))
  (.append body
           (ast.Try :body try-body
                    :handlers []
                    :orelse []
                    :finalbody restore-body))
  (for [statement body]
    (append-pending-statement statement))
  (ast.Name :id result-name :ctx (ast.Load)))


(defn compile-let-expr [args]
  (global PENDING-STATEMENTS LET-PROTECTED-BINDING-STACK
          FUNCTION-SCOPE-DEPTH FUNCTION-BINDING-STACK)
  (when (< (len args) 2)
    (raise (SyntaxError "kernel let needs a binding vector and a body")))
  (setv bindings (get args 0))
  (when (not (isinstance bindings List))
    (raise (SyntaxError "kernel let bindings must be a vector")))
  (when (% (len bindings) 2)
    (raise (SyntaxError "kernel let needs name/value binding pairs")))
  (setv binding-targets [])
  (setv binding-values [])
  (setv binding-visible-before [])
  (setv active-names [])
  (setv active-keys [])
  (for [index (range 0 (len bindings) 2)]
    (setv name (get bindings index))
    (.append binding-visible-before (list active-names))
    (setv visible-names [])
    (collect-let-target-names name visible-names)
    (for [visible-name visible-names]
      (upsert-active-name active-names active-keys visible-name))
    (.append binding-targets name)
    (.append binding-values (get bindings (+ index 1))))
  (setv body-forms (cut args 1 None))
  (when (or (any (map control-flow-form? body-forms))
            (any (map scope-declaration-form? body-forms)))
    (return (compile-inline-let-expr binding-targets binding-values body-forms)))
  (setv body-binding-names [])
  (setv forced-body-binding-names [])
  (collect-let-body-binding-names body-forms
                                  body-binding-names
                                  forced-body-binding-names)
  (setv has-async (any (map async-helper-form? body-forms)))
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv result-name (fresh-generated-name "__hy_meta_let_result_"))
  (setv originals-name (fresh-generated-name "__hy_meta_let_originals_"))
  (.append LET-PROTECTED-BINDING-STACK active-keys)
  (.append FUNCTION-BINDING-STACK (+ active-keys body-binding-names))
  (setv FUNCTION-SCOPE-DEPTH (+ FUNCTION-SCOPE-DEPTH 1))
  (try
    (setv body (compile-value-assignment-body body-forms result-name))
    (finally
      (setv FUNCTION-SCOPE-DEPTH (- FUNCTION-SCOPE-DEPTH 1))
      (.pop FUNCTION-BINDING-STACK)
      (.pop LET-PROTECTED-BINDING-STACK)))
  (setv nested-pending (drain-pending-statements))
  (setv PENDING-STATEMENTS outer-pending)
  (setv helper-name (fresh-generated-name "__hy_meta_let_"))
  (append-pending-statement
    ((if has-async ast.AsyncFunctionDef ast.FunctionDef)
      :name helper-name
      :args (compile-arguments (List active-names))
      :body (+ [(ast.Assign
                  :targets [(ast.Name :id originals-name :ctx (ast.Store))]
                  :value (ast.Dict
                           :keys (list (map (fn [key] (ast.Constant :value key))
                                            active-keys))
                           :values (list (map let-load-name active-names))))]
               nested-pending
               body
               [(ast.Return
                  :value
                    (ast.Tuple
                      :elts [(ast.Name :id result-name :ctx (ast.Load))
                             (let-local-bindings-dict body-binding-names
                                                      active-keys
                                                      forced-body-binding-names
                                                      originals-name)]
                      :ctx (ast.Load)))])
      :decorator_list []
      :returns None
      :type_comment None))
  (setv call (ast.Call :func (ast.Name :id helper-name :ctx (ast.Load))
                       :args (list (map let-load-name active-names))
                       :keywords []))
  (setv inner (if has-async
                  (ast.Await :value call)
                  call))
  (for [index (range (- (len binding-targets) 1) -1 -1)]
    (setv inner
          (wrap-let-binding
            (get binding-targets index)
            (get binding-values index)
            inner
            index
            (get binding-visible-before index))))
  (setv call-result-name (fresh-generated-name "__hy_meta_let_call_"))
  (setv bindings-name (fresh-generated-name "__hy_meta_let_bindings_"))
  (append-pending-statement
    (ast.Assign :targets [(ast.Name :id call-result-name :ctx (ast.Store))]
                :value inner))
  (append-pending-statement
    (ast.Assign
      :targets [(ast.Name :id bindings-name :ctx (ast.Store))]
      :value (ast.Subscript :value (ast.Name :id call-result-name :ctx (ast.Load))
                            :slice (ast.Constant :value 1)
                            :ctx (ast.Load))))
  (for [name body-binding-names]
    (append-pending-statement (let-leak-assignment bindings-name name)))
  (ast.Subscript :value (ast.Name :id call-result-name :ctx (ast.Load))
                 :slice (ast.Constant :value 0)
                 :ctx (ast.Load)))


(defn compile-get-expr [args]
  (when (< (len args) 2)
    (raise (SyntaxError "kernel get needs a collection and at least one index")))
  (setv node (compile-expr (get args 0)))
  (for [index (cut args 1 None)]
    (setv node (ast.Subscript :value node
                              :slice (compile-expr index)
                              :ctx (ast.Load))))
  node)


(defn slice-bound [args index]
  (if (< index (len args))
      (compile-expr (get args index))
      None))


(defn cut-slice [args]
  (cond
    (= (len args) 1)
      (ast.Slice :lower None :upper None :step None)
    (= (len args) 2)
      (ast.Slice :lower None :upper (compile-expr (get args 1)) :step None)
    (= (len args) 3)
      (ast.Slice :lower (compile-expr (get args 1))
                 :upper (compile-expr (get args 2))
                 :step None)
    (= (len args) 4)
      (ast.Slice :lower (compile-expr (get args 1))
                 :upper (compile-expr (get args 2))
                 :step (compile-expr (get args 3)))
    True
      (raise (SyntaxError "kernel cut needs a collection, optional start, optional stop, and optional step"))))


(defn compile-cut-expr [args]
  (when (not (in (len args) [1 2 3 4]))
    (raise (SyntaxError "kernel cut needs a collection, optional start, optional stop, and optional step")))
  (ast.Subscript :value (compile-expr (get args 0))
                 :slice (cut-slice args)
                 :ctx (ast.Load)))


(defn compile-comprehension-generators [args value-index form-name]
  (setv generators [])
  (setv current-ifs None)
  (setv index 0)
  (while (< index value-index)
    (cond
      (keyword-named? (get args index) "if")
        (do
          (when (= (len generators) 0)
            (raise (SyntaxError (+ "kernel " form-name " :if needs a preceding generator"))))
          (when (>= (+ index 1) value-index)
            (raise (SyntaxError (+ "kernel " form-name " :if needs a test expression"))))
          (.append current-ifs (compile-expr (get args (+ index 1))))
          (setv index (+ index 2)))
      True
        (do
          (setv is-async 0)
          (when (keyword-named? (get args index) "async")
            (setv is-async 1)
            (setv index (+ index 1)))
          (when (>= (+ index 1) value-index)
            (raise (SyntaxError (+ "kernel " form-name " generator needs a target and iterable"))))
          (setv current-ifs [])
          (.append generators
                   (ast.comprehension :target (store-target (get args index))
                                      :iter (compile-expr (get args (+ index 1)))
                                      :ifs current-ifs
                                      :is_async is-async))
          (setv index (+ index 2)))))
  generators)


(defn side-effect-comprehension-clause? [args value-index]
  (setv found False)
  (for [index (range value-index)]
    (when (or (keyword-named? (get args index) "do")
              (keyword-named? (get args index) "setv"))
      (setv found True)))
  found)


(defn async-comprehension-clause? [args value-index]
  (setv found False)
  (for [index (range value-index)]
    (when (keyword-named? (get args index) "async")
      (setv found True)))
  found)


(defn comprehension-expression-needs-helper? [model]
  (and (isinstance model Expression)
       (or (statement-form? model)
           (and (symbol-named? (expression-head model) "do")
                (> (len model) 2)))))


(defn comprehension-clauses-need-helper? [args value-index]
  (setv found False)
  (setv index 0)
  (while (< index value-index)
    (cond
      (keyword-named? (get args index) "if")
        (do
          (when (and (< (+ index 1) value-index)
                     (comprehension-expression-needs-helper? (get args (+ index 1))))
            (setv found True))
          (setv index (+ index 2)))
      (keyword-named? (get args index) "do")
        (setv index (+ index 2))
      (keyword-named? (get args index) "setv")
        (setv index (+ index 3))
      True
        (do
          (when (keyword-named? (get args index) "async")
            (setv index (+ index 1)))
          (when (and (< (+ index 1) value-index)
                     (comprehension-expression-needs-helper? (get args (+ index 1))))
            (setv found True))
          (setv index (+ index 2)))))
  found)


(defn compile-comprehension-yield [value]
  (setv value-node (compile-expr value))
  (+ (drain-pending-statements)
     [(ast.Expr :value (ast.Yield :value value-node))]))


(defn compile-comprehension-yield-each [iterable]
  (setv iterable-node (compile-expr iterable))
  (setv item-name (fresh-generated-name "__hy_meta_unpack_item_"))
  (+ (drain-pending-statements)
     [(ast.For
        :target (ast.Name :id item-name :ctx (ast.Store))
        :iter iterable-node
        :body [(ast.Expr
                 :value (ast.Yield
                          :value (ast.Name :id item-name :ctx (ast.Load))))]
        :orelse [])]))


(defn compile-dict-comprehension-yield [key value]
  (setv key-node (compile-expr key))
  (setv value-node (compile-expr value))
  (+ (drain-pending-statements)
     [(ast.Expr :value (ast.Yield
                         :value (ast.Tuple :elts [key-node value-node]
                                           :ctx (ast.Load))))]))


(defn compile-dict-update-statement [result-name mapping]
  (setv mapping-node (compile-expr mapping))
  (+ (drain-pending-statements)
     [(ast.Expr
        :value (ast.Call
                 :func (ast.Attribute
                         :value (ast.Name :id result-name :ctx (ast.Load))
                         :attr "update"
                         :ctx (ast.Load))
                 :args [mapping-node]
                 :keywords []))]))


(defn compile-comprehension-update-statement [result-name iterable method-name]
  (setv iterable-node (compile-expr iterable))
  (+ (drain-pending-statements)
     [(ast.Expr
        :value (ast.Call
                 :func (ast.Attribute
                         :value (ast.Name :id result-name :ctx (ast.Load))
                         :attr method-name
                         :ctx (ast.Load))
                 :args [iterable-node]
                 :keywords []))]))


(defn compile-comprehension-result-statement [result-name final-kind final-key final-value]
  (cond
    (= final-kind "list")
      (if (unpack-iterable-form? final-value)
          (compile-comprehension-update-statement result-name
                                                  (get final-value 1)
                                                  "extend")
          (do
            (setv value-node (compile-expr final-value))
            (+ (drain-pending-statements)
               [(ast.Expr
                  :value (ast.Call
                           :func (ast.Attribute
                                   :value (ast.Name :id result-name :ctx (ast.Load))
                                   :attr "append"
                                   :ctx (ast.Load))
                           :args [value-node]
                           :keywords []))])))
    (= final-kind "set")
      (if (unpack-iterable-form? final-value)
          (compile-comprehension-update-statement result-name
                                                  (get final-value 1)
                                                  "update")
          (do
            (setv value-node (compile-expr final-value))
            (+ (drain-pending-statements)
               [(ast.Expr
                  :value (ast.Call
                           :func (ast.Attribute
                                   :value (ast.Name :id result-name :ctx (ast.Load))
                                   :attr "add"
                                   :ctx (ast.Load))
                           :args [value-node]
                           :keywords []))])))
    (= final-kind "dict")
      (do
        (setv key-node (compile-expr final-key))
        (setv value-node (compile-expr final-value))
        (+ (drain-pending-statements)
           [(ast.Assign
              :targets [(ast.Subscript
                          :value (ast.Name :id result-name :ctx (ast.Load))
                          :slice key-node
                          :ctx (ast.Store))]
              :value value-node)]))
    (= final-kind "gfor")
      (if (unpack-iterable-form? final-value)
          (compile-comprehension-yield-each (get final-value 1))
          (compile-comprehension-yield final-value))
    True
      (raise (SyntaxError "kernel unknown comprehension result kind"))))


(defn collect-scope-declaration-names [model names]
  (cond
    (isinstance model Expression)
      (do
        (when (> (len model) 0)
          (setv head (get model 0))
          (cond
            (or (symbol-named? head "global")
                (symbol-named? head "nonlocal"))
              (for [name (declaration-names (cut model 1 None) (str head))]
                (append-unique-name names name))
            (or (symbol-named? head "quote")
                (symbol-named? head "quasiquote")
                (symbol-named? head "fn")
                (symbol-named? head "defn")
                (symbol-named? head "defclass"))
              None
            True
              (for [item model]
                (collect-scope-declaration-names item names)))))
    (or (isinstance model List)
        (isinstance model Tuple)
        (isinstance model Set)
        (isinstance model FString)
        (isinstance model FComponent))
      (for [item model]
        (collect-scope-declaration-names item names))
    (isinstance model Dict)
      (for [item (model-list model)]
        (collect-scope-declaration-names item names))
    True None))


(defn collect-comprehension-setx-names [args value-index final-kind final-key final-value]
  (setv names [])
  (setv declared-names [])
  (setv index 0)
  (while (< index value-index)
    (cond
      (keyword-named? (get args index) "if")
        (do
          (when (< (+ index 1) value-index)
            (collect-setx-binding-names (get args (+ index 1)) names)
            (collect-scope-declaration-names (get args (+ index 1)) declared-names))
          (setv index (+ index 2)))
      (keyword-named? (get args index) "do")
        (do
          (when (< (+ index 1) value-index)
            (collect-setx-binding-names (get args (+ index 1)) names)
            (collect-scope-declaration-names (get args (+ index 1)) declared-names))
          (setv index (+ index 2)))
      (keyword-named? (get args index) "setv")
        (do
          (when (< (+ index 2) value-index)
            (collect-setx-binding-names (get args (+ index 2)) names)
            (collect-scope-declaration-names (get args (+ index 2)) declared-names))
          (setv index (+ index 3)))
      True
        (do
          (when (keyword-named? (get args index) "async")
            (setv index (+ index 1)))
          (when (< (+ index 1) value-index)
            (collect-setx-binding-names (get args (+ index 1)) names)
            (collect-scope-declaration-names (get args (+ index 1)) declared-names))
          (setv index (+ index 2)))))
  (when (= final-kind "dict")
    (collect-setx-binding-names final-key names)
    (collect-scope-declaration-names final-key declared-names))
  (collect-setx-binding-names final-value names)
  (collect-scope-declaration-names final-value declared-names)
  (setv filtered [])
  (for [name names]
    (when (not (in name declared-names))
      (.append filtered name)))
  filtered)


(defn setx-scope-declarations [names]
  (if names
      (if (> FUNCTION-SCOPE-DEPTH 0)
          [(ast.Nonlocal :names names)]
          [(ast.Global :names names)])
      []))


(defn setx-enclosing-scope-bindings [names]
  (if (and names (> FUNCTION-SCOPE-DEPTH 0))
      [(ast.If :test (ast.Constant :value False)
               :body (list (map
                              (fn [name]
                                (ast.Assign
                                  :targets [(ast.Name :id name :ctx (ast.Store))]
                                  :value (ast.Constant :value None)))
                              names))
               :orelse [])]
      []))


(defn empty-comprehension-result [final-kind]
  (cond
    (= final-kind "list") (ast.List :elts [] :ctx (ast.Load))
    (= final-kind "set") (ast.Call :func (ast.Name :id "set" :ctx (ast.Load))
                                   :args []
                                   :keywords [])
    (= final-kind "dict") (ast.Dict :keys [] :values [])
    True (raise (SyntaxError "kernel unknown comprehension result kind"))))


(defn empty-gfor-result []
  (setv item-name (fresh-generated-name "__hy_meta_empty_gfor_item_"))
  (ast.GeneratorExp
    :elt (ast.Constant :value None)
    :generators [(ast.comprehension
                   :target (ast.Name :id item-name :ctx (ast.Store))
                   :iter (ast.Tuple :elts [] :ctx (ast.Load))
                   :ifs []
                   :is_async 0)]))


(defn compile-comprehension-loop-statements [args index value-index final-statements
                                             form-name [allow-async False]]
  (if (>= index value-index)
      final-statements
      (cond
        (keyword-named? (get args index) "if")
          (do
            (when (>= (+ index 1) value-index)
              (raise (SyntaxError (+ "kernel " form-name " :if needs a test expression"))))
            (setv test (compile-expr (get args (+ index 1))))
            (+ (drain-pending-statements)
               [(ast.If :test test
                        :body (compile-comprehension-loop-statements
                                 args (+ index 2) value-index final-statements
                                 form-name allow-async)
                        :orelse [])]))
        (keyword-named? (get args index) "do")
          (do
            (when (>= (+ index 1) value-index)
              (raise (SyntaxError (+ "kernel " form-name " :do needs an expression"))))
            (+ (compile-form-with-pending-as-statements (get args (+ index 1)))
               (compile-comprehension-loop-statements
                 args (+ index 2) value-index final-statements form-name
                 allow-async)))
        (keyword-named? (get args index) "setv")
          (do
            (when (>= (+ index 2) value-index)
              (raise (SyntaxError (+ "kernel " form-name " :setv needs a target and value"))))
            (+ (compile-form-with-pending-as-statements
                 (Expression [(Symbol "setv")
                              (get args (+ index 1))
                              (get args (+ index 2))]))
               (compile-comprehension-loop-statements
                 args (+ index 3) value-index final-statements form-name
                 allow-async)))
        True
          (do
            (setv is-async False)
            (when (keyword-named? (get args index) "async")
              (setv is-async True)
              (setv index (+ index 1)))
            (when (and is-async (not allow-async))
              (raise (SyntaxError (+ "kernel " form-name " :do/:setv lowering does not support async generators in sync helpers"))))
            (when (>= (+ index 1) value-index)
              (raise (SyntaxError (+ "kernel " form-name " generator needs a target and iterable"))))
            (setv iterable (compile-expr (get args (+ index 1))))
            (+ (drain-pending-statements)
               [((if is-async ast.AsyncFor ast.For)
                  :target (store-target (get args index))
                  :iter iterable
                  :body (compile-comprehension-loop-statements
                           args (+ index 2) value-index final-statements
                           form-name allow-async)
                  :orelse [])])))))


(defn compile-side-effect-comprehension [args value-index final-kind
                                        final-key final-value wrapper-name form-name]
  (global PENDING-STATEMENTS)
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv final-statements
        (cond
          (= final-kind "dict")
            (compile-dict-comprehension-yield final-key final-value)
          (unpack-iterable-form? final-value)
            (compile-comprehension-yield-each (get final-value 1))
          True
            (compile-comprehension-yield final-value)))
  (setv loop-body (compile-comprehension-loop-statements
                    args 0 value-index final-statements form-name))
  (setv nested-pending (drain-pending-statements))
  (setv PENDING-STATEMENTS outer-pending)
  (setv helper-name (fresh-generated-name (+ "__hy_meta_" form-name "_")))
  (setv setx-names
        (collect-comprehension-setx-names args value-index final-kind
                                          final-key final-value))
  (for [statement (setx-enclosing-scope-bindings setx-names)]
    (append-pending-statement statement))
  (append-pending-statement
    (ast.FunctionDef :name helper-name
                     :args (ast.arguments :posonlyargs []
                                          :args []
                                          :vararg None
                                          :kwonlyargs []
                                          :kw_defaults []
                                          :kwarg None
                                          :defaults [])
                     :body (+ (setx-scope-declarations setx-names)
                              nested-pending
                              loop-body)
                     :decorator_list []
                     :returns None
                     :type_comment None))
  (setv call (ast.Call :func (ast.Name :id helper-name :ctx (ast.Load))
                       :args []
                       :keywords []))
  (if wrapper-name
      (ast.Call :func (ast.Name :id wrapper-name :ctx (ast.Load))
                :args [call]
                :keywords [])
      call))


(defn compile-async-side-effect-comprehension [args value-index final-kind
                                              final-key final-value form-name]
  (global PENDING-STATEMENTS)
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv result-name (fresh-generated-name (+ "__hy_meta_" form-name "_result_")))
  (setv final-statements
        (compile-comprehension-result-statement
          result-name final-kind final-key final-value))
  (setv loop-body (compile-comprehension-loop-statements
                    args 0 value-index final-statements form-name True))
  (setv nested-pending (drain-pending-statements))
  (setv PENDING-STATEMENTS outer-pending)
  (setv helper-name (fresh-generated-name (+ "__hy_meta_" form-name "_async_")))
  (setv setx-names
        (collect-comprehension-setx-names args value-index final-kind
                                          final-key final-value))
  (for [statement (setx-enclosing-scope-bindings setx-names)]
    (append-pending-statement statement))
  (setv body
        (if (= final-kind "gfor")
            (+ (setx-scope-declarations setx-names)
               nested-pending
               loop-body)
            (+ (setx-scope-declarations setx-names)
               nested-pending
               [(ast.Assign
                  :targets [(ast.Name :id result-name :ctx (ast.Store))]
                  :value (empty-comprehension-result final-kind))]
               loop-body
               [(ast.Return
                  :value (ast.Name :id result-name :ctx (ast.Load)))])))
  (append-pending-statement
    (ast.AsyncFunctionDef :name helper-name
                          :args (ast.arguments :posonlyargs []
                                               :args []
                                               :vararg None
                                               :kwonlyargs []
                                               :kw_defaults []
                                               :kwarg None
                                               :defaults [])
                          :body body
                          :decorator_list []
                          :returns None
                          :type_comment None))
  (setv call (ast.Call :func (ast.Name :id helper-name :ctx (ast.Load))
                       :args []
                       :keywords []))
  (if (= final-kind "gfor")
      call
      (ast.Await :value call)))


(defn compile-dfor-unpack-expr [args mapping-index mapping]
  (global PENDING-STATEMENTS)
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv result-name (fresh-generated-name "__hy_meta_dfor_result_"))
  (setv final-statements (compile-dict-update-statement result-name mapping))
  (setv loop-body (compile-comprehension-loop-statements
                    args 0 mapping-index final-statements "dfor"))
  (setv nested-pending (drain-pending-statements))
  (setv PENDING-STATEMENTS outer-pending)
  (setv helper-name (fresh-generated-name "__hy_meta_dfor_unpack_"))
  (setv setx-names
        (collect-comprehension-setx-names args mapping-index "dict"
                                          None mapping))
  (for [statement (setx-enclosing-scope-bindings setx-names)]
    (append-pending-statement statement))
  (append-pending-statement
    (ast.FunctionDef :name helper-name
                     :args (ast.arguments :posonlyargs []
                                          :args []
                                          :vararg None
                                          :kwonlyargs []
                                          :kw_defaults []
                                          :kwarg None
                                          :defaults [])
                     :body (+ (setx-scope-declarations setx-names)
                              nested-pending
                              [(ast.Assign
                                 :targets [(ast.Name :id result-name
                                                     :ctx (ast.Store))]
                                 :value (ast.Dict :keys [] :values []))]
                              loop-body
                              [(ast.Return
                                 :value (ast.Name :id result-name
                                                  :ctx (ast.Load)))])
                     :decorator_list []
                     :returns None
                     :type_comment None))
  (ast.Call :func (ast.Name :id helper-name :ctx (ast.Load))
            :args []
            :keywords []))


(defn compile-lfor-expr [args]
  (when (= (len args) 1)
    (return (empty-comprehension-result "list")))
  (when (< (len args) 3)
    (raise (SyntaxError "kernel lfor needs clauses and a value expression")))
  (setv value-index (- (len args) 1))
  (setv final-value (get args value-index))
  (if (or (side-effect-comprehension-clause? args value-index)
          (unpack-iterable-form? final-value)
          (comprehension-clauses-need-helper? args value-index)
          (comprehension-expression-needs-helper? final-value))
      (if (async-comprehension-clause? args value-index)
          (compile-async-side-effect-comprehension
            args value-index "list" None final-value "lfor")
          (compile-side-effect-comprehension
            args value-index "value" None final-value
            "list"
            "lfor"))
      (ast.ListComp :elt (compile-expr final-value)
                    :generators (compile-comprehension-generators args value-index "lfor"))))


(defn compile-sfor-expr [args]
  (when (= (len args) 1)
    (return (empty-comprehension-result "set")))
  (when (< (len args) 3)
    (raise (SyntaxError "kernel sfor needs clauses and a value expression")))
  (setv value-index (- (len args) 1))
  (setv final-value (get args value-index))
  (if (or (side-effect-comprehension-clause? args value-index)
          (unpack-iterable-form? final-value)
          (comprehension-clauses-need-helper? args value-index)
          (comprehension-expression-needs-helper? final-value))
      (if (async-comprehension-clause? args value-index)
          (compile-async-side-effect-comprehension
            args value-index "set" None final-value "sfor")
          (compile-side-effect-comprehension
            args value-index "value" None final-value
            "set"
            "sfor"))
      (ast.SetComp :elt (compile-expr final-value)
                   :generators (compile-comprehension-generators args value-index "sfor"))))


(defn compile-gfor-expr [args]
  (when (= (len args) 1)
    (return (empty-gfor-result)))
  (when (< (len args) 3)
    (raise (SyntaxError "kernel gfor needs clauses and a value expression")))
  (setv value-index (- (len args) 1))
  (setv final-value (get args value-index))
  (if (or (side-effect-comprehension-clause? args value-index)
          (unpack-iterable-form? final-value)
          (comprehension-clauses-need-helper? args value-index)
          (comprehension-expression-needs-helper? final-value))
      (if (async-comprehension-clause? args value-index)
          (compile-async-side-effect-comprehension
            args value-index "gfor" None final-value "gfor")
          (compile-side-effect-comprehension
            args value-index "value" None final-value
            None
            "gfor"))
      (ast.GeneratorExp :elt (compile-expr final-value)
                        :generators (compile-comprehension-generators args value-index "gfor"))))


(defn compile-dfor-expr [args]
  (when (= (len args) 2)
    (return (empty-comprehension-result "dict")))
  (when (< (len args) 3)
    (raise (SyntaxError "kernel dfor needs clauses and a key/value pair or #** mapping")))
  (setv last-arg (get args -1))
  (if (unpack-mapping-form? last-arg)
      (do
        (when (not (= (len last-arg) 2))
          (raise (SyntaxError "kernel dfor #** unpacking needs one expression")))
        (compile-dfor-unpack-expr args (- (len args) 1) (get last-arg 1)))
      (do
        (when (< (len args) 4)
          (raise (SyntaxError "kernel dfor needs clauses, a key expression, and a value expression")))
        (setv key-index (- (len args) 2))
        (if (or (side-effect-comprehension-clause? args key-index)
                (comprehension-clauses-need-helper? args key-index)
                (comprehension-expression-needs-helper? (get args key-index))
                (comprehension-expression-needs-helper? (get args (+ key-index 1))))
            (if (async-comprehension-clause? args key-index)
                (compile-async-side-effect-comprehension
                  args key-index "dict" (get args key-index) (get args (+ key-index 1))
                  "dfor")
                (compile-side-effect-comprehension
                  args key-index "dict" (get args key-index) (get args (+ key-index 1))
                  "dict"
                  "dfor"))
            (ast.DictComp :key (compile-expr (get args key-index))
                          :value (compile-expr (get args (+ key-index 1)))
                          :generators (compile-comprehension-generators args key-index "dfor"))))))


(defn compile-dot-chain [args ctx allow-calls]
  (when (= (len args) 0)
    (raise (SyntaxError "kernel dot chain needs a receiver")))
  (setv node (compile-expr (get args 0)))
  (for [index (range 1 (len args))]
    (setv part (get args index))
    (setv part-ctx (if (= index (- (len args) 1)) ctx (ast.Load)))
    (cond
      (isinstance part Symbol)
        (setv node
              (ast.Attribute :value node
                             :attr (mangle (str part))
                             :ctx part-ctx))
      (isinstance part List)
        (do
          (when (not (= (len part) 1))
            (raise (SyntaxError "kernel dot subscript parts need exactly one index")))
          (setv node
                (ast.Subscript :value node
                               :slice (compile-expr (get part 0))
                               :ctx part-ctx)))
      (or (unpack-iterable-form? part)
          (unpack-mapping-form? part))
        (raise (SyntaxError "kernel dot chain parts cannot be unpacking forms"))
      (isinstance part Expression)
        (do
          (when (not allow-calls)
            (raise (SyntaxError "kernel dot targets cannot contain method calls")))
          (when (not (isinstance (get part 0) Symbol))
            (raise (SyntaxError "kernel dot method names must be symbols")))
          (setv node
                (compile-call-node
                  (ast.Attribute :value node
                                 :attr (mangle (str (get part 0)))
                                 :ctx (ast.Load))
                  (cut part 1 None))))
      True
        (raise (SyntaxError "kernel dot chain parts must be symbols, index vectors, or method calls"))))
  node)


(defn compile-attribute [args]
  (compile-dot-chain args (ast.Load) True))


(defn method-shortcut-receiver-index [args]
  (setv index 0)
  (while (< index (len args))
    (setv arg (get args index))
    (cond
      (isinstance arg Keyword)
        (do
          (if (< (+ index 2) (len args))
              (setv index (+ index 2))
              (return index)))
      (unpack-mapping-form? arg)
        (setv index (+ index 1))
      True
        (return index)))
  (raise (SyntaxError "kernel method call shortcut needs a receiver")))


(defn compile-method-shortcut-call [root args]
  (when (= (len args) 0)
    (raise (SyntaxError "kernel method call shortcut needs a receiver")))
  (setv receiver-index (method-shortcut-receiver-index args))
  (setv receiver (get args receiver-index))
  (when (or (unpack-iterable-form? receiver)
            (unpack-mapping-form? receiver))
    (raise (SyntaxError "kernel method call shortcut receiver cannot be an unpacking form")))
  (compile-call
    (Expression (+ [(get root 0) receiver]
                   (list (cut root 2 None))))
    (+ (list (cut args (+ receiver-index 1) None))
       (list (cut args 0 receiver-index)))))


(defn compile-call-args [args]
  (setv entries [])
  (setv seen-keyword False)
  (setv needs-statements False)
  (setv index 0)
  (while (< index (len args))
    (setv arg (get args index))
    (cond
      (isinstance arg Keyword)
        (do
          (when (>= (+ index 1) (len args))
            (raise (SyntaxError "kernel keyword arguments need a value")))
          (setv seen-keyword True)
          (setv compiled (compile-expr-isolated-pending (get args (+ index 1))))
          (when (get compiled 0)
            (setv needs-statements True))
          (.append entries ["kw" (mangle arg.name) (get compiled 0) (get compiled 1)])
          (setv index (+ index 2)))
      (unpack-mapping-form? arg)
        (do
          (setv seen-keyword True)
          (setv compiled (compile-expr-isolated-pending (get arg 1)))
          (when (get compiled 0)
            (setv needs-statements True))
          (.append entries ["kwpack" None (get compiled 0) (get compiled 1)])
          (setv index (+ index 1)))
      True
        (do
          (if (unpack-iterable-form? arg)
              (do
                (when seen-keyword
                  (setv needs-statements True))
                (setv compiled (compile-expr-isolated-pending (get arg 1)))
                (when (get compiled 0)
                  (setv needs-statements True))
                (.append entries ["star" None (get compiled 0) (get compiled 1)]))
              (do
                (when seen-keyword
                  (setv needs-statements True))
                (setv compiled (compile-expr-isolated-pending arg))
                (when (get compiled 0)
                  (setv needs-statements True))
                (.append entries ["pos" None (get compiled 0) (get compiled 1)])))
          (setv index (+ index 1)))))
  (setv compiled-args [])
  (setv keywords [])
  (setv statements [])
  (if needs-statements
      (for [entry entries]
        (setv kind (get entry 0))
        (.extend statements (get entry 2))
        (cond
          (= kind "pos")
            (.append compiled-args (get entry 3))
          (= kind "star")
            (.append compiled-args (ast.Starred :value (get entry 3)
                                               :ctx (ast.Load)))
          (= kind "kw")
            (.append keywords (ast.keyword :arg (get entry 1)
                                           :value (get entry 3)))
          (= kind "kwpack")
            (.append keywords (ast.keyword :arg None :value (get entry 3)))))
      (for [entry entries]
        (setv kind (get entry 0))
        (cond
          (= kind "pos")
            (.append compiled-args (get entry 3))
          (= kind "star")
            (.append compiled-args (ast.Starred :value (get entry 3)
                                               :ctx (ast.Load)))
          (= kind "kw")
            (.append keywords (ast.keyword :arg (get entry 1)
                                           :value (get entry 3)))
          (= kind "kwpack")
            (.append keywords (ast.keyword :arg None :value (get entry 3))))))
  [compiled-args keywords statements])


(defn compile-call-node [func-node args]
  (setv parts (compile-call-args args))
  (setv statements (get parts 2))
  (when statements
    (setv func-name (fresh-generated-name "__hy_meta_call_func_"))
    (append-pending-statement
      (ast.Assign :targets [(ast.Name :id func-name :ctx (ast.Store))]
                  :value func-node))
    (setv func-node (ast.Name :id func-name :ctx (ast.Load)))
    (for [statement statements]
      (append-pending-statement statement)))
  (ast.Call :func func-node
            :args (get parts 0)
            :keywords (get parts 1)))


(defn compile-call-ast [root args]
  (setv root-compiled (compile-expr-isolated-pending root))
  (for [statement (get root-compiled 0)]
    (append-pending-statement statement))
  (compile-call-node (get root-compiled 1) args))


(defn dotted-none-statement-root? [root]
  (and (isinstance root Expression)
       (= (len root) 2)
       (symbol-named? (expression-head root) ".")
       (none-expression-statement-head? (get root 1))))


(defn compile-call [root args]
  (cond
    (dotted-none-statement-root? root)
      (compile-statement-none-expr
        (Expression (+ [(get root 1)] (list args))))
    (and (isinstance root Expression)
         (symbol-named? (expression-head root) ".")
         (> (len root) 1)
         (symbol-named? (get root 1) "None"))
      (compile-method-shortcut-call root args)
    (isinstance root Symbol)
      (do
        (setv op (str root))
        (cond
          (in op BINOPS) (compile-binop op args)
          (in op CMPOPS) (compile-compare op args)
          (= op "chainc") (compile-chainc-expr args)
          (= op "if") (compile-if-expr args)
          (in op ["and" "or"]) (compile-boolop op args)
          (= op "not") (compile-not-expr args)
          (= op "setx") (compile-setx-expr args)
          (in op ["~" "invert" "bnot"]) (compile-invert-expr args)
          (= op "await") (compile-await-expr args)
          (= op "when") (compile-when-expr args)
          (= op "cond") (compile-cond-expr args)
          (= op "do") (compile-do-expr args)
          (= op "quote") (compile-quote-expr args)
          (= op "quasiquote") (compile-quasiquote-expr args)
          (= op "fn") (compile-fn-expr args)
          (= op "deftype") (unsupported-type-parameters-error "deftype")
          (= op "let") (compile-let-expr args)
          (= op "get") (compile-get-expr args)
          (= op "cut") (compile-cut-expr args)
          (= op "lfor") (compile-lfor-expr args)
          (= op "sfor") (compile-sfor-expr args)
          (= op "gfor") (compile-gfor-expr args)
          (= op "dfor") (compile-dfor-expr args)
          (= op ".") (compile-attribute args)
          True (compile-call-ast root args)))
    True (compile-call-ast root args)))


(defn compile-expr [model]
  "Compile an expression model to an AST node, stamping the node with the
  model's precise source span (PEP 657 fine-grained positions). Because
  sub-expressions are compiled through this same wrapper, every user-source node
  gets its own narrow position instead of inheriting the enclosing statement's
  coarse span via fix-missing-locations."
  (apply-model-location (compile-expr-raw model) model))


(defn compile-expr-raw [model]
  (cond
    (isinstance model Integer) (ast.Constant :value (int model))
    (isinstance model Float) (ast.Constant :value (float model))
    (isinstance model Complex) (ast.Constant :value (complex model))
    (isinstance model String) (ast.Constant :value (str model))
    (isinstance model Bytes) (ast.Constant :value (bytes model))
    (isinstance model FString) (compile-fstring model)
    (isinstance model Keyword) (hy-model-call "Keyword" [(ast.Constant :value model.name)])
    (isinstance model Symbol) (load-symbol model)
    (isinstance model List) (compile-sequence model ast.List)
    (isinstance model Tuple) (compile-sequence model ast.Tuple)
    (isinstance model Set) (compile-sequence model ast.Set)
    (isinstance model Dict) (compile-dict model)
    (isinstance model Expression)
     (do
       (when (= (len model) 0)
         (raise (SyntaxError "kernel cannot compile empty expressions")))
       (setv head (get model 0))
       (reject-placeholder-special-form head)
       (cond
         (symbol-named? head "do") (compile-do-expr (cut model 1 None))
         (symbol-named? head "if") (compile-if-expr (cut model 1 None))
         (symbol-named? head "when") (compile-when-expr (cut model 1 None))
         (symbol-named? head "cond") (compile-cond-expr (cut model 1 None))
         (symbol-named? head "try") (compile-try-expr (cut model 1 None))
         (symbol-named? head "with") (compile-with-expr (cut model 1 None))
         (symbol-named? head "match") (compile-match-expr (cut model 1 None))
         (symbol-named? head "py") (compile-py-expr (cut model 1 None))
         (symbol-named? head "yield") (compile-yield-expr (cut model 1 None))
         (symbol-named? head "raise") (compile-statement-none-expr model)
         (symbol-named? head "setv") (compile-setv-expr (cut model 1 None))
         (none-expression-statement-head? head) (compile-statement-none-expr model)
         True
           (do
             (when (statement-form? model)
               (raise (SyntaxError "kernel statement form used as expression")))
             (compile-call head (cut model 1 None)))))
    True (raise (SyntaxError (+ "kernel cannot compile " (repr model))))))


(defn compile-annassign [target annotation value]
  (when (or (isinstance target List) (isinstance target Tuple))
    (raise (SyntaxError "kernel annotations need a name, attribute, or subscript target")))
  (ast.AnnAssign :target (store-target target)
                 :annotation (compile-expr annotation)
                 :value value
                 :simple (if (isinstance target Symbol) 1 0)))


(defn compile-annotate-statement [args]
  (when (not (= (len args) 2))
    (raise (SyntaxError "kernel annotate needs a target and annotation")))
  [(compile-annassign (get args 0) (get args 1) None)])


(defn compile-statement-none-expr [form]
  (global PENDING-STATEMENTS)
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv statements (compile-form-with-pending-as-statements form))
  (setv PENDING-STATEMENTS outer-pending)
  (for [statement statements]
    (append-pending-statement statement))
  (ast.Constant :value None))


(defn compile-isolated-value-assignment-body [forms result-name]
  (global PENDING-STATEMENTS)
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (try
    (compile-value-assignment-body forms result-name)
    (finally
      (setv PENDING-STATEMENTS outer-pending))))


(defn compile-setv-expr [args]
  (for [statement (compile-setv args)]
    (append-pending-statement statement))
  (ast.Constant :value None))


(defn compile-setv [args]
  (setv body [])
  (setv index 0)
  (while (< index (len args))
    (setv target (get args index))
    (if (keyword-named? target "chain")
        (do
          (when (>= (+ index 2) (len args))
            (raise (SyntaxError "kernel setv :chain needs a target vector and value")))
          (setv chain-targets (get args (+ index 1)))
          (when (not (isinstance chain-targets List))
            (raise (SyntaxError "kernel setv :chain targets must be a vector")))
          (setv value (compile-expr (get args (+ index 2))))
          (.extend body (drain-pending-statements))
          (.append body
                   (ast.Assign :targets (list (map store-target chain-targets))
                               :value value))
          (setv index (+ index 3)))
        (do
          (when (>= (+ index 1) (len args))
            (raise (SyntaxError "kernel setv needs name/value pairs")))
          (setv value (compile-expr (get args (+ index 1))))
          (.extend body (drain-pending-statements))
          (.append body
                   (if (annotation-form? target)
                       (compile-annassign (annotation-target target)
                                          (annotation-type target)
                                          value)
                       (ast.Assign :targets [(store-target target)]
                                   :value value)))
          (setv index (+ index 2)))))
  body)


(defn compile-del [args]
  (when (= (len args) 0)
    (return []))
  [(ast.Delete :targets (list (map del-target args)))])


(defn augassign-target [target]
  (when (or (isinstance target List)
            (isinstance target Tuple)
            (unpack-iterable-form? target))
    (raise (SyntaxError "kernel augmented assignment target must be a name, attribute, subscript, or slice")))
  (store-target target))


(defn compile-augassign [op args]
  (when (< (len args) 2)
    (raise (SyntaxError (+ "kernel " op " needs a target and at least one value"))))
  (setv values (cut args 1 None))
  [(ast.AugAssign :target (augassign-target (get args 0))
                  :op ((get AUGOPS op))
                  :value (if (= (len values) 1)
                             (compile-expr (get values 0))
                             (compile-binop (get AUG-VALUE-OPS op) values)))])


(defn dotted-relative-symbol? [module]
  (and (isinstance module Symbol)
       (> (len (str module)) 0)
       (= (.lstrip (str module) ".") "")))


(defn relative-import-expression? [module]
  (and (isinstance module Expression)
       (> (len module) 1)
       (dotted-relative-symbol? (get module 0))
       (symbol-named? (get module 1) "None")))


(defn compile-import-target [module]
  (when (dotted-relative-symbol? module)
    (return [None (len (str module))]))
  (when (relative-import-expression? module)
    (setv parts [])
    (for [part (cut module 2 None)]
      (when (not (isinstance part Symbol))
        (raise (SyntaxError "kernel relative import module parts must be symbols")))
      (.append parts (mangle (str part))))
    (return [(if parts (.join "." parts) None)
             (len (str (get module 0)))]))
  (when (isinstance module Symbol)
    (return [(mangle (str module)) 0]))
  (when (and (isinstance module Expression)
             (> (len module) 1)
             (symbol-named? (get module 0) "."))
    (setv parts [])
    (for [part (cut module 1 None)]
      (when (not (isinstance part Symbol))
        (raise (SyntaxError "kernel dotted import module parts must be symbols")))
      (.append parts (mangle (str part))))
    (return [(.join "." parts) 0]))
  (raise (SyntaxError "kernel import module names must be symbols or dotted symbols")))


(defn compile-import-module-name [module]
  (setv target (compile-import-target module))
  (when (> (get target 1) 0)
    (raise (SyntaxError "kernel relative import needs imported names")))
  (get target 0))


(defn compile-import [args]
  (setv body [])
  (setv index 0)
  (while (< index (len args))
    (setv module (get args index))
    (setv target (compile-import-target module))
    (setv module-name (get target 0))
    (setv import-level (get target 1))
    (setv index (+ index 1))
    (cond
      (and (< index (len args))
           (symbol-named? (get args index) "*"))
        (do
          (.append body
                   (ast.ImportFrom :module module-name
                                   :names [(ast.alias :name "*" :asname None)]
                                   :level import-level))
          (setv index (+ index 1)))
      (and (< index (len args))
           (keyword-named? (get args index) "as"))
        (do
          (when (> import-level 0)
            (raise (SyntaxError "kernel relative import aliases need an imported-name list")))
          (when (>= (+ index 1) (len args))
            (raise (SyntaxError "kernel import :as needs an alias")))
          (setv alias (get args (+ index 1)))
          (when (not (isinstance alias Symbol))
            (raise (SyntaxError "kernel import aliases must be symbols")))
          (.append body
                   (ast.Import :names [(ast.alias :name module-name
                                                   :asname (mangle (str alias)))]))
          (setv index (+ index 2)))
      (and (< index (len args))
           (isinstance (get args index) List))
        (do
          (.append body
                   (ast.ImportFrom :module module-name
                                   :names (compile-import-names (get args index))
                                   :level import-level))
          (setv index (+ index 1)))
      True
        (do
          (when (> import-level 0)
            (raise (SyntaxError "kernel relative import needs imported names")))
          (.append body
                   (ast.Import :names [(ast.alias :name module-name :asname None)])))))
  body)


(defn compile-import-names [items]
  (setv names [])
  (setv index 0)
  (while (< index (len items))
    (setv name (get items index))
    (when (not (isinstance name Symbol))
      (raise (SyntaxError "kernel imported names must be symbols")))
    (setv asname None)
    (when (and (< (+ index 2) (len items))
               (keyword-named? (get items (+ index 1)) "as"))
      (setv alias (get items (+ index 2)))
      (when (not (isinstance alias Symbol))
        (raise (SyntaxError "kernel imported aliases must be symbols")))
      (setv asname (mangle (str alias)))
      (setv index (+ index 2)))
    (.append names (ast.alias :name (if (symbol-named? name "*")
                                        "*"
                                        (mangle (str name)))
                              :asname asname))
    (setv index (+ index 1)))
  names)


(defn compile-statement-body [forms]
  (setv body [])
  (for [form forms]
    (.extend body (compile-form-with-pending-as-statements form)))
  (if body body [(ast.Pass)]))


(defn compile-value-body [forms]
  (if (= (len forms) 0)
      [(ast.Return :value (ast.Constant :value None))]
      (do
        (setv body [])
        (for [form (cut forms 0 -1)]
          (.extend body (compile-form-with-pending-as-statements form)))
        (setv last-form (get forms -1))
        (if (value-preserving-statement-form? last-form)
            (do
              (setv value (compile-expr last-form))
              (.extend body (drain-pending-statements))
              (.append body (ast.Return :value value)))
            (if (statement-form? last-form)
                (do
                  (.extend body (compile-form-with-pending-as-statements last-form))
                  (.append body (ast.Return :value (ast.Constant :value None))))
                (do
                  (setv value (compile-expr last-form))
                  (.extend body (drain-pending-statements))
                  (.append body (ast.Return :value value)))))
        body)))


(defn compile-final-assignment-body [last-form result-name body]
  (if (value-preserving-statement-form? last-form)
      (do
        (setv value (compile-expr last-form))
        (.extend body (drain-pending-statements))
        (setv statement
              (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                          :value value))
        (.append body (apply-model-location statement last-form)))
      (if (statement-form? last-form)
          (do
            (.extend body (compile-form-with-pending-as-statements last-form))
            (setv statement
                  (ast.Assign :targets [(ast.Name :id result-name
                                                  :ctx (ast.Store))]
                              :value (ast.Constant :value None)))
            (.append body (apply-model-location statement last-form)))
          (do
            (setv value (compile-expr last-form))
            (.extend body (drain-pending-statements))
            (setv statement
                  (ast.Assign :targets [(ast.Name :id result-name
                                                  :ctx (ast.Store))]
                              :value value))
            (.append body (apply-model-location statement last-form)))))
  body)


(defn compile-final-module-body [last-form body]
  (if (value-preserving-statement-form? last-form)
      (do
        (setv value (compile-expr last-form))
        (.extend body (drain-pending-statements))
        (.append body (apply-model-location (ast.Expr :value value) last-form)))
      (if (statement-form? last-form)
          (.extend body (compile-form-with-pending-as-statements last-form))
          (do
            (setv value (compile-expr last-form))
            (.extend body (drain-pending-statements))
            (.append body (apply-model-location (ast.Expr :value value) last-form)))))
  body)


(defn compile-final-module-or-assignment-body [last-form result-name body]
  (if (is result-name None)
      (compile-final-module-body last-form body)
      (compile-final-assignment-body last-form result-name body)))


(defn compile-value-assignment-body [forms result-name]
  (when (= (len forms) 0)
    (return [(ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                         :value (ast.Constant :value None))]))
  (setv body [])
  (for [form (cut forms 0 -1)]
    (.extend body (compile-form-with-pending-as-statements form)))
  (compile-final-assignment-body (get forms -1) result-name body))


(defn compile-branch-as-statements [form]
  (if (and (isinstance form Expression)
           (symbol-named? (expression-head form) "do"))
      (compile-statement-body (cut form 1 None))
      (compile-form-with-pending-as-statements form)))


(defn compile-if-statement [args]
  (when (not (in (len args) [2 3]))
    (raise (SyntaxError "kernel statement if needs test, then, and optional else forms")))
  (setv test (compile-expr (get args 0)))
  (setv test-pending (drain-pending-statements))
  (+ test-pending
     [(ast.If :test test
              :body (compile-branch-as-statements (get args 1))
              :orelse (if (= (len args) 3)
                          (compile-branch-as-statements (get args 2))
                          []))]))


(defn compile-when-statement [args]
  (when (= (len args) 0)
    (raise (SyntaxError "kernel statement when needs a test expression")))
  (setv test (compile-expr (get args 0)))
  (setv test-pending (drain-pending-statements))
  (+ test-pending
     [(ast.If :test test
              :body (compile-statement-body (cut args 1 None))
              :orelse [])]))


(defn compile-cond-statement [args]
  (when (% (len args) 2)
    (raise (SyntaxError "kernel statement cond needs test/result pairs")))
  (setv body [])
  (for [index (range (- (len args) 2) -1 -2)]
    (setv test (compile-expr (get args index)))
    (setv test-pending (drain-pending-statements))
    (setv body
          (+ test-pending
             [(ast.If :test test
                      :body (compile-branch-as-statements (get args (+ index 1)))
                      :orelse body)])))
  (if body body [(ast.Pass)]))


(defn compile-match-star-pattern [model]
  (setv name (get model 1))
  (when (not (isinstance name Symbol))
    (raise (SyntaxError "kernel match #* pattern needs a symbol")))
  (ast.MatchStar :name (if (symbol-named? name "_") None (mangle (str name)))))


(defn compile-match-sequence-patterns [model]
  (setv patterns [])
  (setv seen-star False)
  (setv index 0)
  (while (< index (len model))
    (setv item (get model index))
    (when (keyword-named? item "as")
      (raise (SyntaxError "kernel match sequence :as needs a pattern and name")))
    (setv as-name None)
    (when (and (< (+ index 2) (len model))
               (keyword-named? (get model (+ index 1)) "as"))
      (setv as-name (get model (+ index 2)))
      (when (not (isinstance as-name Symbol))
        (raise (SyntaxError "kernel match sequence :as name must be a symbol")))
      (setv index (+ index 2)))
    (if (unpack-iterable-form? item)
        (do
          (when seen-star
            (raise (SyntaxError "kernel match sequence patterns only support one #* capture")))
          (when (not (is as-name None))
            (raise (SyntaxError "kernel match sequence #* patterns cannot use :as")))
          (setv seen-star True)
          (.append patterns (compile-match-star-pattern item)))
        (.append patterns (compile-match-pattern item as-name)))
    (setv index (+ index 1)))
  patterns)


(defn match-mapping-key-literal? [model]
  (or (isinstance model Integer)
      (isinstance model Float)
      (isinstance model Complex)
      (isinstance model String)
      (isinstance model Bytes)))


(defn compile-match-mapping-pattern [model]
  (setv keys [])
  (setv patterns [])
  (setv rest None)
  (setv items (model-list model))
  (when (% (len items) 2)
    (setv last-item (get items -1))
    (when (not (unpack-mapping-form? last-item))
      (raise (SyntaxError "kernel match mapping patterns need key/pattern pairs plus optional #** rest")))
    (setv rest-name (get last-item 1))
    (when (not (isinstance rest-name Symbol))
      (raise (SyntaxError "kernel match #** rest pattern needs a symbol")))
    (setv rest (if (symbol-named? rest-name "_") None (mangle (str rest-name))))
    (setv items (cut items 0 -1)))
  (for [index (range 0 (len items) 2)]
    (setv key (get items index))
    (when (not (match-mapping-key-literal? key))
      (raise (SyntaxError "kernel match mapping pattern keys must be literals")))
    (.append keys (compile-expr key))
    (.append patterns (compile-match-pattern (get items (+ index 1)))))
  (ast.MatchMapping :keys keys :patterns patterns :rest rest))


(defn compile-match-class-pattern [model]
  (setv head (get model 0))
  (setv cls (compile-expr head))
  (when (not (or (isinstance cls ast.Name)
                 (isinstance cls ast.Attribute)))
    (raise (SyntaxError "kernel match class pattern needs a name or attribute")))
  (setv patterns [])
  (setv kwd-attrs [])
  (setv kwd-patterns [])
  (setv index 1)
  (while (< index (len model))
    (setv part (get model index))
    (if (isinstance part Keyword)
        (do
          (when (>= (+ index 1) (len model))
            (raise (SyntaxError "kernel match class keyword patterns need a value")))
          (.append kwd-attrs part.name)
          (.append kwd-patterns (compile-match-pattern (get model (+ index 1))))
          (setv index (+ index 2)))
        (do
          (when kwd-attrs
            (raise (SyntaxError "kernel match class positional patterns cannot follow keyword patterns")))
          (.append patterns (compile-match-pattern part))
          (setv index (+ index 1)))))
  (ast.MatchClass :cls cls
                  :patterns patterns
                  :kwd_attrs kwd-attrs
                  :kwd_patterns kwd-patterns))


(defn compile-match-pattern [model [as-name None]]
  (if (not (is as-name None))
      (do
        (when (not (isinstance as-name Symbol))
          (raise (SyntaxError "kernel match :as name must be a symbol")))
        (ast.MatchAs :pattern (compile-match-pattern model)
                     :name (mangle (str as-name))))
      (cond
        (and (isinstance model Symbol) (= (str model) "_"))
          (ast.MatchAs :pattern None :name None)
        (and (isinstance model Symbol) (in (str model) ["None" "True" "False"]))
          (ast.MatchSingleton :value (eval (str model)))
        (isinstance model Symbol)
          (ast.MatchAs :pattern None :name (mangle (str model)))
        (isinstance model Keyword)
          (ast.MatchClass
            :cls (hy-model-class "Keyword")
            :patterns [(ast.MatchValue :value (ast.Constant :value model.name))]
            :kwd_attrs []
            :kwd_patterns [])
        (or (isinstance model Integer)
            (isinstance model Float)
            (isinstance model Complex)
            (isinstance model String)
            (isinstance model Bytes))
          (ast.MatchValue :value (compile-expr model))
        (or (isinstance model List) (isinstance model Tuple))
          (ast.MatchSequence :patterns (compile-match-sequence-patterns model))
        (isinstance model Dict)
          (compile-match-mapping-pattern model)
        (isinstance model Expression)
          (do
            (when (= (len model) 0)
              (raise (SyntaxError "kernel match cannot compile an empty class pattern")))
            (setv head (get model 0))
            (cond
              (symbol-named? head "|")
                (do
                  (when (< (len model) 3)
                    (raise (SyntaxError "kernel match | patterns need at least two alternatives")))
                  (ast.MatchOr :patterns (list (map compile-match-pattern (cut model 1 None)))))
              (symbol-named? head "as")
                (do
                  (when (not (= (len model) 3))
                    (raise (SyntaxError "kernel match as patterns need a pattern and name")))
                  (compile-match-pattern (get model 1) (get model 2)))
              (symbol-named? head ".")
                (ast.MatchValue :value (compile-expr model))
              True
                (compile-match-class-pattern model)))
        True
          (raise (SyntaxError "kernel match supports wildcard, capture, literal, keyword, sequence, mapping, class, dotted-value, or, and as patterns")))))


(defn parse-match-flat-clauses [items]
  (setv clauses [])
  (setv index 0)
  (while (< index (len items))
    (setv pattern (get items index))
    (setv as-name None)
    (setv guard None)
    (setv index (+ index 1))
    (when (and (< index (len items))
               (keyword-named? (get items index) "as"))
      (when (>= (+ index 1) (len items))
        (raise (SyntaxError "kernel match :as needs a name")))
      (setv as-name (get items (+ index 1)))
      (when (not (isinstance as-name Symbol))
        (raise (SyntaxError "kernel match :as name must be a symbol")))
      (setv index (+ index 2)))
    (when (and (< index (len items))
               (keyword-named? (get items index) "if"))
      (when (>= (+ index 2) (len items))
        (raise (SyntaxError "kernel match guard clauses need a guard and body")))
      (setv guard (get items (+ index 1)))
      (setv index (+ index 2))
      (when (and (< index (len items))
                 (keyword-named? (get items index) "as"))
        (raise (SyntaxError "kernel match :as clause cannot come after :if guard"))))
    (when (>= index (len items))
      (raise (SyntaxError "kernel match clauses need a body")))
    (.append clauses [pattern as-name guard [(get items index)]])
    (setv index (+ index 1)))
  clauses)


(defn parse-match-clauses [items]
  (parse-match-flat-clauses items))


(defn compile-match-guard [guard]
  (if (is guard None)
      None
      (if (or (statement-form? guard)
              (and (isinstance guard Expression)
                   (symbol-named? (expression-head guard) "do")))
          (do
            (global PENDING-STATEMENTS)
            (setv outer-pending PENDING-STATEMENTS)
            (setv PENDING-STATEMENTS [])
            (setv body
                  (if (and (isinstance guard Expression)
                           (symbol-named? (expression-head guard) "do"))
                      (compile-value-body (cut guard 1 None))
                      (compile-value-body [guard])))
            (setv nested-pending (drain-pending-statements))
            (setv PENDING-STATEMENTS outer-pending)
            (setv helper-name (fresh-generated-name "__hy_meta_match_guard_"))
            (append-pending-statement
              (ast.FunctionDef :name helper-name
                               :args (ast.arguments :posonlyargs []
                                                    :args []
                                                    :vararg None
                                                    :kwonlyargs []
                                                    :kw_defaults []
                                                    :kwarg None
                                                    :defaults [])
                               :body (+ nested-pending body)
                               :decorator_list []
                               :returns None
                               :type_comment None))
            (ast.Call :func (ast.Name :id helper-name :ctx (ast.Load))
                      :args []
                      :keywords []))
          (compile-expr guard))))


(defn compile-match-cases [clauses [result-name None]]
  (setv preamble [])
  (setv cases [])
  (for [clause clauses]
    (setv guard (compile-match-guard (get clause 2)))
    (.extend preamble (drain-pending-statements))
    (.append cases
             (ast.match_case
               :pattern (compile-match-pattern (get clause 0) (get clause 1))
               :guard guard
               :body (if (is result-name None)
                         (compile-statement-body (get clause 3))
                         (compile-value-assignment-body (get clause 3) result-name)))))
  [preamble cases])


(defn compile-match [args]
  (when (< (len args) 1)
    (raise (SyntaxError "kernel match needs a subject")))
  (setv subject (compile-expr (get args 0)))
  (setv subject-pending (drain-pending-statements))
  (setv clauses (parse-match-clauses (cut args 1 None)))
  (if clauses
      (do
        (setv compiled-cases (compile-match-cases clauses))
        (+ subject-pending
           (get compiled-cases 0)
           (drain-pending-statements)
           [(ast.Match :subject subject :cases (get compiled-cases 1))]))
      (+ subject-pending [(ast.Pass)])))


(defn compile-match-expr [args]
  (when (< (len args) 1)
    (raise (SyntaxError "kernel match expression needs a subject")))
  (setv result-name (fresh-generated-name "__hy_meta_match_result_"))
  (setv subject (compile-expr (get args 0)))
  (for [statement (drain-pending-statements)]
    (append-pending-statement statement))
  (append-pending-statement
    (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                :value (ast.Constant :value None)))
  (setv clauses (parse-match-clauses (cut args 1 None)))
  (when clauses
    (setv compiled-cases (compile-match-cases clauses result-name))
    (for [statement (get compiled-cases 0)]
      (append-pending-statement statement))
    (append-pending-statement
      (ast.Match :subject subject :cases (get compiled-cases 1))))
  (ast.Name :id result-name :ctx (ast.Load)))


(defn loop-else-clause? [form]
  (symbol-named? (expression-head form) "else"))


(defn split-loop-body [forms]
  (if (and (> (len forms) 0) (loop-else-clause? (get forms -1)))
      [(cut forms 0 -1) (compile-statement-body (cut (get forms -1) 1 None))]
      [forms []]))


(defn compile-while [args]
  (when (= (len args) 0)
    (raise (SyntaxError "kernel while needs a test expression")))
  (setv split (split-loop-body (cut args 1 None)))
  (setv test (compile-expr (get args 0)))
  (setv test-pending (drain-pending-statements))
  (setv body (compile-statement-body (get split 0)))
  (setv orelse (get split 1))
  (if test-pending
      [(ast.While
         :test (ast.Constant :value True)
         :body (+ test-pending
                  [(ast.If :test (ast.UnaryOp :op (ast.Not) :operand test)
                           :body (+ orelse [(ast.Break)])
                           :orelse [])]
                  body)
         :orelse [])]
      [(ast.While :test test
                  :body body
                  :orelse orelse)]))


(defn compile-for-bindings [bindings]
  (when (not (isinstance bindings List))
    (raise (SyntaxError "kernel for bindings must be a vector")))
  (setv entries [])
  (setv index 0)
  (while (< index (len bindings))
    (when (keyword-named? (get bindings index) "if")
      (raise (SyntaxError "kernel for :if needs a preceding generator")))
    (setv is-async False)
    (when (keyword-named? (get bindings index) "async")
      (setv is-async True)
      (setv index (+ index 1)))
    (when (>= (+ index 1) (len bindings))
      (raise (SyntaxError "kernel for needs target/iterable binding pairs")))
    (setv entry [is-async (get bindings index) (get bindings (+ index 1)) []])
    (setv index (+ index 2))
    (while (and (< index (len bindings))
                (keyword-named? (get bindings index) "if"))
      (when (>= (+ index 1) (len bindings))
        (raise (SyntaxError "kernel for :if needs a test expression")))
      (.append (get entry 3) (get bindings (+ index 1)))
      (setv index (+ index 2)))
    (.append entries entry))
  entries)


(defn wrap-for-filter-body [filters body]
  (setv filtered-body body)
  (for [test-form (reversed filters)]
    (setv test (compile-expr test-form))
    (setv test-pending (drain-pending-statements))
    (setv filtered-body
          (+ test-pending
             [(ast.If :test test
                      :body filtered-body
                      :orelse [])])))
  filtered-body)


(defn compile-for [args]
  (when (< (len args) 1)
    (raise (SyntaxError "kernel for needs a binding vector")))
  (setv entries (compile-for-bindings (get args 0)))
  (when (= (len entries) 0)
    (return []))
  (setv split (split-loop-body (cut args 1 None)))
  (setv body (compile-statement-body (get split 0)))
  (setv orelse (get split 1))
  (setv reversed-entries (list (reversed entries)))
  (for [index (range (len reversed-entries))]
    (setv entry (get reversed-entries index))
    (setv node-class (if (get entry 0) ast.AsyncFor ast.For))
    (setv iterable (compile-expr (get entry 2)))
    (setv entry-body (wrap-for-filter-body (get entry 3) body))
    (setv body
          (+ (drain-pending-statements)
             [(node-class :target (store-target (get entry 1))
                          :iter iterable
                          :body entry-body
                          :orelse (if (= index (- (len reversed-entries) 1)) orelse [])
                          :type_comment None)])))
  body)


(defn compile-with-items [managers]
  (when (not (isinstance managers List))
    (raise (SyntaxError "kernel with managers must be a vector")))
  (when (= (len managers) 0)
    (raise (SyntaxError "kernel with needs at least one context manager")))
  (setv entries [])
  (if (= (len managers) 1)
      (do
        (setv context-expr (compile-expr (get managers 0)))
        (setv context-pending (drain-pending-statements))
        (.append entries
                 [False
                  (ast.withitem :context_expr context-expr
                                :optional_vars None)
                  context-pending]))
      (do
        (setv index 0)
        (while (< index (len managers))
          (setv is-async False)
          (when (keyword-named? (get managers index) "async")
            (setv is-async True)
            (setv index (+ index 1)))
          (when (>= (+ index 1) (len managers))
            (raise (SyntaxError "kernel with manager pairs need target/context entries")))
          (setv target (get managers index))
          (setv context-expr (compile-expr (get managers (+ index 1))))
          (setv context-pending (drain-pending-statements))
          (.append entries
                   [is-async
                    (ast.withitem
                      :context_expr context-expr
                      :optional_vars (if (symbol-named? target "_")
                                         None
                                         (store-target target)))
                    context-pending])
          (setv index (+ index 2)))))
  entries)


(defn compile-with [args]
  (when (< (len args) 1)
    (raise (SyntaxError "kernel with needs a manager vector")))
  (setv entries (compile-with-items (get args 0)))
  (setv body (compile-statement-body (cut args 1 None)))
  (for [index (range (- (len entries) 1) -1 -1)]
    (setv entry (get entries index))
    (setv node-class (if (get entry 0) ast.AsyncWith ast.With))
    (setv body
          (+ (get entry 2)
             [(node-class :items [(get entry 1)]
                          :body body
                          :type_comment None)])))
  body)


(defn compile-with-expr [args]
  (global PENDING-STATEMENTS)
  (when (< (len args) 1)
    (raise (SyntaxError "kernel with expression needs a manager vector")))
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv entries (compile-with-items (get args 0)))
  (setv result-name (fresh-generated-name "__hy_meta_with_result_"))
  (setv body (compile-value-assignment-body (cut args 1 None) result-name))
  (setv body (+ (drain-pending-statements) body))
  (for [index (range (- (len entries) 1) -1 -1)]
    (setv entry (get entries index))
    (setv node-class (if (get entry 0) ast.AsyncWith ast.With))
    (setv body
          (+ (get entry 2)
             [(node-class :items [(get entry 1)]
                          :body body
                          :type_comment None)])))
  (setv PENDING-STATEMENTS outer-pending)
  (for [statement (+ [(ast.Assign :targets [(ast.Name :id result-name
                                                     :ctx (ast.Store))]
                                 :value (ast.Constant :value None))]
                     body)]
    (append-pending-statement statement))
  (ast.Name :id result-name :ctx (ast.Load)))


(defn try-clause? [form]
  (setv head (expression-head form))
  (or (symbol-named? head "except")
      (symbol-named? head "except*")
      (symbol-named? head "else")
      (symbol-named? head "finally")))


(defn compile-raise [args]
  (when (not (in (len args) [0 1 3]))
    (raise (SyntaxError "kernel raise accepts zero, one, or expression :from cause")))
  (when (= (len args) 3)
    (when (not (keyword-named? (get args 1) "from"))
      (raise (SyntaxError "kernel raise cause syntax is expression :from cause"))))
  [(ast.Raise :exc (if (> (len args) 0) (compile-expr (get args 0)) None)
              :cause (if (= (len args) 3) (compile-expr (get args 2)) None))])


(defn compile-pass [args]
  (when args
    (raise (SyntaxError "kernel pass accepts no arguments")))
  [(ast.Pass)])


(defn compile-assert [args]
  (when (or (< (len args) 1) (> (len args) 2))
    (raise (SyntaxError "kernel assert accepts one or two expressions")))
  [(ast.Assert :test (compile-expr (get args 0))
               :msg (if (= (len args) 2) (compile-expr (get args 1)) None))])


(defn compile-name-declaration [args node-class form-name]
  [(node-class :names (declaration-names args form-name))])


(defn compile-return [args]
  (when (> (len args) 1)
    (raise (SyntaxError "kernel return accepts zero or one expression")))
  [(ast.Return :value (if (= (len args) 1) (compile-expr (get args 0)) None))])


(defn compile-yield-expr [args]
  (when (> (len args) 2)
    (raise (SyntaxError "kernel yield accepts zero, one, or :from expression")))
  (if (and (= (len args) 2)
           (keyword-named? (get args 0) "from"))
      (ast.YieldFrom :value (compile-expr (get args 1)))
      (do
        (when (= (len args) 2)
          (raise (SyntaxError "kernel yield accepts zero, one, or :from expression")))
        (ast.Yield :value (if (= (len args) 1)
                              (compile-expr (get args 0))
                              None)))))


(defn compile-yield [args]
  [(ast.Expr :value (compile-yield-expr args))])


(defn compile-exception-type [model]
  (if (or (isinstance model List) (isinstance model Tuple))
      (ast.Tuple :elts (list (map compile-expr model))
                 :ctx (ast.Load))
      (compile-expr model)))


(defn likely-binding-symbol? [model]
  (and (isinstance model Symbol)
       (> (len (str model)) 0)
       (not (illegal-binding-symbol? model))
       (not (.isupper (get (str model) 0)))))


(defn except-spec-name-first? [spec]
  (and (= (len spec) 2)
       (isinstance (get spec 0) Symbol)
       (or (isinstance (get spec 1) List)
           (isinstance (get spec 1) Tuple)
           (and (likely-binding-symbol? (get spec 0))
                (not (likely-binding-symbol? (get spec 1)))))))


(defn parse-except-spec [spec]
  (when (not (isinstance spec List))
    (raise (SyntaxError "kernel except spec must be a vector")))
  (when (> (len spec) 2)
    (raise (SyntaxError "kernel except spec accepts [], [type], [type name], or [name type]")))
  (cond
    (= (len spec) 0)
      [None None]
    (= (len spec) 1)
      [(compile-exception-type (get spec 0)) None]
    (illegal-binding-symbol? (get spec 0))
      (raise (SyntaxError (+ "kernel cannot bind constant "
                             (str (get spec 0))
                             " in except")))
    (except-spec-name-first? spec)
      (do
        (when (not (isinstance (get spec 0) Symbol))
          (raise (SyntaxError "kernel except binding name must be a symbol")))
        (validate-binding-symbol (get spec 0) "except")
        [(compile-exception-type (get spec 1))
         (mangle (str (get spec 0)))])
    True
      (do
        (when (not (isinstance (get spec 1) Symbol))
          (raise (SyntaxError "kernel except binding name must be a symbol")))
        (validate-binding-symbol (get spec 1) "except")
        [(compile-exception-type (get spec 0))
         (mangle (str (get spec 1)))])))


(defn protected-except-binding? [name]
  (for [names LET-PROTECTED-BINDING-STACK]
    (when (in name names)
      (return True)))
  False)


(defn protected-except-handler-body [source-name temp-name body]
  (setv original-name (fresh-generated-name "__hy_meta_except_original_"))
  [(ast.Assign
     :targets [(ast.Name :id original-name :ctx (ast.Store))]
     :value (ast.Name :id source-name :ctx (ast.Load)))
   (ast.Try
     :body (+ [(ast.Assign
                 :targets [(ast.Name :id source-name :ctx (ast.Store))]
                 :value (ast.Name :id temp-name :ctx (ast.Load)))]
              body)
     :handlers []
     :orelse []
     :finalbody [(ast.Assign
                   :targets [(ast.Name :id source-name :ctx (ast.Store))]
                   :value (ast.Name :id original-name :ctx (ast.Load)))])])


(defn compile-except-handler [clause]
  (when (< (len clause) 2)
    (raise (SyntaxError "kernel except needs a spec vector")))
  (setv parsed (parse-except-spec (get clause 1)))
  (setv handler-name (get parsed 1))
  (setv body (compile-statement-body (cut clause 2 None)))
  (when (and handler-name (protected-except-binding? handler-name))
    (setv temp-name (fresh-generated-name "__hy_meta_except_"))
    (setv body (protected-except-handler-body handler-name temp-name body))
    (setv handler-name temp-name))
  (ast.ExceptHandler :type (get parsed 0)
                     :name handler-name
                     :body body))


(defn compile-except-handler-assignment [clause result-name]
  (when (< (len clause) 2)
    (raise (SyntaxError "kernel except needs a spec vector")))
  (setv parsed (parse-except-spec (get clause 1)))
  (setv handler-name (get parsed 1))
  (setv body (compile-value-assignment-body
               (cut clause 2 None)
               result-name))
  (when (and handler-name (protected-except-binding? handler-name))
    (setv temp-name (fresh-generated-name "__hy_meta_except_"))
    (setv body (protected-except-handler-body handler-name temp-name body))
    (setv handler-name temp-name))
  (ast.ExceptHandler :type (get parsed 0)
                     :name handler-name
                     :body body))


(defn reraising-handler []
  (ast.ExceptHandler :type (ast.Name :id "BaseException" :ctx (ast.Load))
                     :name None
                     :body [(ast.Raise :exc None :cause None)]))


(defn compile-try-expr [args]
  (global PENDING-STATEMENTS)
  (setv outer-pending PENDING-STATEMENTS)
  (setv PENDING-STATEMENTS [])
  (setv body-forms [])
  (setv normal-handler-forms [])
  (setv handler-forms [])
  (setv orelse [])
  (setv finalbody [])
  (setv in-clauses False)
  (setv has-except False)
  (setv has-except-star False)
  (setv has-else False)
  (for [form args]
    (if (try-clause? form)
        (do
          (setv in-clauses True)
          (setv head (expression-head form))
          (cond
            (symbol-named? head "except")
              (do
                (when (or has-else finalbody)
                  (raise (SyntaxError "kernel try expression except clauses must precede else and finally")))
                (when has-except-star
                  (raise (SyntaxError "kernel try expression cannot mix except and except* clauses")))
                (setv has-except True)
                (.append normal-handler-forms form))
            (symbol-named? head "except*")
              (do
                (when (or has-else finalbody)
                  (raise (SyntaxError "kernel try expression except clauses must precede else and finally")))
                (when has-except
                  (raise (SyntaxError "kernel try expression cannot mix except and except* clauses")))
                (setv has-except-star True)
                (.append handler-forms form))
            (symbol-named? head "else")
              (do
                (when finalbody
                  (raise (SyntaxError "kernel try expression else clause must precede finally")))
                (when has-else
                  (raise (SyntaxError "kernel try expression only supports one else clause")))
                (setv has-else True)
                (setv orelse (cut form 1 None)))
            (symbol-named? head "finally")
              (do
                (when finalbody
                  (raise (SyntaxError "kernel try expression only supports one finally clause")))
                (setv finalbody (compile-statement-body (cut form 1 None))))
            True (raise (SyntaxError "unreachable kernel try expression clause"))))
        (do
          (when in-clauses
            (raise (SyntaxError "kernel try expression body forms cannot follow clauses")))
          (.append body-forms form))))
  (setv result-name (fresh-generated-name "__hy_meta_try_result_"))
  (setv inline-statements
        [(ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                     :value (ast.Constant :value None))])
  (if has-except-star
      (do
        (setv try-body (if orelse
                           (compile-statement-body body-forms)
                           (compile-value-assignment-body body-forms result-name)))
        (setv assign-handlers [])
        (for [handler handler-forms]
          (.append assign-handlers
                   (compile-except-handler-assignment handler result-name)))
        (setv assign-orelse (if orelse
                                (compile-value-assignment-body orelse result-name)
                                []))
        (.append inline-statements
                 (ast.TryStar :body try-body
                              :handlers assign-handlers
                              :orelse assign-orelse
                              :finalbody finalbody)))
      (do
        (setv assign-handlers [])
        (for [handler normal-handler-forms]
          (.append assign-handlers
                   (compile-except-handler-assignment handler result-name)))
        (setv compiled-orelse (if has-else
                                  (compile-value-assignment-body orelse result-name)
                                  []))
        (setv try-body (if has-else
                           (compile-statement-body body-forms)
                           (compile-value-assignment-body body-forms result-name)))
        (when (and has-else (= (len assign-handlers) 0))
          (.append assign-handlers (reraising-handler)))
        (if (and (= (len assign-handlers) 0)
                 (not has-else)
                 (= (len finalbody) 0))
            (setv inline-statements try-body)
            (.append inline-statements
                     (ast.Try :body try-body
                              :handlers assign-handlers
                              :orelse compiled-orelse
                              :finalbody finalbody)))))
  (setv nested-pending (drain-pending-statements))
  (setv PENDING-STATEMENTS outer-pending)
  (for [statement (+ nested-pending inline-statements)]
    (append-pending-statement statement))
  (ast.Name :id result-name :ctx (ast.Load)))


(defn compile-try [args]
  (setv body-forms [])
  (setv handlers [])
  (setv orelse [])
  (setv finalbody [])
  (setv in-clauses False)
  (setv has-except False)
  (setv has-except-star False)
  (for [form args]
    (if (try-clause? form)
        (do
          (setv in-clauses True)
          (setv head (expression-head form))
          (cond
            (symbol-named? head "except")
              (do
                (when (or orelse finalbody)
                  (raise (SyntaxError "kernel try except clauses must precede else and finally")))
                (when has-except-star
                  (raise (SyntaxError "kernel try cannot mix except and except* clauses")))
                (setv has-except True)
                (.append handlers (compile-except-handler form)))
            (symbol-named? head "except*")
              (do
                (when (or orelse finalbody)
                  (raise (SyntaxError "kernel try except clauses must precede else and finally")))
                (when has-except
                  (raise (SyntaxError "kernel try cannot mix except and except* clauses")))
                (setv has-except-star True)
                (.append handlers (compile-except-handler form)))
            (symbol-named? head "else")
              (do
                (when finalbody
                  (raise (SyntaxError "kernel try else clause must precede finally")))
                (when orelse
                  (raise (SyntaxError "kernel try only supports one else clause")))
                (setv orelse (compile-statement-body (cut form 1 None))))
            (symbol-named? head "finally")
              (do
                (when finalbody
                  (raise (SyntaxError "kernel try only supports one finally clause")))
                (setv finalbody (compile-statement-body (cut form 1 None))))
            True (raise (SyntaxError "unreachable kernel try clause"))))
        (do
          (when in-clauses
            (raise (SyntaxError "kernel try body forms cannot follow clauses")))
          (.append body-forms form))))
  (when (and (not handlers) (not finalbody))
    (raise (SyntaxError "kernel try needs except or finally clauses")))
  (when (and orelse (not handlers))
    (raise (SyntaxError "kernel try else needs an except clause")))
  [((if has-except-star ast.TryStar ast.Try)
     :body (compile-statement-body body-forms)
     :handlers handlers
     :orelse orelse
     :finalbody finalbody)])


(defn parameter-destructure-target? [param]
  (isinstance param Tuple))


(defn parameter-name-symbol? [param]
  (and (isinstance param Symbol)
       (not (illegal-binding-symbol? param))
       (not (in (str param) ["/" "*"]))))


(defn default-parameter-form? [param]
  (and (isinstance param List)
       (= (len param) 2)
       (or (parameter-name-symbol? (get param 0))
           (annotation-form? (get param 0))
           (parameter-destructure-target? (get param 0)))))


(defn compiled-argument [name annotation]
  (validate-binding-symbol name "parameter")
  (ast.arg :arg (mangle (str name))
           :annotation (if (not (is annotation None))
                           (compile-expr annotation)
                           None)))


(defn append-parameter [target target-annotation target-args target-defaults
                        has-default default-value kwonly?
                        destructured-names destructured-values
                        destructured-prefix temp-index]
  (cond
    (parameter-name-symbol? target)
      (.append target-args (compiled-argument target target-annotation))
    (parameter-destructure-target? target)
      (do
        (setv temp-name (Symbol (+ "__hy_meta_arg_" (str temp-index))))
        (setv temp-index (+ temp-index 1))
        (.append target-args (compiled-argument temp-name target-annotation))
        (collect-destructure-bindings
          target
          (load-symbol temp-name)
          destructured-names
          destructured-values)
        (.append destructured-prefix
                 (ast.Assign :targets [(store-target target)]
                             :value (load-symbol temp-name))))
    True
      (raise (SyntaxError (+ "kernel parameters must be symbols or tuple destructuring patterns, not "
                             (repr target)))))
  (cond
    has-default
      (.append target-defaults (compile-expr default-value))
    kwonly?
      (.append target-defaults None))
  temp-index)


(defn prepare-arguments [params]
  (when (not (isinstance params List))
    (raise (SyntaxError "kernel defn parameters must be a vector")))
  (setv posonlyargs [])
  (setv args [])
  (setv defaults [])
  (setv kwonlyargs [])
  (setv kw-defaults [])
  (setv seen-default False)
  (setv seen-slash False)
  (setv kwonly-mode False)
  (setv bare-star False)
  (setv vararg None)
  (setv kwarg None)
  (setv destructured-names [])
  (setv destructured-values [])
  (setv destructured-prefix [])
  (setv temp-index 0)
  (for [param params]
    (setv param-annotation None)
    (when (annotation-form? param)
      (setv param-annotation (annotation-type param))
      (setv param (annotation-target param)))
    (cond
      (symbol-named? param "/")
        (do
          (when (not (is param-annotation None))
            (raise (SyntaxError "kernel / parameter delimiter cannot be annotated")))
          (when seen-slash
            (raise (SyntaxError "kernel only supports one / positional-only delimiter")))
          (when kwonly-mode
            (raise (SyntaxError "kernel / positional-only delimiter cannot follow keyword-only parameters")))
          (when (= (len args) 0)
            (raise (SyntaxError "kernel at least one parameter must precede /")))
          (setv seen-slash True)
          (setv posonlyargs args)
          (setv args []))
      (symbol-named? param "*")
        (do
          (when (not (is param-annotation None))
            (raise (SyntaxError "kernel * parameter delimiter cannot be annotated")))
          (when kwonly-mode
            (raise (SyntaxError "kernel only supports one keyword-only delimiter or #* vararg")))
          (when kwarg
            (raise (SyntaxError "kernel * keyword-only delimiter cannot follow #** kwargs")))
          (setv kwonly-mode True)
          (setv bare-star True))
      (parameter-name-symbol? param)
        (do
          (when kwarg
            (raise (SyntaxError "kernel parameters cannot follow #** kwargs")))
          (if kwonly-mode
              (setv temp-index
                    (append-parameter param param-annotation
                                      kwonlyargs kw-defaults
                                      False None True
                                      destructured-names destructured-values
                                      destructured-prefix temp-index))
              (do
                (when seen-default
                  (raise (SyntaxError "kernel required parameters cannot follow default parameters")))
                (setv temp-index
                      (append-parameter param param-annotation
                                        args defaults
                                        False None False
                                        destructured-names destructured-values
                                        destructured-prefix temp-index)))))
      (parameter-destructure-target? param)
        (do
          (when kwarg
            (raise (SyntaxError "kernel destructured parameters cannot follow #** kwargs")))
          (if kwonly-mode
              (setv temp-index
                    (append-parameter param param-annotation
                                      kwonlyargs kw-defaults
                                      False None True
                                      destructured-names destructured-values
                                      destructured-prefix temp-index))
              (do
                (when seen-default
                  (raise (SyntaxError "kernel required parameters cannot follow default parameters")))
                (setv temp-index
                      (append-parameter param param-annotation
                                        args defaults
                                        False None False
                                        destructured-names destructured-values
                                        destructured-prefix temp-index)))))
      (default-parameter-form? param)
        (do
          (when kwarg
            (raise (SyntaxError "kernel default parameters cannot follow #** kwargs")))
          (setv target (get param 0))
          (setv target-annotation param-annotation)
          (when (annotation-form? target)
            (when (not (is target-annotation None))
              (raise (SyntaxError "kernel parameters cannot have two annotations")))
            (setv target-annotation (annotation-type target))
            (setv target (annotation-target target)))
          (if kwonly-mode
              (setv temp-index
                    (append-parameter target target-annotation
                                      kwonlyargs kw-defaults
                                      True (get param 1) True
                                      destructured-names destructured-values
                                      destructured-prefix temp-index))
              (do
                (setv seen-default True)
                (setv temp-index
                      (append-parameter target target-annotation
                                        args defaults
                                        True (get param 1) False
                                        destructured-names destructured-values
                                        destructured-prefix temp-index)))))
      (and (unpack-iterable-form? param)
           (isinstance (get param 1) Symbol))
        (do
          (when kwarg
            (raise (SyntaxError "kernel #* varargs cannot follow #** kwargs")))
          (when vararg
            (raise (SyntaxError "kernel only supports one #* vararg parameter")))
          (when kwonly-mode
            (raise (SyntaxError "kernel #* varargs cannot follow keyword-only parameters")))
          (setv vararg (ast.arg :arg (mangle (str (get param 1)))
                                :annotation (if (not (is param-annotation None))
                                                (compile-expr param-annotation)
                                                None)))
          (setv kwonly-mode True))
      (and (unpack-mapping-form? param)
           (isinstance (get param 1) Symbol))
        (do
          (when kwarg
            (raise (SyntaxError "kernel only supports one #** kwargs parameter")))
          (setv kwarg (ast.arg :arg (mangle (str (get param 1)))
                               :annotation (if (not (is param-annotation None))
                                               (compile-expr param-annotation)
                                               None))))
      True
        (raise (SyntaxError "kernel parameters must be symbols, /, *, tuple destructuring patterns, [symbol default] pairs, #* varargs, or #** kwargs"))))
  (when (and bare-star (not kwonlyargs))
    (raise (SyntaxError "kernel named arguments must follow bare *")))
  [(ast.arguments :posonlyargs posonlyargs
                  :args args
                  :vararg vararg
                  :kwonlyargs kwonlyargs
                  :kw_defaults kw-defaults
                  :kwarg kwarg
                  :defaults defaults)
   destructured-names
   destructured-values
   destructured-prefix])


(defn compile-arguments [params]
  (get (prepare-arguments params) 0))


(defn compile-function-body [forms [allow-return-value True] [local-bindings None]]
  (global FUNCTION-SCOPE-DEPTH FUNCTION-BINDING-STACK)
  (setv FUNCTION-SCOPE-DEPTH (+ FUNCTION-SCOPE-DEPTH 1))
  (.append FUNCTION-BINDING-STACK (if (is local-bindings None) [] local-bindings))
  (try
    (do
      (setv body [])
      (if (= (len forms) 0)
          [(ast.Pass)]
          (do
            (for [form (cut forms 0 -1)]
              (.extend body (compile-form-with-pending-as-statements form)))
            (setv last-form (get forms -1))
            (if (value-preserving-statement-form? last-form)
                (do
                  (setv value (compile-expr last-form))
                  (.extend body (drain-pending-statements))
                  (if allow-return-value
                      (.append body (ast.Return :value value))
                      (.append body (ast.Expr :value value))))
                (if (statement-form? last-form)
                    (do
                      (.extend body (compile-form-with-pending-as-statements last-form))
                      (when allow-return-value
                        (.append body (ast.Return :value (ast.Constant :value None)))))
                    (do
                      (setv value (compile-expr last-form))
                      (.extend body (drain-pending-statements))
                      (if allow-return-value
                          (.append body (ast.Return :value value))
                          (.append body (ast.Expr :value value))))))
            body)))
    (finally
      (.pop FUNCTION-BINDING-STACK)
      (setv FUNCTION-SCOPE-DEPTH (- FUNCTION-SCOPE-DEPTH 1)))))


(defn compile-defn [args]
  (setv preamble [])
  (setv decorators [])
  (setv offset 0)
  (setv is-async False)
  (when (and (> (len args) 0) (keyword-named? (get args 0) "async"))
    (setv is-async True)
    (setv offset 1))
  (when (and (> (len args) offset) (keyword-named? (get args offset) "tp"))
    (unsupported-type-parameters-error "defn"))
  (when (and (> (len args) offset) (isinstance (get args offset) List))
    (setv decorators (list (map compile-expr (get args offset))))
    (.extend preamble (drain-pending-statements))
    (setv offset (+ offset 1)))
  (when (< (- (len args) offset) 2)
    (raise (SyntaxError "kernel defn needs a name and parameter vector")))
  (setv name (get args offset))
  (setv returns None)
  (when (annotation-form? name)
    (setv returns (compile-expr (annotation-type name)))
    (setv name (annotation-target name)))
  (when (not (isinstance name Symbol))
    (raise (SyntaxError "kernel defn name must be a symbol")))
  (validate-binding-symbol name "defn")
  (setv params (get args (+ offset 1)))
  (setv body-forms (cut args (+ offset 2) None))
  (setv prepared (prepare-arguments (get args (+ offset 1))))
  (.extend preamble (drain-pending-statements))
  (+ preamble
     [((if is-async ast.AsyncFunctionDef ast.FunctionDef)
        :name (mangle (str name))
        :args (get prepared 0)
        :body (+ (get prepared 3)
                 (compile-function-body
                   body-forms
                   (or (not is-async)
                       (not (any (map yield-form?
                                      body-forms))))
                   (collect-function-binding-names params body-forms)))
        :decorator_list decorators
        :returns returns
        :type_comment None)]))


(defn compile-class-bases [bases]
  (setv compiled-bases [])
  (setv keywords [])
  (setv seen-keyword False)
  (setv index 0)
  (while (< index (len bases))
    (setv item (get bases index))
    (if (isinstance item Keyword)
        (do
          (setv seen-keyword True)
          (when (>= (+ index 1) (len bases))
            (raise (SyntaxError "kernel defclass keyword bases need a value")))
          (.append keywords
                   (ast.keyword :arg item.name
                                :value (compile-expr (get bases (+ index 1)))))
          (setv index (+ index 2)))
        (do
          (when seen-keyword
            (raise (SyntaxError "kernel defclass positional bases cannot follow keyword bases")))
          (.append compiled-bases (compile-expr item))
          (setv index (+ index 1)))))
  [compiled-bases keywords])


(defn compile-defclass [args]
  (setv preamble [])
  (setv decorators [])
  (setv offset 0)
  (when (and (> (len args) 0) (keyword-named? (get args 0) "tp"))
    (unsupported-type-parameters-error "defclass"))
  (when (and (> (len args) 0) (isinstance (get args 0) List))
    (setv decorators (list (map compile-expr (get args 0))))
    (.extend preamble (drain-pending-statements))
    (setv offset 1))
  (when (< (- (len args) offset) 1)
    (raise (SyntaxError "kernel defclass needs a name")))
  (setv name (get args offset))
  (when (not (isinstance name Symbol))
    (raise (SyntaxError "kernel defclass name must be a symbol")))
  (validate-binding-symbol name "defclass")
  (setv body-offset (+ offset 1))
  (setv bases [])
  (when (and (> (len args) body-offset)
             (not (isinstance (get args body-offset) List)))
    (raise (SyntaxError "kernel defclass body needs a base vector")))
  (when (and (> (len args) body-offset)
             (isinstance (get args body-offset) List))
    (setv bases (get args body-offset))
    (setv body-offset (+ body-offset 1)))
  (setv compiled-bases (compile-class-bases bases))
  (.extend preamble (drain-pending-statements))
  (+ preamble
     [(ast.ClassDef :name (mangle (str name))
                    :bases (get compiled-bases 0)
                    :keywords (get compiled-bases 1)
                    :body (compile-statement-body (cut args body-offset None))
                    :decorator_list decorators)]))


(defn compile-form-as-statements [form]
  (if (and (statement-form? form)
           (not (expression-valued-statement-form? form)))
      (do
        (when (wrapped-annotation-form? form)
          (return (compile-annotate-statement (cut (get form 0) 1 None))))
        (setv head (get form 0))
        (setv args (cut form 1 None))
        (cond
          (symbol-named? head "setv") (compile-setv args)
          (symbol-named? head "annotate") (compile-annotate-statement args)
          (symbol-named? head "defn") (compile-defn args)
          (symbol-named? head "defclass") (compile-defclass args)
          (symbol-named? head "deftype") (unsupported-type-parameters-error "deftype")
          (symbol-named? head "import") (compile-import args)
          (symbol-named? head "pys") (compile-pys args)
          (symbol-named? head "if") (compile-if-statement args)
          (symbol-named? head "when") (compile-when-statement args)
          (symbol-named? head "cond") (compile-cond-statement args)
          (symbol-named? head "do") (compile-statement-body args)
          (symbol-named? head "while") (compile-while args)
          (symbol-named? head "for") (compile-for args)
          (symbol-named? head "match") (compile-match args)
          (symbol-named? head "with") (compile-with args)
          (symbol-named? head "try") (compile-try args)
          (symbol-named? head "raise") (compile-raise args)
          (symbol-named? head "pass") (compile-pass args)
          (symbol-named? head "assert") (compile-assert args)
          (symbol-named? head "global") (compile-name-declaration args ast.Global "global")
          (symbol-named? head "nonlocal") (compile-nonlocal-declaration args)
          (symbol-named? head "return") (compile-return args)
          (symbol-named? head "yield") (compile-yield args)
          (symbol-named? head "del") (compile-del args)
          (in (str head) AUGOPS) (compile-augassign (str head) args)
          (symbol-named? head "break")
            (do
              (when (!= (len args) 0)
                (raise (SyntaxError "kernel break takes no arguments")))
              [(ast.Break)])
          (symbol-named? head "continue")
            (do
              (when (!= (len args) 0)
                (raise (SyntaxError "kernel continue takes no arguments")))
              [(ast.Continue)])
          (symbol-named? head "defmacro")
            (raise (SyntaxError "kernel defmacro is only allowed at top level"))
          True (raise (SyntaxError "unreachable kernel statement"))))
      [(ast.Expr :value (compile-expr form))]))


(defn compile-form-with-pending-as-statements [form]
  (setv statements (compile-form-as-statements form))
  (apply-model-location-to-statements (+ (drain-pending-statements) statements)
                                      form))


(defn future-import-form? [form]
  (and (isinstance form Expression)
       (> (len form) 1)
       (symbol-named? (get form 0) "import")
       (isinstance (get form 1) Symbol)
       (= (str (get form 1)) "__future__")))


(defn compile-source-to-module [source [filename "<kernel>"] [result-name RESULT-NAME]
                                [module-name None] [module-package None]
                                [import-stdlib True]]
  (global PENDING-STATEMENTS GENERATED-NAME-INDEX LOCAL-MACRO-INDEX
          MODULE-BINDING-NAMES FUNCTION-BINDING-STACK REQUIRE-PACKAGE
          WARN-ON-CORE-SHADOW)
  (setv PENDING-STATEMENTS [])
  (setv GENERATED-NAME-INDEX 0)
  (setv LOCAL-MACRO-INDEX 0)
  (setv MODULE-BINDING-NAMES [])
  (setv FUNCTION-BINDING-STACK [])
  (setv REQUIRE-PACKAGE module-package)
  (setv WARN-ON-CORE-SHADOW True)
  (setv target-module (if module-name (.get sys.modules module-name) None))
  (when (is target-module None)
    (setv target-module (types.ModuleType (or module-name "hy_meta_kernel.session")))
    (setv target-module.__package__ module-package))
  (setv forms (prepare-forms (read-many source :filename filename)
                             target-module
                             filename))
  (collect-let-body-binding-names forms MODULE-BINDING-NAMES [])
  (setv body [])
  (when (and (> (len forms) 1)
             (isinstance (get forms 0) String))
    (.append body
             (ast.Expr :value (ast.Constant :value (str (get forms 0)))))
    (setv forms (cut forms 1 None)))
  (while (and (> (len forms) 0)
              (future-import-form? (get forms 0)))
    (.extend body (compile-form-with-pending-as-statements (get forms 0)))
    (setv forms (cut forms 1 None)))
  (when import-stdlib
    (.append body (ast.Import :names [(ast.alias :name "hy" :asname None)])))
  (if (= (len forms) 0)
      (when (not (is result-name None))
        (.append body
                 (ast.Assign :targets [(ast.Name :id result-name :ctx (ast.Store))]
                             :value (ast.Constant :value None))))
      (do
        (for [form (cut forms 0 -1)]
          (.extend body (compile-form-with-pending-as-statements form)))
        (compile-final-module-or-assignment-body (get forms -1) result-name body)))
  (ast.fix-missing-locations (ast.Module :body body :type_ignores [])))


(defn python-source [source [filename "<kernel>"]]
  (ast.unparse (compile-source-to-module source filename)))


(defn eval-source [source [module None] [filename "<kernel>"]]
  (setv module (or module (types.ModuleType "hy_meta_kernel.session")))
  (setv module-name module.__name__)
  (setv module-package (getattr module "__package__" None))
  (setv module-cache-present (in module-name sys.modules))
  (setv previous-module (.get sys.modules module-name))
  (setv (get sys.modules module-name) module)
  (try
    (setv module.hy hy)
    (setv code (compile (compile-source-to-module source filename RESULT-NAME
                                                  module-name module-package)
                        filename
                        "exec"))
    (exec code module.__dict__)
    (get module.__dict__ RESULT-NAME)
    (finally
      (if module-cache-present
          (setv (get sys.modules module-name) previous-module)
          (.pop sys.modules module-name None)))))


(defn self-check []
  (and
    (= (eval-source "(defn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) (fact 5)")
       120)
    (= (eval-source "(((fn [x] (fn [y] (+ x y))) 10) 32)")
       42)
    (= (str (eval-source "'alpha"))
       "alpha")
    (= (eval-source "\"kernel doc\" (setv x 42) [__doc__ x]")
       ["kernel doc" 42])
    (= (eval-source "(do 1 2 3)")
       3)
    (= (eval-source "(try 1)")
       1)
    (= (eval-source "(try (/ 1 0) (except [ZeroDivisionError] 42))")
       42)
    (= (eval-source "(import contextlib) (with [x (contextlib.nullcontext 41)] (+ x 1))")
       42)
    (= (eval-source "(defn f [] (try (/ 1 0) (except [ZeroDivisionError] 42))) (f)")
       42)
    (= (eval-source "(import contextlib) (defn f [] (with [x (contextlib.nullcontext 41)] (+ x 1))) (f)")
       42)
    (= (eval-source "(setv x \"a\") (setv y (do (setv x \"b\") \"c\")) [x y]")
       ["b" "c"])
    (= (eval-source "(when True 40 42)")
       42)
    (= (eval-source "(and True 42)")
       42)
    (= (eval-source "(or False 42)")
       42)
    (= (eval-source "(not False)")
       True)
    (= (eval-source "(if (setx x 42) x 0)")
       42)
    (= (eval-source "(+ (% 20 6) (// 20 3) (** 2 5) (& 7 3) (invert -1) (- (| 8 1) 10))")
       42)
    (= (eval-source "(bnot 0b00101111)")
       -48)
    (= (eval-source "(in 3 [1 2 3])")
       True)
    (= (eval-source "(not-in 4 [1 2 3])")
       True)
    (= (eval-source "(is None None)")
       True)
    (= (eval-source "(setv e Ellipsis) (setv Ellipsis 14) (and (= Ellipsis 14) (!= ... 14) (is ... e))")
       True)
    (= (eval-source "(= (when False 42) None)")
       True)
    (= (eval-source "(cond False 1 True 42)")
       42)
    (= (eval-source "(setv x 0) (when True (setv x 42)) x")
       42)
    (= (eval-source "(setv x 0) (when (do (setv x 1) True) (setv x (+ x 41))) x")
       42)
    (= (eval-source "(setv x 0) (cond False (setv x 1) True (setv x 42)) x")
       42)
    (= (eval-source "(setv x 0) (cond (do (setv x 1) False) (setv x 0) (do (setv x (+ x 1)) True) (setv x (+ x 40))) x")
       42)
    (= (eval-source "(import math) (.sqrt math 81)")
       9.0)
    (= (eval-source "(import math :as m) (.sqrt m 1764)")
       42.0)
    (= (eval-source "(import math [sqrt :as root]) (root 1764)")
       42.0)
    (= (eval-source "(import math *) (sqrt 1764)")
       42.0)
    (in "import a_b.c_d"
        (python-source "(import a-b.c-d)" "<kernel:import-mangle>"))
    (in "from a_b import c"
        (python-source "(import a-b [c])" "<kernel:import-from-mangle>"))
    (in "from .sibling import sibling_value"
        (python-source "(import .sibling [sibling-value])" "<kernel:relative-import>"))
    (in "from .. import resources"
        (python-source "(import .. [resources])" "<kernel:relative-import-parent>"))
    (= (eval-source "(defmacro inc [x] (Expression [(Symbol \"+\") x (Integer 1)])) (inc 41)")
       42)
    (= (eval-source "(defmacro answer [] '42) (answer)")
       42)
    (= (eval-source "(defmacro incq [x] `(+ ~x 1)) (incq 41)")
       42)
    (= (eval-source "(defmacro add-all [xs] `(+ ~@xs)) (add-all [10 20 12])")
       42)
    (= (eval-source "(require builtins) (builtins.defn required-defn [] 42) (required-defn)")
       42)
    (= (eval-source "(hy.R.tests/resources/tlib.qplah 1 2 3)")
       [8 1 2 3])
    (= (eval-source "(defclass C [] (setv answer 42)) (. C answer)")
       42)
    (= (eval-source "(defn dec [f] (fn [] (+ (f) 1))) (defn [dec] f [] 41) (f)")
       42)
    (= (eval-source "(defn cdec [cls] (setattr cls \"answer\" 42) cls) (defclass [cdec] C []) (. C answer)")
       42)
    (= (eval-source "(defclass C []) (setv (. C answer) 42) (. C answer)")
       42)
    (= (eval-source "(get [10 32] 1)")
       32)
    (= (eval-source "(. [10 20 12] [1])")
       20)
    (= (eval-source "(. \"ab hello\" (strip \"ab \") (upper))")
       "HELLO")
    (= (eval-source "(setv xs [1 2 3]) (setv (get xs 1) 38) (+ (get xs 0) (get xs 1) (get xs 2))")
       42)
    (= (eval-source "(setv xs [0 1 2 3]) (setv (cut xs 1 3) [20 22]) xs")
       [0 20 22 3])
    (= (eval-source "(setv xs [0 1 2 3]) (del (cut xs 1 3)) xs")
       [0 3])
    (= (eval-source "(setv xs [10 20 12]) (setv (. xs [1]) 40) (+ (get xs 1) 2)")
       42)
    (= (eval-source "(= (cut [0 10 20 30] 1 3) [10 20])")
       True)
    (= (eval-source "(= [(lfor 1) (sfor 1) (list (gfor 1)) (dfor 1 2)] [[] #{} [] {}])")
       True)
    (= (eval-source "(= (lfor x [1 2 3] (* x 2)) [2 4 6])")
       True)
    (= (eval-source "(= (lfor x [1 2 3] :if (> x 1) x) [2 3])")
       True)
    (= (eval-source "(= (lfor x [[1 2] [3 4] [5]] #* x) [1 2 3 4 5])")
       True)
    (= (eval-source "(= (sfor x [1 1 2] x) #{1 2})")
       True)
    (= (eval-source "(= (sfor x [[1 2] [2 3]] #* x) #{1 2 3})")
       True)
    (= (eval-source "(= (dfor x [1 2] x (* x x)) {1 1 2 4})")
       True)
    (= (eval-source "(sum (gfor x [1 2 3] x))")
       6)
    (= (eval-source "(= (list (gfor x [[1 2] [3 4] [5]] #* x)) [1 2 3 4 5])")
       True)
    (= (eval-source "(setv seen []) (= (list (gfor x [[1 2] [3 4] [5]] :do (.append seen (len x)) #* x)) [1 2 3 4 5])")
       True)
    (= (eval-source "(defn sub [] (setv x (yield \"first\")) (yield (+ \"received: \" (str x))) (yield \"last\")) (setv g (gfor f [sub] #* (f))) [(next g) (.send g \"hello\") (next g)]")
       ["first" "received: None" "last"])
     (= (eval-source "(setv s \"\") [(lfor x (do (setv s \"x\") \"ab\") y (do (+= s \"y\") \"def\") (+ x y s)) s]")
       [["adxy" "aexy" "afxy" "bdxyy" "bexyy" "bfxyy"] "xyy"])
    (= (eval-source "(setv seen []) [(lfor x (range 3) :if (do (.append seen x) (% x 2)) x) seen]")
       [[1] [0 1 2]])
    (= (eval-source "(setv x 1) (+= x 2 3) (*= x 7) x")
       42)
    (= (eval-source "(setv x 45) (%= x 43) (<<= x 4) (|= x 10) x")
       42)
    (= (eval-source "(setv xs [1 2 3]) (del (get xs 1)) (+ (len xs) (get xs 1) 37)")
       42)
    (= (eval-source "(let [x 10 y 32] (+ x y))")
       42)
    (= (eval-source "(let [a \"a\" b (+ a \"b\") c (+ b \"c\")] c)")
       "abc")
    (= (eval-source "(let [x \"foo\" y \"bar\" x (+ x y) y (+ y x) x (+ x x)] [x y])")
       ["foobarfoobar" "barfoobar"])
    (= (eval-source "(let [[head #* tail] (range 3)] [head tail])")
       [0 [1 2]])
    (= (eval-source "(let [[nhead #* #(c #* nrest)] [0 1 2]] [nhead c nrest])")
       [0 1 [2]])
    (= (eval-source "(let [(annotate x int) 42] x)")
       42)
    (= (eval-source "(setv n 5 acc 1) (while (> n 1) (setv acc (* acc n)) (setv n (- n 1))) acc")
       120)
    (= (eval-source "(setv acc 0) (for [x [1 2 3]] (setv acc (+ acc x))) acc")
       6)
    (= (eval-source "(setv acc 0) (for [x [1 2] y [10 20]] (setv acc (+ acc x y))) acc")
       66)
    (= (eval-source "(setv l []) (for [] (.append l 1)) l")
       [])
    (= (eval-source "(setv s \"\") (setv out []) (for [x \"ab\" y (do (+= s \"y\") \"de\")] (.append out (+ x y s))) [out s]")
       [["ady" "aey" "bdyy" "beyy"] "yy"])
    (= (eval-source "(setv acc 0) (for [x [1 2]] (setv acc (+ acc x)) (else (setv acc (+ acc 39)))) acc")
       42)
    (= (eval-source "(import asyncio) (defn :async numbers [] (for [i [1 2]] (yield i))) (defn :async use [] (setv x 0) (for [:async a (numbers)] (setv x (+ x a)) (else (setv x (+ x 39)))) x) (asyncio.run (use))")
       42)
    (= (eval-source "(setv out 0) (match 3 1 (setv out 1) 3 (setv out 42) _ (setv out 0)) out")
       42)
    (= (eval-source "(setv out 0) (match [1 2] [a b] (setv out (+ a b 39)) _ (setv out 0)) out")
       42)
    (= (eval-source "(setv out 0) (match {\"x\" 10 \"y\" 32} {\"x\" a \"y\" b} (setv out (+ a b)) _ (setv out 0)) out")
       42)
    (= (eval-source "(setv out 0) (match [10 32] [a b] :if (= (+ a b) 0) (setv out 1) [a b] :if (= (+ a b) 42) (setv out (+ a b)) _ (setv out 0)) out")
       42)
    (= (eval-source "(defclass P [] (setv __match_args__ (tuple [\"x\" \"y\"])) (defn __init__ [self x y] (setv (. self x) x) (setv (. self y) y))) (setv out 0) (match (P 10 32) (P a b) (setv out (+ a b)) _ (setv out 0)) out")
       42)
    (= (eval-source "(setv out 0) (match 2 (| 1 2) (setv out 42) _ (setv out 0)) out")
       42)
    (= (eval-source "(setv out 0) (match [10 32] (as [a b] pair) (setv out (+ (get pair 0) (get pair 1))) _ (setv out 0)) out")
       42)
    (= (eval-source "(match 0 0 :if True 42 _ 0)")
       42)
    (= (eval-source "(match [0 1 2] [0 #* xs] :as whole :if (do (setv size (len whole)) (= size 3)) (sum xs) _ 0)")
       3)
    (= (eval-source "(= (match :hello :hello ':ok _ ':bad) ':ok)")
       True)
    (= (eval-source "(import contextlib [nullcontext]) (setv x 0) (with [value (nullcontext 42)] (setv x value)) x")
       42)
    (= (eval-source "(import contextlib [nullcontext]) (setv x (with [value (nullcontext 40)] (+ value 2))) x")
       42)
    (= (eval-source "(import contextlib [nullcontext]) (setv y 0) (setv x (with [value (nullcontext 40)] (setv y 2) (+ value y))) [x y]")
       [42 2])
    (= (eval-source "(import contextlib [nullcontext]) (setv x 1) (with [_ (nullcontext) y (nullcontext 41)] (setv x (+ x y))) x")
       42)
    (= (eval-source "(import asyncio) (defclass ACtx [] (defn __init__ [self value] (setv (. self value) value)) (defn :async __aenter__ [self] (. self value)) (defn :async __aexit__ [self exc-type exc-value traceback] False)) (defn :async use [] (setv x 0) (with [:async value (ACtx 42)] (setv x value)) x) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defclass ACtx [] (defn __init__ [self value] (setv (. self value) value)) (defn :async __aenter__ [self] (. self value)) (defn :async __aexit__ [self exc-type exc-value traceback] False)) (defn :async use [] (return (with [:async value (ACtx 40)] (+ value 2)))) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defclass ACtx [] (defn __init__ [self value] (setv (. self value) value)) (defn :async __aenter__ [self] (. self value)) (defn :async __aexit__ [self exc-type exc-value traceback] False)) (defn :async use [] (setv y 0) (setv x (with [:async value (ACtx 40)] (setv y 2) (+ value y))) [x y]) (asyncio.run (use))")
       [42 2])
    (= (eval-source "(setv x 0) (try (raise (ValueError \"bad\")) (except [ValueError e] (setv x 42))) x")
       42)
    (= (eval-source "(setv x (try (+ 20 22))) x")
       42)
    (= (eval-source "(setv x (try 1 (else (+ 20 22)))) x")
       42)
    (= (eval-source "(setv x (try (raise (ValueError \"bad\")) (except [ValueError e] (+ 20 22)))) x")
       42)
    (= (eval-source "(setv x (try (get \"foo\" 5) (except [[IndexError NameError]] 42))) x")
       42)
    (= (eval-source "(setv x (try (abs \"hi\") (except [e TypeError] (is (type e) TypeError)))) x")
       True)
    (= (eval-source "(setv x (try (get {1 2} 3) (except [e [KeyError AttributeError]] (is (type e) KeyError)))) x")
       True)
    (= (eval-source "(setv y 0) (setv x (try (raise (ValueError \"bad\")) (except [ValueError e] (setv y 42) \"ok\"))) [x y]")
       ["ok" 42])
    (= (eval-source "(setv seen []) (setv a 1 b (try (.append seen a) (setv a 2) 3)) [a b seen]")
       [2 3 [1]])
    (= (eval-source "(setv seen []) (setv x (try (raise (ExceptionGroup \"bad\" [(KeyError \"k\") (ValueError \"v\")])) (except* [KeyError e] (.append seen \"k\") \"key\") (except* [ValueError e] (.append seen \"v\") \"value\") (finally (.append seen \"f\")))) [seen x]")
       [["k" "v" "f"] "value"])
    (= (eval-source "(setv ok False) (try (try (raise (ValueError \"inner\")) (except [ValueError e] (raise (RuntimeError \"outer\") :from e))) (except [RuntimeError e] (setv ok (isinstance (getattr e \"__cause__\") ValueError)))) ok")
       True)
    (= (eval-source "(setv x 0) (try (raise (ExceptionGroup \"bad\" [(ValueError \"bad\")])) (except* [ValueError e] (setv x 42))) x")
       42)
    (= (eval-source "(setv x 0) (try (setv x 1) (finally (setv x (+ x 41)))) x")
       42)
    (= (eval-source "(setv i 0 acc 0) (while (< i 5) (setv i (+ i 1)) (if (= i 3) (continue) 0) (setv acc (+ acc i))) acc")
       12)
    (= (eval-source "(setv i 0) (while True (setv i (+ i 1)) (if (= i 3) (break) 0)) i")
       3)
    (= (eval-source "(while False) 42")
       42)
    (= (eval-source "(setv i 0) (while (< i 2) (setv i (+ i 1)) (else (setv i (+ i 40)))) i")
       42)
    (= (eval-source "(setv s \"\") (setv x 2) (while (do (+= s \"a\") x) (+= s \"b\") (-= x 1) (else (+= s \"z\"))) s")
       "ababaz")
    (= (eval-source "(setv s \"\") (setv x 2) (setv continued False) (while (do (+= s \"a\") x) (+= s \"b\") (when (and (= x 1) (not continued)) (+= s \"c\") (setv continued True) (continue)) (-= x 1) (else (+= s \"z\"))) s")
       "ababcabaz")
    (= (eval-source "(setv s \"\") (for [x \"123\"] (+= s x) (setv y 0) (while (do (when (and (= x \"2\") (= y 1)) (break)) (< y 3)) (+= s \"y\") (+= y 1))) s")
       "1yyy2y3yyy")
    (= (eval-source "(defn f [x [y 32]] (+ x y)) (f 10)")
       42)
    (= (eval-source "((fn [x [y 32]] (+ x y)) 10 1)")
       11)
    (= (eval-source "(defn f [x [y 32]] (+ x y)) (f 10 1)")
       11)
    (= (eval-source "(defn f [x #* xs] (+ x (len xs))) (f 10 20 30)")
       12)
    (= (eval-source "((fn [#* xs] (len xs)) 1 2 3)")
       3)
    (= (eval-source "(defn spread [#* xs] (len xs)) (spread #* [1 2 3])")
       3)
    (= (eval-source "(defn f [x #* xs] (+ x (len xs))) (f #* [10 20 30])")
       12)
    (= (eval-source "(setv xs [2 3]) (= [1 #* xs 4] [1 2 3 4])")
       True)
    (= (eval-source "(setv d {\"b\" 2}) (= {\"a\" 1 #** d} {\"a\" 1 \"b\" 2})")
       True)
    (= (eval-source "(setv xs [1 2 3] ys [4 5]) (+ #* xs #* ys)")
       15)
    (= (eval-source "(* #* [2 3 7])")
       42)
    (= (eval-source "(* #* [])")
       1)
    (= (eval-source "(and #* [1 2 3])")
       3)
    (= (eval-source "(or #* [False 0 42])")
       42)
    (= (eval-source "(< #* [1 2 3])")
       True)
    (= (eval-source "(= #* [1 1 1])")
       True)
    (= (eval-source "(= #* [1 2 1])")
       False)
    (= (eval-source "(+ #* [[1] [2]])")
       [1 2])
    (= (eval-source "(setv :chain [a b c] 3) [a b c]")
       [3 3 3])
    (= (eval-source "(defn an [x] (is x None)) (setv x 1) [(an (setv x 2)) x]")
       [True 2])
    (= (eval-source "(setv p (setv q 12)) [p q]")
       [None 12])
    (= (eval-source "(defn an [x] (is x None)) [(an (setv)) (an (setv :chain [a b] 3)) a b]")
       [True True 3 3])
    (= (eval-source "(defn an [x] (is x None)) [(an (setv x (defn phooey [] 7))) x (phooey)]")
       [True None 7])
    (= (eval-source "(defn an [x] (is x None)) (setv seen []) [(an (setv x (for [i (range 3)] (.append seen i)))) x seen]")
       [True None [0 1 2]])
    (= (eval-source "(defn an [x] (is x None)) [(an (setv x (assert True))) x (an (pass))]")
       [True None True])
    (= (eval-source "(setv v1 1 :chain [v2 v3] 2 v4 4 :chain [v5 v6 v7] 5) [v1 v2 v3 v4 v5 v6 v7]")
       [1 2 2 4 5 5 5])
    (= (eval-source "(setv :chain [[y #* z w] x [a b c d]] \"abcd\") [y z w x a b c d]")
       ["a" ["b" "c"] "d" "abcd" "a" "b" "c" "d"])
    (= (eval-source "(setv l (* [0] 5)) (setv calls []) (defn f [i] (.append calls [i (list l)]) i) (setv :chain [(get l (f 1)) (get l (f 2)) (get l (f 3))] (f 9)) [calls l]")
       [[[9 [0 0 0 0 0]] [1 [0 0 0 0 0]] [2 [0 9 0 0 0]] [3 [0 9 9 0 0]]] [0 9 9 9 0]])
    (= (eval-source "(setv [a #* rest] [10 20 12]) (+ a (sum rest))")
       42)
    (= (eval-source "(dfor pair [[\"a\" 10] [\"b\" 32]] #** {(get pair 0) (get pair 1)})")
       {"a" 10 "b" 32})
    (= (eval-source "(defn f [x [y 32]] (+ x y)) (f :x 10)")
       42)
    (= (eval-source "(defn f [* [#(x y) [20 22]]] (+ x y)) (f)")
       42)
    (= (eval-source "(defn f [* #(x y)] (+ x y)) (f :__hy_meta_arg_0 [20 22])")
       42)
    (= (eval-source "(defn f [#** kw] (.get kw \"x\")) (f :x 42)")
       42)
    (= (eval-source "(defn f [x #* xs #** kw] (+ x (len xs) (.get kw \"y\"))) (f 10 #* [20 30] :y 30)")
       42)
    (= (eval-source "(defn f [#** kw] (.get kw \"x\")) (f #** {\"x\" 42})")
       42)
    (= (eval-source "(defn f [] (return 42) 0) (f)")
       42)
    (= (eval-source "(defn gen [] (yield 10) (yield 20)) (sum (gen))")
       30)
    (= (eval-source "(defn gen [] (yield 10) (yield :from [20 12])) (list (gen))")
       [10 20 12])
    (= (eval-source "(defn f [] (yield :from) (yield :from)) (list (f))")
       [:from :from])
    (= (eval-source "(defn sub [] (yield 1) (yield 2) (/ 1 0)) (defn gen [] (try (yield :from (sub)) (except [ZeroDivisionError] (yield 39)))) (sum (gen))")
       42)
    (= (eval-source "(defn gen [] (setv x (yield \"first\")) (yield (+ \"received: \" x)) (yield \"last\")) (setv g (gen)) [(next g) (.send g \"hello\") (next g)]")
       ["first" "received: hello" "last"])
    (= (eval-source "(defn sub [] (yield 10) (return 32)) (defn gen [] (setv value (yield :from (sub))) (yield value)) (list (gen))")
       [10 32])
    (= (eval-source "(pass) 42")
       42)
    (= (eval-source "(assert (= (+ 20 22) 42) \"sum still works\") 42")
       42)
    (= (eval-source "(setv x 0) (defn f [] (global x) (setv x 42)) (f) x")
       42)
    (= (eval-source "(defn outer [] (setv x 0) (defn inner [] (nonlocal x) (setv x 42)) (inner) x) (outer)")
       42)
    (= (eval-source "(import asyncio) (defn :async coro [] 42) (asyncio.run (coro))")
       42)
    (= (eval-source "(import asyncio) (asyncio.run ((fn :async [x] (await (asyncio.sleep 0)) (+ x 2)) 40))")
       42)
    (= (eval-source "(import asyncio) (defn :async agen [] (yield 20) (yield 22)) (defn :async use [] (setv total 0) (for [:async x (agen)] (setv total (+ total x))) total) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defn :async use [] (setv total 0) (setv agen (fn :async [] (yield 20) (yield 22))) (for [:async x (agen)] (setv total (+ total x))) total) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defn :async inner [] 40) (defn :async outer [] (+ (await (inner)) 2)) (asyncio.run (outer))")
       42)
    (= (eval-source "(import asyncio) (defn :async inner [x] x) (defn :async use [] (return (try (await (inner 42)) (except [ValueError e] 0)))) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defn :async inner [x] x) (defn :async use [] (return (try (raise (ValueError \"bad\")) (except [ValueError e] (await (inner 42)))))) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defn :async inner [x] x) (defn :async use [] (return (try (raise (ExceptionGroup \"bad\" [(ValueError \"v\")])) (except* [ValueError e] (await (inner 42)))))) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defn :async inner [x] x) (defn :async use [] (setv y 0) (setv x (try (raise (ValueError \"bad\")) (except [ValueError e] (setv y 42) (await (inner \"ok\"))))) [x y]) (asyncio.run (use))")
       ["ok" 42])
    (= (eval-source "(import asyncio) (defclass AValues [] (defn __init__ [self values] (setv (. self values) (iter values))) (defn __aiter__ [self] self) (defn :async __anext__ [self] (try (return (next (. self values))) (except [StopIteration] (raise StopAsyncIteration))))) (defn :async use [] (setv total 0) (for [:async x (AValues [10 20 12])] (setv total (+ total x))) total) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defclass AValues [] (defn __init__ [self values] (setv (. self values) (iter values))) (defn __aiter__ [self] self) (defn :async __anext__ [self] (try (return (next (. self values))) (except [StopIteration] (raise StopAsyncIteration))))) (defn :async use [] (sum (lfor :async x (AValues [10 20 12]) x))) (asyncio.run (use))")
       42)
    (= (eval-source "(import asyncio) (defclass AValues [] (defn __init__ [self values] (setv (. self values) (iter values))) (defn __aiter__ [self] self) (defn :async __anext__ [self] (try (return (next (. self values))) (except [StopIteration] (raise StopAsyncIteration))))) (defn :async use [] (setv seen []) (setv values (lfor :async x (AValues [1 2 3]) :do (.append seen x) :setv y (* x 2) y)) [(sum values) seen]) (asyncio.run (use))")
       [12 [1 2 3]])
    (= (eval-source "(import asyncio) (defclass AValues [] (defn __init__ [self values] (setv (. self values) (iter values))) (defn __aiter__ [self] self) (defn :async __anext__ [self] (try (return (next (. self values))) (except [StopIteration] (raise StopAsyncIteration))))) (defn :async use [] (setv values []) (for [:async value (gfor :async x (AValues [1 2 3 4]) :do (when (= x 4) (break)) x)] (.append values value)) values) (asyncio.run (use))")
       [1 2 3])
    (= (eval-source "(setv [x y] [10 32]) (+ x y)")
       42)
    (= (eval-source "(setv (annotate x int) 42) (and (= x 42) (in (get __annotations__ \"x\") [int \"int\"]))")
       True)
    (= (eval-source "(annotate y str) (in (get __annotations__ \"y\") [str \"str\"])")
       True)
    (= (eval-source "(defn #^ int f [#^ int x] x) (and (= (f 42) 42) (in (get (getattr f \"__annotations__\") \"x\") [int \"int\"]) (in (get (getattr f \"__annotations__\") \"return\") [int \"int\"]))")
       True)
    (= (eval-source "(setv f (fn #^ int [#^ int x] x)) (and (= (f 42) 42) (in (get (getattr f \"__annotations__\") \"x\") [int \"int\"]) (in (get (getattr f \"__annotations__\") \"return\") [int \"int\"]))")
       True)
    (= (eval-source "(setv [x [y z]] [10 [20 12]]) (+ x y z)")
       42)
    (= (eval-source "(let [[x y] [10 32]] (+ x y))")
       42)
    (= (eval-source "(let [[(annotate x int) y] [10 32]] (+ x y))")
       42)
    (= (eval-source "(let [[x [y z]] [10 [20 12]]] (+ x y z))")
       42)
    (= (eval-source "(defn f [#(x y)] (+ x y)) (f [10 32])")
       42)
    (= (eval-source "(defn f [#^ tuple #(x y)] (+ x y)) (f [10 32])")
       42)
    (= (eval-source "((fn [#(x y)] (+ x y)) [10 32])")
       42)
    (= (eval-source "((fn [#^ tuple #(x y)] (+ x y)) [10 32])")
       42)
    (= (eval-source "(defn f [[#(x y) [10 32]]] (+ x y)) (f)")
       42)
    (= (eval-source "(defn f [[#^ tuple #(x y) [10 32]]] (+ x y)) (f)")
       42)))
