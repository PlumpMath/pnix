(ns pnix-clj.unparse
  "AST -> pnix source, for emitting specialization residuals (roadmap M1).

  Output is fully parenthesized so operator precedence can never change the
  meaning; the round-trip contract is parse(unparse(ast)) == ast up to
  span/source metadata (checked by strip-positions equality in the report and
  tests). Strings containing a literal \"${\" cannot be emitted as one string
  literal without re-parsing as a template, so they are emitted as an
  equivalent concatenation — structurally different, semantically equal (those
  cases are verified by evaluation instead of structural equality)."
  (:refer-clojure :exclude [ident?])
  (:require [clojure.string :as str]))

(def lane-classification
  {:lane :core
   :scope :ast-to-source-rendering
   :role :deterministic-pnix-unparse-and-residual-rendering
   :product-runtime :allowed
   :semantic-authority :rendering-only
   :mutation :forbidden
   :admission :roundtrip-gated-upstream
   :determinism :required
   :allowed-output :pnix-source-string})

(declare unparse)

(def ^:private ident-pattern
  #"[A-Za-z_][A-Za-z0-9_\-']*")

(def ^:private keywords
  #{"let" "in" "if" "then" "else" "rec" "with" "assert" "import" "inherit"
    "or" "true" "false" "null"})

(defn- ident?
  [s]
  (and (string? s)
       (re-matches ident-pattern s)
       (not (contains? keywords s))))

(defn- escape-string
  [s]
  (-> s
      (str/replace "\\" "\\\\")
      (str/replace "\"" "\\\"")
      (str/replace "\n" "\\n")
      (str/replace "\t" "\\t")
      (str/replace "\r" "\\r")))

(defn- string-literal
  "Emit a pnix string literal. A literal \"${\" would re-parse as a template,
  so split around it and emit a concatenation of safe pieces."
  [s]
  (if (str/includes? s "${")
    (let [pieces (str/split s #"\$\{" -1)]
      (str "("
           (str/join " + \"$\" + \"{\" + "
                     (map #(str "\"" (escape-string %) "\"") pieces))
           ")"))
    (str "\"" (escape-string s) "\"")))

(defn- attr-name
  "An attribute-position name: bare identifier, quoted string, or ${expr}."
  [k]
  (cond
    (and (map? k) (= :dynamic-attr-key (:kind k)))
    (str "${" (unparse (:expr k)) "}")

    (ident? k) k
    :else (str "\"" (escape-string (str k)) "\"")))

(defn- paren
  [ast]
  (str "(" (unparse ast) ")"))

(defn- binding-entry
  "One let/attrset binding. Plain-inherit bindings must round-trip as
  `inherit n;` — emitting `n = n;` instead would self-reference in a recursive
  scope."
  [{:keys [key path value from-enclosing] :as entry}]
  (cond
    (and from-enclosing
         (string? key)
         (= :var (:op value))
         (= key (:name value)))
    (str "inherit " key "; ")

    path
    (str (str/join "." (map attr-name path)) " = " (paren value) "; ")

    :else
    (str (attr-name (or key (:name entry))) " = " (paren value) "; ")))

(defn- lambda-param
  [{:keys [param param-pattern]}]
  (if param-pattern
    (let [{:keys [params ellipsis? as]} param-pattern
          entries (concat
                   (map (fn [{:keys [name default]}]
                          (if default
                            (str name " ? " (paren default))
                            name))
                        params)
                   (when ellipsis? ["..."]))]
      (str "{ " (str/join ", " entries) " }"
           (when as (str "@" as))))
    param))

(defn unparse
  "Render an AST node back to pnix source (fully parenthesized)."
  [{:keys [op] :as ast}]
  (case op
    :int (str (:value ast))
    :float (str (:value ast))
    :bool (if (:value ast) "true" "false")
    :null "null"
    :string (string-literal (:value ast))
    :path (str (:value ast))
    :var (:name ast)

    :string-template
    (str "\""
         (apply str
                (map (fn [{:keys [kind] :as part}]
                       (case kind
                         :text (escape-string (:value part))
                         :expr (str "${" (unparse (:expr part)) "}")))
                     (:parts ast)))
         "\"")

    :list
    (str "[ " (str/join " " (map paren (:items ast))) " ]")

    :attrset
    (str (when (:recursive ast) "rec ")
         "{ " (apply str (map binding-entry (:attrs ast))) "}")

    :let
    (str "(let " (apply str (map (fn [{:keys [name value from-enclosing]}]
                                   (binding-entry {:key name
                                                   :value value
                                                   :from-enclosing from-enclosing}))
                                 (:bindings ast)))
         "in " (paren (:body ast)) ")")

    :if
    (str "(if " (paren (:condition ast))
         " then " (paren (:then ast))
         " else " (paren (:else ast)) ")")

    :assert
    (str "(assert " (paren (:condition ast)) "; " (paren (:body ast)) ")")

    :with
    (str "(with " (paren (:env-expr ast)) "; " (paren (:body ast)) ")")

    :lambda
    (str "(" (lambda-param ast) ": " (paren (:body ast)) ")")

    :select
    ;; Continuous attrPath (`:attrs`) or single-segment (`:attr`).
    (let [path (or (:attrs ast) (when-let [a (:attr ast)] [a]))]
      (str (paren (:target ast))
           (apply str (map #(str "." (attr-name %)) path))
           (when-let [d (:default ast)]
             (str " or " (paren d)))))

    :has-attr
    (str (paren (:target ast)) " ? " (attr-name (:attr ast)))

    :not
    (str "(!" (paren (:expr ast)) ")")

    :neg
    (str "(- " (paren (:expr ast)) ")")

    :import
    (str "(import " (:target ast) ")")

    :call
    (str "(" (paren (:fn ast)) " " (paren (:arg ast)) ")")

    :binary
    (str "(" (paren (:left ast)) " " (:operator ast) " " (paren (:right ast)) ")")

    (throw (ex-info "unparse: unsupported AST op" {:op op}))))

(def ^:private position-keys
  [:span :source :source-hash :attr-span :attr-spans :name-span :target-span
   :param-span :key-span :path-spans :expr-span])

(defn strip-positions
  "Remove span/source metadata recursively so structurally-equal ASTs from
  different sources compare equal."
  [x]
  (cond
    (map? x) (into {}
                   (keep (fn [[k v]]
                           (when-not (some #{k} position-keys)
                             [k (strip-positions v)])))
                   x)
    (vector? x) (mapv strip-positions x)
    (seq? x) (map strip-positions x)
    :else x))
