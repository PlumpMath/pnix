(ns pnix.clr-meta.compiler-kernel.v1)

(def kernel-identity "pnix.clr-meta.compiler-kernel.v1")

(def kernel-namespace "pnix.clr-meta.compiler-kernel.v1")

(def kernel-entry "compile-source")

(def source-limits-id "pnix.clr-meta.compiler-kernel-source-limits.v1")

(def source-profile-id "pnix.clr-meta.compiler-kernel-source.v1")

(def add-opcode "add.ovf")

(def subtract-opcode "sub.ovf")

(def equality-opcode "ceq")

(def less-than-opcode "clt")

(def symbol-named?
  (fn* symbol-named? [form expected]
    (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "symbol")
      (pnix.clr-meta.compiler-support.data.v1/string-equal?
        (pnix.clr-meta.compiler-support.data.v1/symbol-name form)
        expected)
      false)))

(def count-is?
  (fn* count-is? [form expected]
    (= (pnix.clr-meta.compiler-support.data.v1/count form) expected)))

(def count-at-least?
  (fn* count-at-least? [form minimum]
    (if (< (pnix.clr-meta.compiler-support.data.v1/count form) minimum)
      false
      true)))

(def binding-kind?
  (fn* binding-kind? [binding expected]
    (pnix.clr-meta.compiler-support.data.v1/string-equal?
      (pnix.clr-meta.compiler-support.data.v1/nth binding 0)
      expected)))

(def binding-arity-is?
  (fn* binding-arity-is? [binding expected]
    (= (pnix.clr-meta.compiler-support.data.v1/nth binding 2) expected)))

(def callable-binding?
  (fn* callable-binding? [binding]
    (if (binding-kind? binding "kernel-call")
      true
      (if (binding-kind? binding "support-call")
        true
        (binding-kind? binding "intrinsic")))))

(def bind-support-call
  (fn* bind-support-call [env name arity]
    (pnix.clr-meta.compiler-support.data.v1/env-bind
      env
      name
      "support-call"
      name
      arity)))

(def seed-support-calls
  (fn* seed-support-calls [env]
    (let* [e01 (bind-support-call env "pnix.clr-meta.compiler-support.reader.v1/read-all" 2)
           e02 (bind-support-call e01 "pnix.clr-meta.compiler-support.data.v1/kind-is?" 2)
           e03 (bind-support-call e02 "pnix.clr-meta.compiler-support.data.v1/count" 1)
           e04 (bind-support-call e03 "pnix.clr-meta.compiler-support.data.v1/nth" 2)
           e05 (bind-support-call e04 "pnix.clr-meta.compiler-support.data.v1/symbol-name" 1)
           e06 (bind-support-call e05 "pnix.clr-meta.compiler-support.data.v1/string-equal?" 2)
           e07 (bind-support-call e06 "pnix.clr-meta.compiler-support.data.v1/env-new" 0)
           e08 (bind-support-call e07 "pnix.clr-meta.compiler-support.data.v1/env-bind" 5)
           e09 (bind-support-call e08 "pnix.clr-meta.compiler-support.data.v1/env-lookup" 2)
           e10 (bind-support-call e09 "pnix.clr-meta.compiler-support.data.v1/reject" 3)
           e11 (bind-support-call e10 "pnix.clr-meta.compiler-support.pesink.v1/begin" 5)
           e12 (bind-support-call e11 "pnix.clr-meta.compiler-support.pesink.v1/define-constant" 2)
           e13 (bind-support-call e12 "pnix.clr-meta.compiler-support.pesink.v1/define-method" 3)
           e14 (bind-support-call e13 "pnix.clr-meta.compiler-support.pesink.v1/begin-initializer" 1)
           e15 (bind-support-call e14 "pnix.clr-meta.compiler-support.pesink.v1/end-initializer" 1)
           e16 (bind-support-call e15 "pnix.clr-meta.compiler-support.pesink.v1/begin-method" 3)
           e17 (bind-support-call e16 "pnix.clr-meta.compiler-support.pesink.v1/end-method" 1)
           e18 (bind-support-call e17 "pnix.clr-meta.compiler-support.pesink.v1/allocate-local" 1)
           e19 (bind-support-call e18 "pnix.clr-meta.compiler-support.pesink.v1/new-label" 1)
           e20 (bind-support-call e19 "pnix.clr-meta.compiler-support.pesink.v1/mark-label" 2)
           e21 (bind-support-call e20 "pnix.clr-meta.compiler-support.pesink.v1/emit-literal" 3)
           e22 (bind-support-call e21 "pnix.clr-meta.compiler-support.pesink.v1/emit-load-arg" 2)
           e23 (bind-support-call e22 "pnix.clr-meta.compiler-support.pesink.v1/emit-load-local" 2)
           e24 (bind-support-call e23 "pnix.clr-meta.compiler-support.pesink.v1/emit-load-field" 2)
           e25 (bind-support-call e24 "pnix.clr-meta.compiler-support.pesink.v1/emit-store-local" 2)
           e26 (bind-support-call e25 "pnix.clr-meta.compiler-support.pesink.v1/emit-store-field" 2)
           e27 (bind-support-call e26 "pnix.clr-meta.compiler-support.pesink.v1/emit-call" 3)
           e28 (bind-support-call e27 "pnix.clr-meta.compiler-support.pesink.v1/emit-opcode" 2)
           e29 (bind-support-call e28 "pnix.clr-meta.compiler-support.pesink.v1/emit-branch-false" 2)
           e30 (bind-support-call e29 "pnix.clr-meta.compiler-support.pesink.v1/emit-branch" 2)
           e31 (bind-support-call e30 "pnix.clr-meta.compiler-support.pesink.v1/emit-pop" 1)
           e32 (bind-support-call e31 "pnix.clr-meta.compiler-support.pesink.v1/emit-ret" 1)
           e33 (bind-support-call e32 "pnix.clr-meta.compiler-support.pesink.v1/finish" 1)]
      e33)))

(def seed-intrinsics
  (fn* seed-intrinsics [env]
    (let* [e1 (pnix.clr-meta.compiler-support.data.v1/env-bind env "+" "intrinsic" "+" 2)
           e2 (pnix.clr-meta.compiler-support.data.v1/env-bind e1 "-" "intrinsic" "-" 2)
           e3 (pnix.clr-meta.compiler-support.data.v1/env-bind e2 "=" "intrinsic" "=" 2)
           e4 (pnix.clr-meta.compiler-support.data.v1/env-bind e3 "<" "intrinsic" "<" 2)]
      e4)))

(def literal?
  (fn* literal? [form]
    (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "nil")
      true
      (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "boolean")
        true
        (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "int64")
          true
          (pnix.clr-meta.compiler-support.data.v1/kind-is? form "string"))))))

(def literal-kind
  (fn* literal-kind [form]
    (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "nil")
      "nil"
      (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "boolean")
        "boolean"
        (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "int64")
          "int64"
          (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "string")
            "string"
            (pnix.clr-meta.compiler-support.data.v1/reject
              "lower"
              "not-a-literal"
              form)))))))

(def fn-initializer?
  (fn* fn-initializer? [form]
    (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "list")
      (if (count-is? form 4)
        (if (symbol-named? (pnix.clr-meta.compiler-support.data.v1/nth form 0) "fn*")
          (if (pnix.clr-meta.compiler-support.data.v1/kind-is?
                (pnix.clr-meta.compiler-support.data.v1/nth form 1)
                "symbol")
            (pnix.clr-meta.compiler-support.data.v1/kind-is?
              (pnix.clr-meta.compiler-support.data.v1/nth form 2)
              "vector")
            false)
          false)
        false)
      false)))

(def collect-definitions
  (fn* collect-definitions [forms index env]
    (if (= index (pnix.clr-meta.compiler-support.data.v1/count forms))
      env
      (let* [form (pnix.clr-meta.compiler-support.data.v1/nth forms index)]
        (if (pnix.clr-meta.compiler-support.data.v1/kind-is? form "list")
          (if (count-is? form 3)
            (let* [head (pnix.clr-meta.compiler-support.data.v1/nth form 0)
                   name-form (pnix.clr-meta.compiler-support.data.v1/nth form 1)
                   initializer (pnix.clr-meta.compiler-support.data.v1/nth form 2)]
              (if (symbol-named? head "def")
                (if (pnix.clr-meta.compiler-support.data.v1/kind-is? name-form "symbol")
                  (let* [name (pnix.clr-meta.compiler-support.data.v1/symbol-name name-form)
                         existing (pnix.clr-meta.compiler-support.data.v1/env-lookup env name)]
                    (if (pnix.clr-meta.compiler-support.data.v1/kind-is? existing "nil")
                      (if (fn-initializer? initializer)
                        (let* [fn-name (pnix.clr-meta.compiler-support.data.v1/symbol-name
                                        (pnix.clr-meta.compiler-support.data.v1/nth initializer 1))
                               params (pnix.clr-meta.compiler-support.data.v1/nth initializer 2)]
                          (if (pnix.clr-meta.compiler-support.data.v1/string-equal? name fn-name)
                            (collect-definitions
                              forms
                              (+ index 1)
                              (pnix.clr-meta.compiler-support.data.v1/env-bind
                                env
                                name
                                "kernel-call"
                                name
                                (pnix.clr-meta.compiler-support.data.v1/count params)))
                            (pnix.clr-meta.compiler-support.data.v1/reject
                              "validate"
                              "fn-name-mismatch"
                              form)))
                        (if (literal? initializer)
                          (collect-definitions
                            forms
                            (+ index 1)
                            (pnix.clr-meta.compiler-support.data.v1/env-bind
                              env
                              name
                              "constant"
                              name
                              0))
                          (pnix.clr-meta.compiler-support.data.v1/reject
                            "validate"
                            "unsupported-def-initializer"
                            form)))
                      (pnix.clr-meta.compiler-support.data.v1/reject
                        "validate"
                        "duplicate-global"
                        name-form)))
                  (pnix.clr-meta.compiler-support.data.v1/reject
                    "validate"
                    "def-name-not-symbol"
                    form))
                (pnix.clr-meta.compiler-support.data.v1/reject
                  "validate"
                  "top-level-not-def"
                  form)))
            (pnix.clr-meta.compiler-support.data.v1/reject
              "validate"
              "bad-def-arity"
              form))
          (pnix.clr-meta.compiler-support.data.v1/reject
            "validate"
            "top-level-not-list"
            form))))))

(def bind-parameters
  (fn* bind-parameters [params index env]
    (if (= index (pnix.clr-meta.compiler-support.data.v1/count params))
      env
      (let* [parameter (pnix.clr-meta.compiler-support.data.v1/nth params index)]
        (if (pnix.clr-meta.compiler-support.data.v1/kind-is? parameter "symbol")
          (bind-parameters
            params
            (+ index 1)
            (pnix.clr-meta.compiler-support.data.v1/env-bind
              env
              (pnix.clr-meta.compiler-support.data.v1/symbol-name parameter)
              "argument"
              index
              0))
          (pnix.clr-meta.compiler-support.data.v1/reject
            "validate"
            "parameter-not-symbol"
            parameter))))))

(def resolve-required
  (fn* resolve-required [env symbol phase]
    (let* [name (pnix.clr-meta.compiler-support.data.v1/symbol-name symbol)
           binding (pnix.clr-meta.compiler-support.data.v1/env-lookup env name)]
      (if (pnix.clr-meta.compiler-support.data.v1/kind-is? binding "nil")
        (pnix.clr-meta.compiler-support.data.v1/reject
          phase
          "unresolved-symbol"
          symbol)
        binding))))

(def validate-node
  (fn* validate-node [mode node index env]
    (if (pnix.clr-meta.compiler-support.data.v1/string-equal? mode "sequence")
      (if (= index (pnix.clr-meta.compiler-support.data.v1/count node))
        true
        (do
          (validate-node
            "expression"
            (pnix.clr-meta.compiler-support.data.v1/nth node index)
            0
            env)
          (validate-node "sequence" node (+ index 1) env)))
      (if (pnix.clr-meta.compiler-support.data.v1/string-equal? mode "bindings")
        (if (= index (pnix.clr-meta.compiler-support.data.v1/count node))
          env
          (let* [next (+ index 1)]
            (if (< next (pnix.clr-meta.compiler-support.data.v1/count node))
              (let* [name-form (pnix.clr-meta.compiler-support.data.v1/nth node index)
                     value-form (pnix.clr-meta.compiler-support.data.v1/nth node next)]
                (if (pnix.clr-meta.compiler-support.data.v1/kind-is? name-form "symbol")
                  (do
                    (validate-node "expression" value-form 0 env)
                    (validate-node
                      "bindings"
                      node
                      (+ index 2)
                      (pnix.clr-meta.compiler-support.data.v1/env-bind
                        env
                        (pnix.clr-meta.compiler-support.data.v1/symbol-name name-form)
                        "local"
                        -1
                        0)))
                  (pnix.clr-meta.compiler-support.data.v1/reject
                    "validate"
                    "binding-name-not-symbol"
                    name-form)))
              (pnix.clr-meta.compiler-support.data.v1/reject
                "validate"
                "odd-binding-vector"
                node))))
        (if (pnix.clr-meta.compiler-support.data.v1/string-equal? mode "expression")
          (if (literal? node)
            true
            (if (pnix.clr-meta.compiler-support.data.v1/kind-is? node "symbol")
              (let* [binding (resolve-required env node "validate")]
                (if (callable-binding? binding)
                  (pnix.clr-meta.compiler-support.data.v1/reject
                    "validate"
                    "callable-used-as-value"
                    node)
                  true))
              (if (pnix.clr-meta.compiler-support.data.v1/kind-is? node "list")
                (if (count-at-least? node 1)
                  (let* [head (pnix.clr-meta.compiler-support.data.v1/nth node 0)]
                    (if (symbol-named? head "if")
                      (if (count-is? node 4)
                        (do
                          (validate-node "expression" (pnix.clr-meta.compiler-support.data.v1/nth node 1) 0 env)
                          (validate-node "expression" (pnix.clr-meta.compiler-support.data.v1/nth node 2) 0 env)
                          (validate-node "expression" (pnix.clr-meta.compiler-support.data.v1/nth node 3) 0 env)
                          true)
                        (pnix.clr-meta.compiler-support.data.v1/reject
                          "validate"
                          "bad-if-arity"
                          node))
                      (if (symbol-named? head "let*")
                        (if (count-is? node 3)
                          (let* [bindings (pnix.clr-meta.compiler-support.data.v1/nth node 1)]
                            (if (pnix.clr-meta.compiler-support.data.v1/kind-is? bindings "vector")
                              (let* [body-env (validate-node "bindings" bindings 0 env)]
                                (validate-node
                                  "expression"
                                  (pnix.clr-meta.compiler-support.data.v1/nth node 2)
                                  0
                                  body-env))
                              (pnix.clr-meta.compiler-support.data.v1/reject
                                "validate"
                                "let-bindings-not-vector"
                                node)))
                          (pnix.clr-meta.compiler-support.data.v1/reject
                            "validate"
                            "bad-let-arity"
                            node))
                        (if (symbol-named? head "do")
                          (if (count-at-least? node 2)
                            (validate-node "sequence" node 1 env)
                            (pnix.clr-meta.compiler-support.data.v1/reject
                              "validate"
                              "empty-do"
                              node))
                          (if (symbol-named? head "fn*")
                            (pnix.clr-meta.compiler-support.data.v1/reject
                              "validate"
                              "nested-fn"
                              node)
                            (if (pnix.clr-meta.compiler-support.data.v1/kind-is? head "symbol")
                              (let* [binding (resolve-required env head "validate")
                                     arity (- (pnix.clr-meta.compiler-support.data.v1/count node) 1)]
                                (if (callable-binding? binding)
                                  (if (binding-arity-is? binding arity)
                                    (validate-node "sequence" node 1 env)
                                    (pnix.clr-meta.compiler-support.data.v1/reject
                                      "validate"
                                      "call-arity"
                                      node))
                                  (pnix.clr-meta.compiler-support.data.v1/reject
                                    "validate"
                                    "non-callable-head"
                                    head)))
                              (pnix.clr-meta.compiler-support.data.v1/reject
                                "validate"
                                "call-head-not-symbol"
                                head)))))))
                  (pnix.clr-meta.compiler-support.data.v1/reject
                    "validate"
                    "empty-list"
                    node))
                (pnix.clr-meta.compiler-support.data.v1/reject
                  "validate"
                  "unsupported-expression"
                  node))))
          (pnix.clr-meta.compiler-support.data.v1/reject
            "validate"
            "unknown-validation-mode"
            mode))))))

(def validate-definition
  (fn* validate-definition [form globals]
    (let* [initializer (pnix.clr-meta.compiler-support.data.v1/nth form 2)]
      (if (fn-initializer? initializer)
        (let* [params (pnix.clr-meta.compiler-support.data.v1/nth initializer 2)
               body (pnix.clr-meta.compiler-support.data.v1/nth initializer 3)
               env (bind-parameters params 0 globals)]
          (validate-node "expression" body 0 env))
        (if (literal? initializer)
          true
          (pnix.clr-meta.compiler-support.data.v1/reject
            "validate"
            "unsupported-def-initializer"
            form))))))

(def validate-definitions
  (fn* validate-definitions [forms index globals]
    (if (= index (pnix.clr-meta.compiler-support.data.v1/count forms))
      true
      (do
        (validate-definition
          (pnix.clr-meta.compiler-support.data.v1/nth forms index)
          globals)
        (validate-definitions forms (+ index 1) globals)))))

(def validate-program
  (fn* validate-program [forms]
    (if (count-at-least? forms 2)
      (let* [envelope (pnix.clr-meta.compiler-support.data.v1/nth forms 0)]
        (if (pnix.clr-meta.compiler-support.data.v1/kind-is? envelope "list")
          (if (count-is? envelope 2)
            (let* [head (pnix.clr-meta.compiler-support.data.v1/nth envelope 0)
                   name-form (pnix.clr-meta.compiler-support.data.v1/nth envelope 1)]
              (if (symbol-named? head "ns")
                (if (symbol-named? name-form kernel-namespace)
                  (let* [empty-env (pnix.clr-meta.compiler-support.data.v1/env-new)
                         support-env (seed-support-calls empty-env)
                         intrinsic-env (seed-intrinsics support-env)
                         globals (collect-definitions forms 1 intrinsic-env)
                         entry (pnix.clr-meta.compiler-support.data.v1/env-lookup globals kernel-entry)]
                    (if (pnix.clr-meta.compiler-support.data.v1/kind-is? entry "nil")
                      (pnix.clr-meta.compiler-support.data.v1/reject
                        "validate"
                        "missing-entry"
                        name-form)
                      (if (binding-kind? entry "kernel-call")
                        (if (binding-arity-is? entry 2)
                          (do
                            (validate-definitions forms 1 globals)
                            globals)
                          (pnix.clr-meta.compiler-support.data.v1/reject
                            "validate"
                            "entry-arity"
                            name-form))
                        (pnix.clr-meta.compiler-support.data.v1/reject
                          "validate"
                          "entry-not-callable"
                          name-form))))
                  (pnix.clr-meta.compiler-support.data.v1/reject
                    "validate"
                    "namespace-mismatch"
                    name-form))
                (pnix.clr-meta.compiler-support.data.v1/reject
                  "validate"
                  "missing-ns-envelope"
                  envelope)))
            (pnix.clr-meta.compiler-support.data.v1/reject
              "validate"
              "bad-ns-envelope"
              envelope))
          (pnix.clr-meta.compiler-support.data.v1/reject
            "validate"
            "bad-ns-envelope"
            envelope)))
      (pnix.clr-meta.compiler-support.data.v1/reject
        "validate"
        "program-too-short"
        forms))))

(def select-intrinsic-opcode
  (fn* select-intrinsic-opcode [name form]
    (if (pnix.clr-meta.compiler-support.data.v1/string-equal? name "+")
      add-opcode
      (if (pnix.clr-meta.compiler-support.data.v1/string-equal? name "-")
        subtract-opcode
        (if (pnix.clr-meta.compiler-support.data.v1/string-equal? name "=")
          equality-opcode
          (if (pnix.clr-meta.compiler-support.data.v1/string-equal? name "<")
            less-than-opcode
            (pnix.clr-meta.compiler-support.data.v1/reject
              "lower"
              "unknown-intrinsic"
              form)))))))

(def lower-node
  (fn* lower-node [mode node index env sink]
    (if (pnix.clr-meta.compiler-support.data.v1/string-equal? mode "arguments")
      (if (= index (pnix.clr-meta.compiler-support.data.v1/count node))
        nil
        (do
          (lower-node
            "expression"
            (pnix.clr-meta.compiler-support.data.v1/nth node index)
            0
            env
            sink)
          (lower-node "arguments" node (+ index 1) env sink)))
      (if (pnix.clr-meta.compiler-support.data.v1/string-equal? mode "do-sequence")
        (let* [next (+ index 1)]
          (do
            (lower-node
              "expression"
              (pnix.clr-meta.compiler-support.data.v1/nth node index)
              0
              env
              sink)
            (if (= next (pnix.clr-meta.compiler-support.data.v1/count node))
              nil
              (do
                (pnix.clr-meta.compiler-support.pesink.v1/emit-pop sink)
                (lower-node "do-sequence" node next env sink)))))
        (if (pnix.clr-meta.compiler-support.data.v1/string-equal? mode "bindings")
          (if (= index (pnix.clr-meta.compiler-support.data.v1/count node))
            env
            (let* [next (+ index 1)
                   name-form (pnix.clr-meta.compiler-support.data.v1/nth node index)
                   value-form (pnix.clr-meta.compiler-support.data.v1/nth node next)
                   ignored (lower-node "expression" value-form 0 env sink)
                   local (pnix.clr-meta.compiler-support.pesink.v1/allocate-local sink)
                   stored (pnix.clr-meta.compiler-support.pesink.v1/emit-store-local sink local)
                   next-env (pnix.clr-meta.compiler-support.data.v1/env-bind
                              env
                              (pnix.clr-meta.compiler-support.data.v1/symbol-name name-form)
                              "local"
                              local
                              0)]
              (lower-node "bindings" node (+ index 2) next-env sink)))
          (if (pnix.clr-meta.compiler-support.data.v1/string-equal? mode "expression")
            (if (literal? node)
              (pnix.clr-meta.compiler-support.pesink.v1/emit-literal
                sink
                (literal-kind node)
                node)
              (if (pnix.clr-meta.compiler-support.data.v1/kind-is? node "symbol")
                (let* [binding (resolve-required env node "lower")]
                  (if (binding-kind? binding "argument")
                    (pnix.clr-meta.compiler-support.pesink.v1/emit-load-arg
                      sink
                      (pnix.clr-meta.compiler-support.data.v1/nth binding 1))
                    (if (binding-kind? binding "local")
                      (pnix.clr-meta.compiler-support.pesink.v1/emit-load-local
                        sink
                        (pnix.clr-meta.compiler-support.data.v1/nth binding 1))
                      (if (binding-kind? binding "constant")
                        (pnix.clr-meta.compiler-support.pesink.v1/emit-load-field
                          sink
                          (pnix.clr-meta.compiler-support.data.v1/nth binding 1))
                        (pnix.clr-meta.compiler-support.data.v1/reject
                          "lower"
                          "callable-used-as-value"
                          node)))))
                (let* [head (pnix.clr-meta.compiler-support.data.v1/nth node 0)]
                  (if (symbol-named? head "if")
                    (let* [else-label (pnix.clr-meta.compiler-support.pesink.v1/new-label sink)
                           end-label (pnix.clr-meta.compiler-support.pesink.v1/new-label sink)]
                      (do
                        (lower-node "expression" (pnix.clr-meta.compiler-support.data.v1/nth node 1) 0 env sink)
                        (pnix.clr-meta.compiler-support.pesink.v1/emit-branch-false sink else-label)
                        (lower-node "expression" (pnix.clr-meta.compiler-support.data.v1/nth node 2) 0 env sink)
                        (pnix.clr-meta.compiler-support.pesink.v1/emit-branch sink end-label)
                        (pnix.clr-meta.compiler-support.pesink.v1/mark-label sink else-label)
                        (lower-node "expression" (pnix.clr-meta.compiler-support.data.v1/nth node 3) 0 env sink)
                        (pnix.clr-meta.compiler-support.pesink.v1/mark-label sink end-label)))
                    (if (symbol-named? head "let*")
                      (let* [bindings (pnix.clr-meta.compiler-support.data.v1/nth node 1)
                             body-env (lower-node "bindings" bindings 0 env sink)]
                        (lower-node
                          "expression"
                          (pnix.clr-meta.compiler-support.data.v1/nth node 2)
                          0
                          body-env
                          sink))
                      (if (symbol-named? head "do")
                        (lower-node "do-sequence" node 1 env sink)
                        (let* [binding (resolve-required env head "lower")
                               arity (- (pnix.clr-meta.compiler-support.data.v1/count node) 1)
                               lowered (lower-node "arguments" node 1 env sink)
                               target (pnix.clr-meta.compiler-support.data.v1/nth binding 1)]
                          (if (binding-kind? binding "intrinsic")
                            (pnix.clr-meta.compiler-support.pesink.v1/emit-opcode
                              sink
                              (select-intrinsic-opcode target node))
                            (pnix.clr-meta.compiler-support.pesink.v1/emit-call
                              sink
                              target
                              arity)))))))))
            (pnix.clr-meta.compiler-support.data.v1/reject
              "lower"
              "unknown-lowering-mode"
              mode)))))))

(def declare-definitions
  (fn* declare-definitions [forms index sink]
    (if (= index (pnix.clr-meta.compiler-support.data.v1/count forms))
      nil
      (let* [form (pnix.clr-meta.compiler-support.data.v1/nth forms index)
             name-form (pnix.clr-meta.compiler-support.data.v1/nth form 1)
             name (pnix.clr-meta.compiler-support.data.v1/symbol-name name-form)
             initializer (pnix.clr-meta.compiler-support.data.v1/nth form 2)]
        (do
          (if (fn-initializer? initializer)
            (pnix.clr-meta.compiler-support.pesink.v1/define-method
              sink
              name
              (pnix.clr-meta.compiler-support.data.v1/count
                (pnix.clr-meta.compiler-support.data.v1/nth initializer 2)))
            (pnix.clr-meta.compiler-support.pesink.v1/define-constant sink name))
          (declare-definitions forms (+ index 1) sink))))))

(def initialize-constants
  (fn* initialize-constants [forms index globals sink]
    (if (= index (pnix.clr-meta.compiler-support.data.v1/count forms))
      nil
      (let* [form (pnix.clr-meta.compiler-support.data.v1/nth forms index)
             name-form (pnix.clr-meta.compiler-support.data.v1/nth form 1)
             initializer (pnix.clr-meta.compiler-support.data.v1/nth form 2)]
        (do
          (if (fn-initializer? initializer)
            nil
            (do
              (lower-node "expression" initializer 0 globals sink)
              (pnix.clr-meta.compiler-support.pesink.v1/emit-store-field
                sink
                (pnix.clr-meta.compiler-support.data.v1/symbol-name name-form))))
          (initialize-constants forms (+ index 1) globals sink))))))

(def lower-definition
  (fn* lower-definition [form globals sink]
    (let* [name-form (pnix.clr-meta.compiler-support.data.v1/nth form 1)
           name (pnix.clr-meta.compiler-support.data.v1/symbol-name name-form)
           initializer (pnix.clr-meta.compiler-support.data.v1/nth form 2)]
      (if (fn-initializer? initializer)
        (let* [params (pnix.clr-meta.compiler-support.data.v1/nth initializer 2)
               body (pnix.clr-meta.compiler-support.data.v1/nth initializer 3)
               arity (pnix.clr-meta.compiler-support.data.v1/count params)
               env (bind-parameters params 0 globals)]
          (do
            (pnix.clr-meta.compiler-support.pesink.v1/begin-method sink name arity)
            (lower-node "expression" body 0 env sink)
            (pnix.clr-meta.compiler-support.pesink.v1/emit-ret sink)
            (pnix.clr-meta.compiler-support.pesink.v1/end-method sink)))
        nil))))

(def lower-definitions
  (fn* lower-definitions [forms index globals sink]
    (if (= index (pnix.clr-meta.compiler-support.data.v1/count forms))
      nil
      (do
        (lower-definition
          (pnix.clr-meta.compiler-support.data.v1/nth forms index)
          globals
          sink)
        (lower-definitions forms (+ index 1) globals sink)))))

(def lower-program
  (fn* lower-program [forms globals sink]
    (do
      (pnix.clr-meta.compiler-support.pesink.v1/begin
        sink
        kernel-identity
        kernel-namespace
        kernel-entry
        source-profile-id)
      (declare-definitions forms 1 sink)
      (pnix.clr-meta.compiler-support.pesink.v1/begin-initializer sink)
      (initialize-constants forms 1 globals sink)
      (pnix.clr-meta.compiler-support.pesink.v1/end-initializer sink)
      (lower-definitions forms 1 globals sink)
      (pnix.clr-meta.compiler-support.pesink.v1/finish sink))))

(def compile-source
  (fn* compile-source [source sink]
    (let* [forms (pnix.clr-meta.compiler-support.reader.v1/read-all
                   source
                   source-limits-id)
           globals (validate-program forms)]
      (lower-program forms globals sink))))
