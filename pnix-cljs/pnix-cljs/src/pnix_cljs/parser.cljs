(ns pnix-cljs.parser
  (:require [pnix-cljs.tokenizer :as tokenizer]))

(declare parse-expression)
(declare parse-primary)
(declare parse-selection)
(declare parse-has-attr)
(declare parse)

(def attribute-name-kinds
  #{:identifier :string :interpolated-string
    :let :in :if :then :else :true :false :null
    :rec :inherit :import})

(defn attribute-name-token? [token]
  (contains? attribute-name-kinds (:kind token)))

(defn parser-state [tokens]
  (atom {:tokens tokens :index 0}))

(defn current-token [parser]
  (let [{:keys [tokens index]} @parser]
    (nth tokens index)))

(defn next-token [parser]
  (let [{:keys [tokens index]} @parser]
    (nth tokens (min (inc index) (dec (count tokens))))))

(defn advance! [parser]
  (let [token (current-token parser)]
    (swap! parser update :index inc)
    token))

(defn match! [parser kind]
  (when (= kind (:kind (current-token parser)))
    (advance! parser)))

(defn parse-failure! [parser detail-class evidence]
  (let [token (current-token parser)]
    (throw (ex-info detail-class
                    {"pnix_error" true
                     "phase" "parse"
                     "class" (if (= detail-class "duplicate-attrset-binding")
                               detail-class
                               "syntax-error")
                     "evidence" (assoc evidence
                                       "detail_class" detail-class
                                       "offset" (:offset token)
                                       "token" (name (:kind token)))}))))

(defn expect! [parser kind]
  (or (match! parser kind)
      (parse-failure! parser
                      "expected-token"
                      {"expected" (name kind)})))

(defn string-segments [token]
  (mapv (fn [part]
          (if (= :text (:kind part))
            part
            {:kind :expression
             :value (parse (:source part))}))
        (:parts token)))

(defn attribute-key [token]
  (if (= :interpolated-string (:kind token))
    {:name-expression {:op :interpolated-string
                       :segments (string-segments token)}}
    {:name (:value token)}))

(defn parse-dynamic-attribute [parser]
  (expect! parser :dynamic-start)
  (let [expression (parse-expression parser)]
    (expect! parser :right-brace)
    {:name-expression expression}))

(defn nest-attr-path
  "Desugar a.b.c = v into nested attrset fields rooted at a."
  [segments value]
  (if (= 1 (count segments))
    {:name (first segments) :value value}
    (let [head (first segments)
          nested (nest-attr-path (rest segments) value)]
      {:name head
       :value {:op :attrset
               :recursive false
               :fields [nested]}})))

(defn attrset-literal?
  [expression]
  (and (map? expression) (= :attrset (:op expression))))

(defn merge-attr-fields
  "Merge field maps by name. Nested attrset literals merge recursively."
  [parser fields field]
  (let [name (:name field)
        existing-idx (first (keep-indexed
                             (fn [idx item]
                               (when (= name (:name item)) idx))
                             fields))]
    (if (nil? existing-idx)
      (conj fields field)
      (let [existing (nth fields existing-idx)
            existing-value (:value existing)
            next-value (:value field)]
        (if (and (attrset-literal? existing-value)
                 (attrset-literal? next-value))
          (assoc fields existing-idx
                 {:name name
                  :value {:op :attrset
                          :recursive (boolean (or (:recursive existing-value)
                                                  (:recursive next-value)))
                          :fields (reduce (fn [acc item]
                                            (merge-attr-fields parser acc item))
                                          (:fields existing-value)
                                          (:fields next-value))}})
          (parse-failure! parser
                          "duplicate-attrset-binding"
                          {"name" name}))))))

(defn parse-static-binding-path
  [parser]
  (let [first-token (current-token parser)]
    (when-not (attribute-name-token? first-token)
      (parse-failure! parser "invalid-attribute-name" {}))
    (advance! parser)
    (loop [path [(:value first-token)]]
      (if (match! parser :dot)
        (let [part (current-token parser)]
          (when-not (attribute-name-token? part)
            (parse-failure! parser "invalid-attribute-name" {}))
          (advance! parser)
          (recur (conj path (:value part))))
        path))))

(defn parse-attrset [parser recursive?]
  (expect! parser :left-brace)
  (loop [fields []]
    (if (match! parser :right-brace)
      {:op :attrset :recursive recursive? :fields fields}
      (if (match! parser :inherit)
        (let [source (when (match! parser :left-paren)
                       (let [value (parse-expression parser)]
                         (expect! parser :right-paren)
                         value))]
          (let [next-fields
                (loop [inherited-fields fields]
                  (if (match! parser :semicolon)
                    inherited-fields
                    (let [name-token (current-token parser)
                          _ (when-not (attribute-name-token? name-token)
                              (parse-failure! parser
                                              "invalid-attribute-name"
                                              {}))
                          _ (advance! parser)
                          name (:value name-token)]
                      (when (some #(= name (:name %)) inherited-fields)
                        (parse-failure! parser
                                        "duplicate-attrset-binding"
                                        {"name" name}))
                      (recur (conj inherited-fields
                                   {:name name
                                    :lexical-inherit (nil? source)
                                    :value (if source
                                             {:op :select
                                              :target source
                                              :name name}
                                             {:op :variable :name name})})))))]
            (recur next-fields)))
        (let [key-token (current-token parser)]
          (if (= :dynamic-start (:kind key-token))
            (let [key (parse-dynamic-attribute parser)]
              (expect! parser :equal)
              (let [value (parse-expression parser)]
                (expect! parser :semicolon)
                (recur (conj fields (assoc key :value value)))))
            (let [path (parse-static-binding-path parser)
                  _ (expect! parser :equal)
                  value (parse-expression parser)
                  _ (expect! parser :semicolon)
                  field (nest-attr-path path value)]
              (recur (merge-attr-fields parser fields field)))))))))

(defn parse-list-element [parser]
  ;; Nix list elements are separated by whitespace. Parentheses delimit a
  ;; compound expression; bare atoms remain separate instead of becoming an
  ;; application chain (`[ f x ]` contains two values, unlike `f x`).
  (case (:kind (current-token parser))
    :left-paren (parse-selection parser)
    :identifier (parse-selection parser)
    :integer (parse-selection parser)
    :float (parse-selection parser)
    :string (parse-selection parser)
    :indented-string (parse-selection parser)
    :uri (parse-selection parser)
    :true (parse-selection parser)
    :false (parse-selection parser)
    :null (parse-selection parser)
    :left-bracket (parse-selection parser)
    :left-brace (parse-selection parser)
    :rec (parse-selection parser)
    (parse-expression parser)))

(defn parse-list [parser]
  (expect! parser :left-bracket)
  (loop [values []]
    (if (match! parser :right-bracket)
      {:op :list :values values}
      (do
        (when (= :eof (:kind (current-token parser)))
          (parse-failure! parser "unterminated-list" {}))
        (recur (conj values (parse-list-element parser)))))))

(defn parse-primary [parser]
  (let [token (current-token parser)]
    (case (:kind token)
      :integer (do (advance! parser) {:op :integer :value (:value token)})
      :float (do (advance! parser) {:op :float :value (:value token)})
      :string (do (advance! parser) {:op :string :value (:value token)})
      :indented-string
      (do
        (advance! parser)
        {:op :indented-string
         :segments
         (string-segments token)})
      :interpolated-string
      (do
        (advance! parser)
        {:op :interpolated-string
         :segments (string-segments token)})
      :uri (do (advance! parser) {:op :string :value (:value token)})
      :true (do (advance! parser) {:op :boolean :value true})
      :false (do (advance! parser) {:op :boolean :value false})
      :null (do (advance! parser) {:op :null :value nil})
      :identifier (do (advance! parser) {:op :variable :name (:value token)})
      :import (do
                (advance! parser)
                (let [path-token (current-token parser)]
                  (when-not (= :path (:kind path-token))
                    (parse-failure! parser "import-path-required" {}))
                  (advance! parser)
                  {:op :import :path (:value path-token)}))
      :left-paren (do
                    (advance! parser)
                    (let [expression (parse-expression parser)]
                      (expect! parser :right-paren)
                      expression))
      :left-bracket (parse-list parser)
      :left-brace (parse-attrset parser false)
      :rec (do
             (advance! parser)
             (parse-attrset parser true))
      (parse-failure! parser "expected-expression" {}))))

(defn parse-selection [parser]
  (loop [expression (parse-primary parser)]
    (if (match! parser :dot)
      (let [token (current-token parser)
            key (if (= :dynamic-start (:kind token))
                  (parse-dynamic-attribute parser)
                  (do
                    (when-not (attribute-name-token? token)
                      (parse-failure! parser "invalid-selection-name" {}))
                    (advance! parser)
                    (attribute-key token)))]
        (let [selection (merge {:op :select :target expression} key)]
          (let [token (current-token parser)]
            (if (and (= :identifier (:kind token))
                     (= "or" (:value token)))
              (do
                (advance! parser)
                (assoc selection :default (parse-selection parser)))
              (recur selection)))))
      expression)))

(def atom-starts
  #{:integer :float :string :interpolated-string :indented-string :uri
    :true :false :null :identifier
    :import :left-paren :left-bracket :left-brace :rec})

(defn parse-application [parser]
  (loop [expression (parse-selection parser)]
    (if (contains? atom-starts (:kind (current-token parser)))
      (recur {:op :call
              :function expression
              :argument (parse-selection parser)})
      expression)))

(defn parse-unary [parser]
  (cond
    (match! parser :bang)
    {:op :unary :operator :not :value (parse-unary parser)}

    (match! parser :minus)
    {:op :unary :operator :negate :value (parse-unary parser)}

    :else
    (parse-has-attr parser)))

(defn parse-binary-level [parser lower operators]
  (loop [left (lower parser)]
    (let [kind (:kind (current-token parser))]
      (if-let [operator (get operators kind)]
        (do
          (advance! parser)
          (recur {:op :binary
                  :operator operator
                  :left left
                  :right (lower parser)}))
        left))))

(defn parse-product [parser]
  (parse-binary-level parser parse-unary {:star :multiply :slash :divide}))

(defn parse-sum [parser]
  (parse-binary-level parser
                      parse-product
                      {:plus :add
                       :minus :subtract
                       :concat :concat}))

(defn parse-attribute-path [parser]
  (loop [names []]
    (let [token (current-token parser)]
      (let [key (if (= :dynamic-start (:kind token))
                  (parse-dynamic-attribute parser)
                  (do
                    (when-not (attribute-name-token? token)
                      (parse-failure! parser "invalid-attribute-path" {}))
                    (advance! parser)
                    (attribute-key token)))
            next-names (conj names key)]
        (if (match! parser :dot)
          (recur next-names)
          next-names)))))

(defn parse-has-attr [parser]
  (loop [target (parse-application parser)]
    (if (match! parser :question)
      (recur {:op :has-attr
              :target target
              :path (parse-attribute-path parser)})
      target)))

(defn parse-merge [parser]
  (let [left (parse-sum parser)]
    (if (match! parser :update)
      {:op :binary
       :operator :update
       :left left
       :right (parse-merge parser)}
      left)))

(defn parse-comparison [parser]
  (parse-binary-level parser
                      parse-merge
                      {:less :less
                       :less-equal :less-equal
                       :greater :greater
                       :greater-equal :greater-equal}))

(defn parse-equality [parser]
  (parse-binary-level parser
                      parse-comparison
                      {:equal-equal :equal
                       :not-equal :not-equal}))

(defn parse-and [parser]
  (parse-binary-level parser parse-equality {:and :and}))

(defn parse-or [parser]
  (parse-binary-level parser parse-and {:or :or}))

(defn token-at [parser index]
  (let [tokens (:tokens @parser)]
    (nth tokens (min index (dec (count tokens))))))

(defn matching-brace-index [parser start]
  (loop [index start depth 0]
    (let [kind (:kind (token-at parser index))]
      (cond
        (= kind :eof) nil
        (= kind :left-brace) (recur (inc index) (inc depth))
        (= kind :right-brace) (if (= depth 1)
                               index
                               (recur (inc index) (dec depth)))
        :else (recur (inc index) depth)))))

(defn pattern-lambda-ahead? [parser]
  (let [{:keys [index]} @parser
        first-kind (:kind (token-at parser index))
        brace-index (cond
                      (= first-kind :left-brace) index
                      (and (= first-kind :identifier)
                           (= :at (:kind (token-at parser (inc index))))
                           (= :left-brace
                              (:kind (token-at parser (+ index 2)))))
                      (+ index 2)
                      :else nil)]
    (when brace-index
      (when-let [close-index (matching-brace-index parser brace-index)]
        (or (= :colon (:kind (token-at parser (inc close-index))))
            (and (= :at (:kind (token-at parser (inc close-index))))
                 (= :identifier
                    (:kind (token-at parser (+ close-index 2))))
                 (= :colon
                    (:kind (token-at parser (+ close-index 3))))))))))

(defn lambda-ahead? [parser]
  (or (and (= :identifier (:kind (current-token parser)))
           (= :colon (:kind (next-token parser))))
      (pattern-lambda-ahead? parser)))

(defn parse-pattern-parameter [parser]
  (let [capture-before
        (when (and (= :identifier (:kind (current-token parser)))
                   (= :at (:kind (next-token parser))))
          (let [name (:value (advance! parser))]
            (advance! parser)
            name))]
    (expect! parser :left-brace)
    (loop [fields [] ellipsis? false]
      (if (match! parser :right-brace)
        (let [capture-after
              (when (match! parser :at)
                (:value (expect! parser :identifier)))]
          (when (and capture-before capture-after)
            (parse-failure! parser "duplicate-pattern-capture" {}))
          {:kind :attrset-pattern
           :fields fields
           :ellipsis ellipsis?
           :capture (or capture-before capture-after)})
        (if (match! parser :ellipsis)
          (do
            (when ellipsis?
              (parse-failure! parser "duplicate-pattern-ellipsis" {}))
            (match! parser :comma)
            (recur fields true))
          (let [name-token (expect! parser :identifier)
                field (cond-> {:name (:value name-token)}
                        (match! parser :question)
                        (assoc :default (parse-expression parser)))]
            (when (some #(= (:name field) (:name %)) fields)
              (parse-failure! parser
                              "duplicate-pattern-binding"
                              {"name" (:name field)}))
            (cond
              (match! parser :comma) (recur (conj fields field) ellipsis?)
              (= :right-brace (:kind (current-token parser)))
              (recur (conj fields field) ellipsis?)
              :else (parse-failure! parser
                                    "pattern-comma-required"
                                    {}))))))))

(defn parse-lambda [parser]
  (let [parameter (if (and (= :identifier (:kind (current-token parser)))
                           (= :colon (:kind (next-token parser))))
                    (:value (advance! parser))
                    (parse-pattern-parameter parser))]
    (expect! parser :colon)
    {:op :lambda
     :parameter parameter
     :body (parse-expression parser)}))

(defn parse-if [parser]
  (expect! parser :if)
  (let [condition (parse-expression parser)]
    (expect! parser :then)
    (let [then-branch (parse-expression parser)]
      (expect! parser :else)
      {:op :if
       :condition condition
       :then then-branch
       :else (parse-expression parser)})))

(defn parse-assert [parser]
  (expect! parser :assert)
  (let [condition (parse-expression parser)]
    (expect! parser :semicolon)
    {:op :assert
     :condition condition
     :body (parse-expression parser)}))

(defn parse-with [parser]
  (expect! parser :with)
  (let [scope (parse-expression parser)]
    (expect! parser :semicolon)
    {:op :with
     :scope scope
     :body (parse-expression parser)}))

(defn parse-let [parser]
  (expect! parser :let)
  (loop [bindings []]
    (if (match! parser :in)
      {:op :let
       :bindings bindings
       :body (parse-expression parser)}
      (if (match! parser :inherit)
        (let [source (when (match! parser :left-paren)
                       (let [value (parse-expression parser)]
                         (expect! parser :right-paren)
                         value))]
          (recur
           (loop [inherited-bindings bindings]
             (if (match! parser :semicolon)
               inherited-bindings
               (let [name-token (expect! parser :identifier)
                     name (:value name-token)]
                 (recur (conj inherited-bindings
                              {:name name
                               :lexical-inherit (nil? source)
                               :value (if source
                                        {:op :select
                                         :target source
                                         :name name}
                                        {:op :variable :name name})})))))))
        (let [name-token (expect! parser :identifier)]
          (expect! parser :equal)
          (let [value (parse-expression parser)]
            (expect! parser :semicolon)
            (recur (conj bindings {:name (:value name-token)
                                   :value value}))))))))

(defn parse-expression [parser]
  (case (:kind (current-token parser))
    :let (parse-let parser)
    :if (parse-if parser)
    :assert (parse-assert parser)
    :with (parse-with parser)
    (if (lambda-ahead? parser)
      (parse-lambda parser)
      (parse-or parser))))

(defn parse [source]
  (let [parser (parser-state (tokenizer/tokenize source))
        expression (parse-expression parser)]
    (when-not (= :eof (:kind (current-token parser)))
      (parse-failure! parser "trailing-input" {}))
    expression))
