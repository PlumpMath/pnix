(ns pnix-clr.parser
  (:require [pnix-clr.lexer :as lexer]
            [pnix-clr.outcome :as outcome]))

(declare parse-expression parse-unary parse-additive parse-postfix
         parse-application parse-application-term)

(defn- parser-state
  [tokens]
  {:tokens tokens :index (atom 0)})

(defn- token-at
  ([state] (token-at state 0))
  ([state lookahead]
   (get (:tokens state) (+ @(:index state) lookahead)
        {:kind :eof :text "" :offset -1})))

(defn- take-token!
  [state]
  (let [token (token-at state)]
    (swap! (:index state) inc)
    token))

(defn- accept!
  [state kind]
  (when (= kind (:kind (token-at state)))
    (take-token! state)))

(defn- expect!
  [state kind]
  (let [token (token-at state)]
    (if (= kind (:kind token))
      (take-token! state)
      (outcome/fail! :parse :syntax-error
                     {:reason "unexpected-token"
                      :expected (name kind)
                      :actual (name (:kind token))
                      :offset (:offset token)}))))

(defn- binary-node
  [operator left right]
  {:op :binary :operator operator :left left :right right})

(defn- literal-attrset?
  [ast]
  (and (map? ast) (= :attrset (:op ast))))

(defn- nest-attr-path
  "Desugar `a.b.c = v` into nested attrset literals rooted at `a`.
  Dynamic key segments are only supported as a single-segment path."
  [segments value]
  (if (= 1 (count segments))
    [(first segments) value]
    (do
      (when (some #(and (map? %) (:pnix/dynamic-attr %)) segments)
        (outcome/fail! :parse :syntax-error
                       {:reason "dynamic-attr-in-nested-path"}))
      (let [[head & tail] segments
            [child-key child-value] (nest-attr-path tail value)]
        [head {:op :attrset
               :recursive? false
               :entries {child-key child-value}}]))))

(defn- merge-attr-field
  "Merge `(name, value)` into entries. Attrset literals merge recursively
  (Nix addAttr / pnix-rs merge_attr_field). Leaf duplicates fail."
  [entries name value offset]
  (if-not (contains? entries name)
    (assoc entries name value)
    (let [existing (get entries name)]
      (if (and (literal-attrset? existing) (literal-attrset? value))
        (assoc entries name
               {:op :attrset
                :recursive? (boolean (or (:recursive? existing)
                                         (:recursive? value)))
                :entries (reduce (fn [acc [k v]]
                                   (merge-attr-field acc k v offset))
                                 (:entries existing)
                                 (:entries value))})
        (outcome/fail! :parse :syntax-error
                       {:reason "duplicate-attribute"
                        :attribute name
                        :offset offset})))))

(defn- parse-string-interp-ast
  "Build a :string-interp AST from a string-interp token (shared with primary)."
  [token]
  {:op :string-interp
   :parts (mapv (fn [part]
                  (case (:kind part)
                    :lit {:op :string :value (:value part)}
                    :interp
                    (let [sub (parser-state
                               (conj (vec (:tokens part))
                                     {:kind :eof :text "" :offset -1}))
                          expr (parse-expression sub)]
                      (expect! sub :eof)
                      expr)))
                (:parts token))})

(defn- dynamic-attr-key?
  [name]
  (and (map? name) (true? (:pnix/dynamic-attr name))))

(defn- parse-attr-name
  "Attr segment: bare ident, keyword-as-ident (true/false/null), quoted
  string (incl. empty), or interpolated string (dynamic key)."
  [state]
  (let [tok (token-at state)]
    (cond
      (= :ident (:kind tok))
      (:text (take-token! state))

      (= :string (:kind tok))
      (:value (take-token! state))

      ;; Dynamic attribute: `"${expr}"` or multi-part interpolated key.
      (= :string-interp (:kind tok))
      (let [t (take-token! state)]
        {:pnix/dynamic-attr true
         :ast (parse-string-interp-ast t)})

      ;; Dynamic attribute, bare form: `attrs.${expr}` (no surrounding quotes).
      (= :dollar-lbrace (:kind tok))
      (do (take-token! state)
          (let [expr (parse-expression state)]
            (expect! state :rbrace)
            {:pnix/dynamic-attr true :ast expr}))

      ;; `builtins.true` / `{ null = 1; }.null` — keyword tokens as attr names.
      (contains? #{:true :false :null} (:kind tok))
      (do (take-token! state)
          (name (:kind tok)))

      :else
      (outcome/fail! :parse :syntax-error
                     {:reason "unexpected-token"
                      :expected "ident"
                      :actual (name (:kind tok))
                      :offset (:offset tok)}))))

(defn- parse-attr-binding-path
  [state]
  (loop [path [(parse-attr-name state)]]
    (if (accept! state :dot)
      (recur (conj path (parse-attr-name state)))
      path)))

(defn- parse-inherit-names
  "Parse `name ... ;` after `inherit` or `inherit (expr)`. Returns
  vector of attribute names and advances past the semicolon."
  [state]
  (loop [names []]
    (if (accept! state :semicolon)
      (if (seq names)
        names
        (outcome/fail! :parse :syntax-error
                       {:reason "empty-inherit"
                        :offset (:offset (token-at state))}))
      (let [name-token (expect! state :ident)
            attr-name (:text name-token)]
        (when (some #(= attr-name %) names)
          (outcome/fail! :parse :syntax-error
                         {:reason "duplicate-attribute"
                          :attribute attr-name
                          :offset (:offset name-token)}))
        (recur (conj names attr-name))))))

(defn- parse-inherit
  "Nix inherit: `inherit a b;` → enclosing vars; `inherit (e) a b;` → selects.
  Returns a map of name → value-ast (enclosing-var or select)."
  [state]
  (expect! state :inherit)
  (let [source (when (accept! state :lparen)
                 (let [expr (parse-expression state)]
                   (expect! state :rparen)
                   expr))
        names (parse-inherit-names state)]
    (into {}
          (map (fn [attr-name]
                 [attr-name
                  (if source
                    {:op :select :target source :attribute attr-name}
                    {:op :enclosing-var :name attr-name})]))
          names)))

(defn- parse-attrset
  [state recursive?]
  (expect! state :lbrace)
  (loop [entries {}]
    (if (accept! state :rbrace)
      {:op :attrset :recursive? recursive? :entries entries}
      (if (= :inherit (:kind (token-at state)))
        (let [inherited (parse-inherit state)
              offset (:offset (token-at state))]
          (recur (reduce (fn [acc [attr-name value]]
                           (merge-attr-field acc attr-name value offset))
                         entries
                         inherited)))
        (let [name-token (token-at state)
              path (parse-attr-binding-path state)
              _ (expect! state :assign)
              value (parse-expression state)
              _ (expect! state :semicolon)
              [attr-name nested-value] (nest-attr-path path value)]
          (recur (merge-attr-field entries attr-name nested-value
                                   (:offset name-token))))))))

(defn- parse-list-item
  [state]
  ;; Nix list elements: bare items stay at select/postfix so
  ;; `[true false]` is two values. Parentheses are a primary inside
  ;; postfix, so `[(f x).a]` keeps trailing select after the group.
  ;;
  ;; A leading unary `-` / `!` is NOT a list element. Nix parses elements at
  ;; expr_select, so `[ -1 ]` and `[ !true ]` are syntax errors there and the
  ;; parenthesized `[ (-1) ]` is the accepting form (oracle: nix-instantiate
  ;; --eval -E '[ -1 ]' => "syntax error, unexpected '-'").
  (parse-postfix state))

(defn- parse-list
  [state]
  (expect! state :lbracket)
  (loop [items []]
    (if (accept! state :rbracket)
      {:op :list :items items}
      (recur (conj items (parse-list-item state))))))

(defn- parse-primary
  [state]
  (let [token (token-at state)]
    (case (:kind token)
      :true (do (take-token! state) {:op :bool :value true})
      :false (do (take-token! state) {:op :bool :value false})
      :null (do (take-token! state) {:op :null})
      :int (do (take-token! state)
               (when (and (= (:value token) System.Int64/MinValue)
                          (= (:text token) lexer/min-i64-magnitude-text))
                 ;; Reached without parse-unary's negate-fold consuming it
                 ;; first, so this is a bare (unsigned) occurrence -- the
                 ;; magnitude never fits a positive Int64.
                 (outcome/fail! :parse :syntax-error
                                {:reason "integer-literal-out-of-range"
                                 :literal (:text token)}))
               {:op :int :value (:value token)})
      :float (do (take-token! state) {:op :float :value (:value token)})
      :string (do (take-token! state) {:op :string :value (:value token)})
      ;; Deprecated Nix URI literals evaluate as plain strings.
      :uri (do (take-token! state) {:op :string :value (:text token)})
      :string-interp
      (do
        (take-token! state)
        (parse-string-interp-ast token))
      :ident (do (take-token! state) {:op :var :name (:text token)})
      :path (do (take-token! state) {:op :path :value (:text token)})
      :lparen (do
                (take-token! state)
                (let [expression (parse-expression state)]
                  (expect! state :rparen)
                  expression))
      :lbracket (parse-list state)
      :rec (do (take-token! state) (parse-attrset state true))
      :lbrace (parse-attrset state false)
      :unsupported-keyword
      (outcome/fail! :parse :syntax-error
                     {:reason "unsupported-construct"
                      :construct (:text token) :offset (:offset token)})
      (outcome/fail! :parse :syntax-error
                     {:reason "unexpected-token"
                      :actual (name (:kind token))
                      :offset (:offset token)}))))

(defn- parse-select-default
  "Nix select-default is a tight primary/postfix (not full expr): no bare
  application or infix outside parentheses."
  [state]
  (parse-postfix state))

(defn- parse-postfix
  [state]
  (loop [target (parse-primary state)]
    (if (accept! state :dot)
      (let [attribute (parse-attr-name state)
            selected {:op :select :target target :attribute attribute}
            ;; `a.b or default` — fallback when the attribute is missing.
            next-tok (token-at state)]
        (if (and (= :ident (:kind next-tok))
                 (= "or" (:text next-tok)))
          (do
            (take-token! state)
            (let [default (parse-select-default state)]
              (recur (assoc selected :or-default default))))
          (recur selected)))
      target)))

(defn parse-unary
  [state]
  (cond
    (accept! state :not)
    ;; Nix `!` binds more loosely than application/selection/`?`/arithmetic.
    ;; Its operand therefore absorbs through the additive level.
    {:op :not :value (parse-additive state)}

    (accept! state :minus)
    (if (and (= :int (:kind (token-at state)))
             (= lexer/min-i64-magnitude-text (:text (token-at state))))
      ;; `-9223372036854775808` is Int64.MinValue -- its unsigned magnitude
      ;; alone never fits a positive Int64, so fold sign+magnitude into one
      ;; literal here rather than negating an already-rejected token.
      (do (take-token! state) {:op :int :value System.Int64/MinValue})
      {:op :negate :value (parse-unary state)})

    :else
    (parse-application state)))

(defn- application-start?
  [kind]
  (contains? #{:true :false :null :int :float :string :ident :path :uri
               :lparen :lbracket :lbrace :rec}
             kind))

(defn- parse-application
  [state]
  (loop [function (parse-application-term state)]
    (if (application-start? (:kind (token-at state)))
      (recur {:op :call :function function :argument (parse-postfix state)})
      function)))

(defn- parse-application-term
  [state]
  ;; `import` consumes one term.  Any following atom remains an argument to
  ;; the imported function: `import ./module.px 1` is `(import ./module.px) 1`.
  ;; `scopedImport` is the same shape with an extra leading scope term:
  ;; `scopedImport { x = 1; } ./module.px`.
  (cond
    (accept! state :import)
    {:op :import :target (parse-application-term state)}

    (accept! state :scoped-import)
    {:op :scoped-import
     :scope (parse-application-term state)
     :target (parse-application-term state)}

    :else
    (parse-postfix state)))

(defn- parse-static-attr-path
  [state]
  (loop [path [(parse-attr-name state)]]
    (if (accept! state :dot)
      (recur (conj path (parse-attr-name state)))
      path)))

(defn- parse-has-attr
  [state]
  (loop [target (parse-unary state)]
    (if (accept! state :question)
      (recur {:op :has-attr
              :target target
              :path (parse-static-attr-path state)})
      target)))

(defn- parse-multiplicative
  [state]
  (loop [left (parse-has-attr state)]
    (let [kind (:kind (token-at state))]
      (if (contains? #{:multiply :divide} kind)
        (do
          (take-token! state)
          (recur (binary-node kind left (parse-has-attr state))))
        left))))

(defn- parse-additive
  [state]
  (loop [left (parse-multiplicative state)]
    (let [kind (:kind (token-at state))]
      (if (contains? #{:plus :minus} kind)
        (do
          (take-token! state)
          (recur (binary-node kind left (parse-multiplicative state))))
        left))))

(defn- parse-comparison
  [state]
  (let [left (parse-additive state)
        kind (:kind (token-at state))]
    (if-not (contains? #{:lt :gt :le :ge} kind)
      left
      (do
        (take-token! state)
        (binary-node kind left (parse-additive state))))))

(defn- parse-equality
  [state]
  (let [left (parse-comparison state)
        kind (:kind (token-at state))]
    (if-not (contains? #{:eq :neq} kind)
      left
      (do
        (take-token! state)
        (let [right (parse-comparison state)
              next-kind (:kind (token-at state))]
          (when (contains? #{:eq :neq} next-kind)
            (outcome/fail! :parse :syntax-error
                           {:reason "non-associative-equality"
                            :operator (:text (token-at state))
                            :offset (:offset (token-at state))}))
          (binary-node kind left right))))))

(defn- parse-update-concat
  "Nix `//` (attrset update) and `++` (list/string concat). Same precedence,
  left-associative here (sufficient for portable overlay and common corpus)."
  [state]
  (loop [left (parse-equality state)]
    (let [kind (:kind (token-at state))]
      (if (contains? #{:update :concat} kind)
        (do
          (take-token! state)
          (recur (binary-node kind left (parse-equality state))))
        left))))

(defn- parse-and
  [state]
  (loop [left (parse-update-concat state)]
    (if (accept! state :and)
      (recur (binary-node :and left (parse-update-concat state)))
      left)))

(defn- parse-or
  [state]
  (loop [left (parse-and state)]
    (if (accept! state :or)
      (recur (binary-node :or left (parse-and state)))
      left)))

(defn- matching-brace-lookahead
  "Index offset (relative to current token) of the `}` that closes the
  brace group starting at the current token. Nil if unbalanced."
  [state]
  (loop [i 0
         depth 0]
    (let [tok (token-at state i)]
      (cond
        (= :eof (:kind tok)) nil
        (= :lbrace (:kind tok)) (recur (inc i) (inc depth))
        (= :rbrace (:kind tok))
        (if (= 1 depth)
          i
          (recur (inc i) (dec depth)))
        :else (recur (inc i) depth)))))

(defn- paramset-lambda-start?
  "True when `{ ... }:` or `{ ... }@name:` starts a pattern lambda."
  [state]
  (when (= :lbrace (:kind (token-at state)))
    (when-let [close-i (matching-brace-lookahead state)]
      (let [after (token-at state (inc close-i))]
        (or (= :colon (:kind after))
            (and (= :at (:kind after))
                 (= :ident (:kind (token-at state (+ close-i 2))))
                 (= :colon (:kind (token-at state (+ close-i 3))))))))))

(defn- parse-paramset-lambda
  "Parse `{ formals }[:|@as:] body` with optional leading `as@` already
  consumed (leading-as is the capture name, or nil)."
  ([state]
   (parse-paramset-lambda state nil))
  ([state leading-as]
   (expect! state :lbrace)
   (loop [params []
          ellipsis? false]
     (let [tok (token-at state)]
       (cond
         (accept! state :rbrace)
         (let [as-name
               (cond
                 leading-as leading-as
                 (accept! state :at)
                 (:text (expect! state :ident))
                 :else nil)]
           (expect! state :colon)
           {:op :lambda
            :param-pattern {:kind :attr-pattern
                            :params params
                            :ellipsis? ellipsis?
                            :as as-name}
            :body (parse-expression state)})

         (accept! state :comma)
         (recur params ellipsis?)

         (accept! state :ellipsis)
         (recur params true)

         (not= :ident (:kind tok))
         (outcome/fail! :parse :syntax-error
                        {:reason "expected-parameter-name"
                         :actual (name (:kind tok))
                         :offset (:offset tok)})

         :else
         (let [name (:text (take-token! state))]
           (if (accept! state :question)
             (let [default (parse-expression state)]
               (recur (conj params {:name name :default default})
                      ellipsis?))
             (recur (conj params {:name name})
                    ellipsis?))))))))

(defn- parse-lambda
  [state]
  (cond
    ;; `x: body`
    (and (= :ident (:kind (token-at state)))
         (= :colon (:kind (token-at state 1))))
    (let [parameter (:text (take-token! state))]
      (take-token! state)
      {:op :lambda :parameter parameter :body (parse-expression state)})

    ;; `name@{ formals }: body`
    (and (= :ident (:kind (token-at state)))
         (= :at (:kind (token-at state 1)))
         (= :lbrace (:kind (token-at state 2))))
    (let [as-name (:text (take-token! state))]
      (take-token! state) ; @
      (parse-paramset-lambda state as-name))

    ;; `{ formals }: body` / `{ formals }@name: body`
    (paramset-lambda-start? state)
    (parse-paramset-lambda state)

    :else
    (parse-or state)))

(defn- parse-if
  [state]
  (if (accept! state :if)
    (let [condition (parse-expression state)]
      (expect! state :then)
      (let [then-branch (parse-expression state)]
        (expect! state :else)
        {:op :if
         :condition condition
         :then then-branch
         :else (parse-expression state)}))
    (parse-lambda state)))

(defn- parse-let
  [state]
  (if (accept! state :let)
    (loop [bindings {}]
      (if (accept! state :in)
        {:op :let :bindings bindings :body (parse-expression state)}
        (if (= :inherit (:kind (token-at state)))
          (let [inherited (parse-inherit state)]
            (recur
             (reduce (fn [acc [binding-name value]]
                       (when (contains? acc binding-name)
                         (outcome/fail! :parse :syntax-error
                                        {:reason "duplicate-binding"
                                         :binding binding-name}))
                       (assoc acc binding-name value))
                     bindings
                     inherited)))
          (let [name-token (token-at state)
                path (parse-attr-binding-path state)]
            (expect! state :assign)
            (let [value (parse-expression state)]
              (expect! state :semicolon)
              (let [[binding-name nested-value] (nest-attr-path path value)]
                (recur (merge-attr-field bindings binding-name nested-value
                                         (:offset name-token)))))))))
    (parse-if state)))

(defn- parse-assert
  [state]
  (if (accept! state :assert)
    (let [condition (parse-expression state)]
      (expect! state :semicolon)
      {:op :assert
       :condition condition
       :body (parse-expression state)})
    (parse-let state)))

(defn- parse-with
  [state]
  (if (accept! state :with)
    (let [attrs (parse-expression state)]
      (expect! state :semicolon)
      {:op :with
       :attrs attrs
       :body (parse-expression state)})
    (parse-assert state)))

(defn parse-expression
  [state]
  (parse-with state))

(defn parse-source
  [source]
  (let [state (parser-state (lexer/tokenize source))
        ast (parse-expression state)]
    (expect! state :eof)
    ast))
