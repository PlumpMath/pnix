(ns pnix-clj.parser
  (:require [clojure.edn :as edn]
            [clojure.string :as str]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :source-to-ast-boundary
   :role :tokenize-parse-and-source-hash
   :product-runtime :allowed
   :semantic-authority :syntax-only
   :mutation :parse-cache-state-only
   :admission :forbidden
   :determinism :required
   :allowed-output :parsed-ast-or-held-parse-result})

(declare parse-expr parse-root parse-string-template-token)

(def ^:private min-i64-magnitude-text
  ;; abs(Long/MIN_VALUE) as unsigned text -- one more than Long/MAX_VALUE, so
  ;; it never parses as a positive long. Only valid when a unary `-`
  ;; immediately negates it (see parse-unary's fold below); a bare
  ;; occurrence is still rejected where int tokens become AST literals.
  "9223372036854775808")

(def ^:private parse-cache
  (atom {}))

(def ^:private parse-cache-stats*
  (atom {:hits 0
         :misses 0}))

(def ^:dynamic *allow-call*
  true)


(defn- normalized-source
  [source]
  (str/trim (str source)))

(defn parse-cache-key
  [source]
  (let [s (normalized-source source)]
    {:schema :pnix-clj.parse-cache-key.v0
     :source-hash (hash/sha256 s)}))

(defn clear-parse-cache!
  []
  (reset! parse-cache {})
  (reset! parse-cache-stats* {:hits 0
                              :misses 0})
  nil)

(defn parse-cache-stats
  []
  (assoc @parse-cache-stats*
         :entries (count @parse-cache)))

(defn- span
  [source]
  [0 (count source)])

(def ^:private ^:dynamic *source-hash*
  "Bound once per parse (parse-source*): every AST node carries the same
  whole-source hash, so recomputing sha256(source) PER NODE made parsing
  O(nodes x |source|) -- quadratic on node-heavy sources (oracle-probed:
  let-10k 14s, list-100k 55s, plus-chain-100k 101s, while parens-100k --
  which allocates no extra nodes -- took 1s). nil fallback keeps direct
  parse-root callers correct."
  nil)

(defn- ast
  ([source op]
   (ast source op nil nil))
  ([source op value]
   (ast source op value nil))
  ([source op value extra]
   (let [extra (or extra nil)]
     (merge {:op op
             :span (or (:span extra) (span source))
             :source-hash (or *source-hash* (hash/sha256 source))}
            (when (contains? #{:int :float :bool :null :string :path} op)
              {:value value})
            extra))))

(defn- token
  [kind text value span]
  {:kind kind
   :text text
   :value value
   :span span})

(defn- int-token-ast
  "Build an :int AST literal from a token, rejecting a bare occurrence of
  the Long/MIN_VALUE magnitude (only valid when parse-unary's `-` fold
  consumes the token directly, which never reaches this function)."
  [source tok idx]
  (when (= min-i64-magnitude-text (:text tok))
    (throw (ex-info "syntax error: integer literal out of range"
                    {:index idx :span (:span tok)})))
  [(ast source :int (:value tok) {:span (:span tok)}) (inc idx)])

(defn- string-template-literal?
  [string-lit]
  (boolean (re-find #"(?<!\\)\$\{" string-lit)))

(defn- decode-string-token
  "Decode a plain (non-template) Nix double-quoted string literal, INCLUDING its
  surrounding quotes. Nix escape rules — NOT Clojure/EDN's: `\\n` `\\r` `\\t`
  `\\\\` `\\\"` are special and `\\$` escapes a dollar, but ANY OTHER `\\c`
  yields the bare char `c` (the backslash is simply dropped) — e.g. `\"a\\ub\"`
  == \"aub\", `\"\\1\"` == \"1\". edn/read-string was wrong here: it treats
  `\\u` as a 4-hex unicode escape and throws on unknown escapes, which real Nix
  never does."
  [string-lit]
  (let [body (subs string-lit 1 (dec (count string-lit)))
        n (count body)
        sb (StringBuilder.)]
    (loop [i 0]
      (if (>= i n)
        (.toString sb)
        (let [ch (.charAt body i)]
          (if (and (= ch \\) (< (inc i) n))
            (let [esc (.charAt body (inc i))]
              (.append sb ^char (case esc
                                  \n \newline
                                  \r \return
                                  \t \tab
                                  esc))
              (recur (+ i 2)))
            (do (.append sb ch)
                (recur (inc i)))))))))

(defn- dedent-indented-string
  "Strip the common leading (space) indentation from an indented-string body, as
  Nix `'' ''` does. A leading newline right after the opening `''` is dropped.
  Tabs are not stripped, matching Nix. Indented-string escapes are decoded by
  the indented string parser after indentation is stripped."
  [body]
  (let [body (if (str/starts-with? body "\n") (subs body 1) body)
        lines (str/split body #"\n" -1)
        indents (->> lines
                     (remove str/blank?)
                     (map #(count (take-while (fn [c] (= c \space)) %))))
        min-indent (if (seq indents) (apply min indents) 0)
        strip (fn [line] (subs line (min min-indent (count line))))]
    (str/join "\n" (map strip lines))))

(defn- indented-escape
  [content idx]
  (let [n (count content)]
    (when (and (< (+ idx 2) n)
               (= \' (.charAt content idx))
               (= \' (.charAt content (inc idx))))
      (let [ch (.charAt content (+ idx 2))]
        (cond
          (= ch \$) ["$" 3]
          (= ch \') ["''" 3]
          (= ch \\) (if (< (+ idx 3) n)
                      (let [escaped (.charAt content (+ idx 3))]
                        [(case escaped
                           \n "\n"
                           \r "\r"
                           \t "\t"
                           (str escaped))
                         4])
                      ["\\" 3]))))))

(defn- indented-template-literal?
  [content]
  (let [n (count content)]
    (loop [idx 0]
      (cond
        (>= idx n) false

        (indented-escape content idx)
        (let [[_ advance] (indented-escape content idx)]
          (recur (+ idx advance)))

        (and (< (inc idx) n)
             (= \$ (.charAt content idx))
             (= \{ (.charAt content (inc idx))))
        true

        :else
        (recur (inc idx))))))

(defn- decode-indented-string
  [content]
  (let [n (count content)]
    (loop [idx 0
           out ""]
      (if (>= idx n)
        out
        (if-let [[decoded advance] (indented-escape content idx)]
          (recur (+ idx advance) (str out decoded))
          (recur (inc idx) (str out (.charAt content idx))))))))

;; --- balanced string/comment scanning -----------------------------------
;;
;; A regex cannot lex Nix strings: `"a${f "x"}b"` nests a string INSIDE the
;; interpolation (recursively, both string kinds, plus comments and braces).
;; These scanners walk the raw source like Nix's mode-switching lexer does
;; (oracle-confirmed, D7). They are shared by tokenize and the template
;; splitters, so both agree on where a splice ends.

(declare scan-dquote-end scan-indented-end)

(defn- scan-line-comment-end
  [^String s i]
  (let [n (count s)]
    (loop [j i]
      (if (or (>= j n) (= \newline (.charAt s j))) j (recur (inc j))))))

(defn- scan-block-comment-end
  "From i at `/*`, return the index after the closing `*/`."
  [^String s i]
  (let [n (count s)]
    (loop [j (+ i 2)]
      (cond
        (>= (inc j) n)
        (throw (ex-info "unterminated block comment" {:span [i n]}))

        (and (= \* (.charAt s j)) (= \/ (.charAt s (inc j))))
        (+ j 2)

        :else (recur (inc j))))))

(defn- scan-legacy-escaped-string-end
  "From i at the backslash of an opening `\\\"`: this dialect historically
  accepted \\\"-delimited strings inside splices (real Nix REJECTS them —
  documented leniency, kept for existing sources). Returns the index after
  the closing `\\\"`."
  [^String s i]
  (let [n (count s)]
    (loop [j (+ i 2)]
      (cond
        (>= j n)
        (throw (ex-info "unterminated string" {:span [i n]}))

        (and (= \\ (.charAt s j)) (< (inc j) n) (= \" (.charAt s (inc j))))
        (+ j 2)

        (= \\ (.charAt s j)) (recur (+ j 2))
        :else (recur (inc j))))))

(defn- ident-part-char?
  [ch]
  (or (Character/isLetterOrDigit ^char ch) (= ch \_) (= ch \-) (= ch \')))

(defn- scan-splice-end
  "From i just after an opening `${`, return the index AFTER the matching `}`.
  Skips strings (both kinds, recursively — they may contain further splices),
  comments, and brace pairs; identifiers are consumed wholesale so a trailing
  apostrophe (foo') is never mistaken for a `''` string opener."
  [^String s i]
  (let [n (count s)]
    (loop [j i
           depth 0]
      (if (>= j n)
        (throw (ex-info "unterminated string interpolation" {:span [i n]}))
        (let [ch (.charAt s j)]
          (cond
            (= ch \")
            (recur (long (scan-dquote-end s j)) depth)

            (and (= ch \\) (< (inc j) n) (= \" (.charAt s (inc j))))
            (recur (long (scan-legacy-escaped-string-end s j)) depth)

            (and (= ch \') (< (inc j) n) (= \' (.charAt s (inc j))))
            (recur (long (scan-indented-end s j)) depth)

            (or (Character/isLetter ^char ch) (= ch \_))
            (recur (long (loop [k (inc j)]
                           (if (and (< k n) (ident-part-char? (.charAt s k)))
                             (recur (inc k))
                             k)))
                   depth)

            (= ch \#)
            (recur (long (scan-line-comment-end s j)) depth)

            (and (= ch \/) (< (inc j) n) (= \* (.charAt s (inc j))))
            (recur (long (scan-block-comment-end s j)) depth)

            (= ch \{) (recur (inc j) (inc depth))

            (= ch \})
            (if (zero? depth)
              (inc j)
              (recur (inc j) (dec depth)))

            :else (recur (inc j) depth)))))))

(defn- scan-dquote-end
  "From i at an opening `\"`, return the index after the closing `\"`,
  crossing `${...}` splices (which may nest strings recursively)."
  [^String s i]
  (let [n (count s)]
    (loop [j (inc i)]
      (if (>= j n)
        (throw (ex-info "unterminated string" {:span [i n]}))
        (let [ch (.charAt s j)]
          (cond
            (= ch \\) (recur (+ j 2))
            (= ch \") (inc j)
            (and (= ch \$) (< (inc j) n) (= \{ (.charAt s (inc j))))
            (recur (long (scan-splice-end s (+ j 2))))
            :else (recur (inc j))))))))

(defn- scan-indented-end
  "From i at an opening `''`, return the index after the closing `''`,
  honoring the ''$ / ''' / ''\\x escapes and crossing `${...}` splices."
  [^String s i]
  (let [n (count s)]
    (loop [j (+ i 2)]
      (if (>= (inc j) n)
        (throw (ex-info "unterminated indented string" {:span [i n]}))
        (let [ch (.charAt s j)
              ch2 (.charAt s (inc j))]
          (cond
            (and (= ch \') (= ch2 \'))
            (let [ch3 (when (< (+ j 2) n) (.charAt s (+ j 2)))]
              (case ch3
                \$ (recur (+ j 3))
                \' (recur (+ j 3))
                \\ (recur (+ j 4))
                (+ j 2)))

            (and (= ch \$) (= ch2 \{))
            (recur (long (scan-splice-end s (+ j 2))))

            :else (recur (inc j))))))))

(def ^:private token-pattern
  ;; Strings and comments are scanned BY HAND above (balanced ${...} nesting
  ;; — a regex cannot lex them); this pattern lexes everything else, anchored
  ;; at the current position via .region + .lookingAt. Number alternative
  ;; sits BEFORE punct so a dot-leading/trailing float wins over the `.`
  ;; select token, reproducing flex maximal munch (D4): float =
  ;; (([1-9][0-9]*\.[0-9]*)|(0?\.[0-9]+))([Ee][+-]?[0-9]+)? — `.5`, `1.`,
  ;; `2.5e-2` are floats, while `1e3` is `1 e3` and `00.5` is `00 .5`.
  #"(?s)(?:(<[A-Za-z0-9._+/-]+>|(?:\.\.?|~)/[A-Za-z0-9._+-]+(?:/[A-Za-z0-9._+-]+)*)|((?:[1-9][0-9]*\.[0-9]*|0?\.[0-9]+)(?:[Ee][+-]?[0-9]+)?|[0-9]+)|(\$\{|->|&&|\|\||!=|<=|>=|==|//|\+\+|\.\.\.|[\{\}\[\]\(\)=;:,\+\-\*/%<>.\?!@])|([A-Za-z_][A-Za-z0-9_\-']*)|(.))")

(defn- ascii-alpha?
  [ch]
  (or (<= (int \A) (int ch) (int \Z))
      (<= (int \a) (int ch) (int \z))))

(defn- ascii-digit?
  [ch]
  (<= (int \0) (int ch) (int \9)))

(defn- uri-scheme-char?
  [ch]
  (or (ascii-alpha? ch)
      (ascii-digit? ch)
      (contains? #{\+ \- \.} ch)))

(defn- uri-body-char?
  [ch]
  (or (ascii-alpha? ch)
      (ascii-digit? ch)
      (contains? #{\% \/ \? \: \@ \& \= \+ \$ \, \- \_ \. \! \~ \* \'} ch)))

(defn- scan-uri-end
  "Return the end of a Nix 2.34.7 URI token at `start`, or nil. The URI is a
  lexer atom, not a quoted attribute name; callers must retain that token-kind
  distinction."
  [^String source start]
  (let [n (count source)]
    (when (and (< start n) (ascii-alpha? (.charAt source start)))
      (let [colon (loop [i (inc start)]
                    (if (and (< i n) (uri-scheme-char? (.charAt source i)))
                      (recur (inc i))
                      i))]
        (when (and (< colon n) (= \: (.charAt source colon)))
          (let [body-start (inc colon)
                end (loop [i body-start]
                      (if (and (< i n) (uri-body-char? (.charAt source i)))
                        (recur (inc i))
                        i))]
            (when (> end body-start) end)))))))

(defn- parse-float-literal
  "Parse a Nix float token, including Nix's `strtod` ERANGE rejection.

  Nix 2.34.7 rejects non-zero literals that overflow OR produce a subnormal
  f64 (including values that round all the way to zero). A genuinely zero
  mantissa remains zero regardless of exponent. This is deliberately a lexer
  rule: `builtins.fromJSON` has different oracle semantics and accepts
  subnormal JSON numbers."
  [literal span]
  (let [mantissa (first (str/split literal #"[eE]" 2))
        nonzero-mantissa? (boolean (re-find #"[1-9]" mantissa))
        value (try
                (Double/parseDouble literal)
                (catch NumberFormatException cause
                  (throw (ex-info (str "invalid float '" literal "'")
                                  {:reason :invalid-float-literal
                                   :literal literal
                                   :span span}
                                  cause))))]
    (when (and nonzero-mantissa?
               (or (not (Double/isFinite value))
                   (< (Math/abs value) Double/MIN_NORMAL)))
      (throw (ex-info (str "invalid float '" literal "'")
                      {:reason :invalid-float-literal
                       :literal literal
                       :span span})))
    value))

(defn- tokenize
  [source]
  ;; Number alternative sits BEFORE punct so a dot-leading/trailing float wins
  ;; over the `.` select token, reproducing flex maximal munch in real Nix
  ;; (oracle-confirmed, D4): float = (([1-9][0-9]*\.[0-9]*)|(0?\.[0-9]+))
  ;; ([Ee][+-]?[0-9]+)? — so `.5`, `1.`, `2.5e-2` are floats, while `1e3` is
  ;; `1 e3` (application) and `00.5` is `00 .5`, exactly as Nix lexes them.
  (let [^String source source
        n (count source)
        matcher (re-matcher token-pattern source)]
    (loop [pos 0
           tokens []]
      (if (>= pos n)
        tokens
        (let [ch (.charAt source pos)
              uri-end (when (ascii-alpha? ch)
                        (scan-uri-end source pos))]
          (cond
            (Character/isWhitespace ch)
            (recur (inc pos) tokens)

            ;; comments: `#` to end of line, `/* ... */` (real Nix has both;
            ;; block comments were missing pre-D7, oracle-confirmed)
            (= ch \#)
            (recur (long (scan-line-comment-end source pos)) tokens)

            (and (= ch \/) (< (inc pos) n) (= \* (.charAt source (inc pos))))
            (recur (long (scan-block-comment-end source pos)) tokens)

            ;; strings: balanced hand scan — `"a${f "x"}b"` nests strings
            ;; inside splices, beyond any regex (D7)
            (= ch \")
            (let [end (long (scan-dquote-end source pos))
                  text (subs source pos end)
                  span [pos end]]
              (recur end
                     (conj tokens
                           (if (string-template-literal? text)
                             (token :string-template text text span)
                             (token :string text (decode-string-token text) span)))))

            (and (= ch \') (< (inc pos) n) (= \' (.charAt source (inc pos))))
            (let [end (long (scan-indented-end source pos))
                  text (subs source pos end)
                  body (subs text 2 (- (count text) 2))
                  dedented (dedent-indented-string body)
                  template? (indented-template-literal? dedented)
                  span [pos end]]
              (recur end
                     (conj tokens
                           (token (if template? :indented-template :string)
                                  text
                                  (if template?
                                    dedented
                                    (decode-indented-string dedented))
                                  span))))

            uri-end
            (let [text (subs source pos uri-end)]
              (recur (long uri-end)
                     (conj tokens (token :uri text text [pos uri-end]))))

            :else
            (do
              (.region matcher pos n)
              (when-not (.lookingAt matcher)
                (throw (ex-info "unsupported token"
                                {:token (str ch)
                                 :span [pos (inc pos)]})))
              (let [path-lit (.group matcher 1)
                    int-lit (.group matcher 2)
                    punct (.group matcher 3)
                    ident (.group matcher 4)
                    unknown (.group matcher 5)
                    end (.end matcher)]
                (cond
                  ;; path literals: `./x` `../x` `~/x` and `<search/path>`.
                  ;; Only these forms (a `.`/`..`/`~`/`<` prefix) are paths, so
                  ;; division `a / b`, `1/0`, `//`, and comparisons are
                  ;; unaffected. Path resolution (relative-to-file, NIX_PATH)
                  ;; is a frontier; a path evaluates to its literal text.
                  path-lit
                  (recur (long end)
                         (conj tokens (token :path path-lit path-lit [pos end])))

                  int-lit
                  (recur (long end)
                         (conj tokens
                               (token (if (str/includes? int-lit ".")
                                        :float
                                        :int)
                                      int-lit
                                      (if (str/includes? int-lit ".")
                                        (parse-float-literal int-lit [pos end])
                                        (if (= int-lit min-i64-magnitude-text)
                                          Long/MIN_VALUE
                                          (Long/parseLong int-lit)))
                                      [pos end])))

                  punct
                  (recur (long end)
                         (conj tokens (token :punct punct punct [pos end])))

                  ident
                  (recur (long end)
                         (conj tokens (token :ident ident ident [pos end])))

                  :else
                  (throw (ex-info "unsupported token"
                                  {:token unknown
                                   :span [pos end]})))))))))))

(defn- expect-text
  [tokens idx text]
  (let [tok (nth tokens idx nil)]
    (when-not (= text (:text tok))
      (throw (ex-info "unexpected token"
                      {:expected text
                       :got (:text tok)
                       :index idx
                       :span (or (:span tok) [idx idx])})))
    (inc idx)))

(defn- ast-span
  [node]
  (:span node))

(defn- combine-span
  [left right]
  [(first (ast-span left)) (second (ast-span right))])

(declare guard-operator-operand parse-postfix)

(defn- parse-list
  [source tokens idx]
  (let [open (nth tokens idx)]
    (loop [i (inc idx)
           items []]
      (let [tok (nth tokens i nil)]
        (cond
          (nil? tok)
          (throw (ex-info "unterminated list"
                          {:index idx
                           :span (:span open)}))

          (= "]" (:text tok))
          [(ast source :list nil {:items items
                                  :span [(first (:span open))
                                         (second (:span tok))]})
           (inc i)]

          :else
          ;; Nix list elements are expr_select ONLY (oracle-confirmed): an
          ;; unparenthesized operator expression (`[ 1 + 2 ]`, `[ 2 / 0 ]`,
          ;; `[ !true ]`, `[ -5 ]`, `[ { } ? a ]`) is a syntax error in real
          ;; Nix, so elements parse at the postfix level, not parse-expr.
          (let [_ (guard-operator-operand tokens i "a list element")
                [item i'] (binding [*allow-call* false]
                            (parse-postfix source tokens i))]
            (recur i' (conj items item))))))))

(defn- parse-attr-key
  [source tok]
  (case (:kind tok)
    :ident (:text tok)
    :string (:value tok)
    :string-template {:kind :dynamic-attr-key
                      :expr (parse-string-template-token source tok)}
    (throw (ex-info "unsupported attr key"
                    {:token tok
                     :span (:span tok)}))))

(defn- dynamic-key-token?
  [tok]
  (and (= :punct (:kind tok))
       (= "${" (:text tok))))

(defn- parse-dynamic-attr-key
  "Parse a bare `${ expr }` dynamic attribute key. `idx` points at the `${`
  token; returns [key index-after-}]. D22 (oracle-gated): real Nix FOLDS
  a literal-string interpolation to a STATIC key at parse -- both orders of
  a folded-key collision are PARSE-time 'attribute already defined' (even
  unforced inside a let), and a folded key is legal as a let name -- while a
  genuinely dynamic expression stays a dynamic key, and QUOTED template keys
  keep their eval-time 'dynamic attribute' behavior (parse-attr-key,
  unfolded)."
  [source tokens idx]
  (let [[expr i] (binding [*allow-call* true]
                   (parse-expr source tokens (inc idx)))
        i-close (expect-text tokens i "}")]
    [(if (= :string (:op expr))
       (:value expr)
       {:kind :dynamic-attr-key :expr expr})
     i-close]))

(defn- parse-attr-path
  "Parse a dotted attribute path `k0 (. k1)*` used as the LHS of an attrset
  binding (e.g. `a.b.c = v`). Each segment is an ident, string, string
  template, or bare `${ expr }` dynamic key. Returns [path-vector
  index-after-path]; a single non-dotted key yields a 1-element path."
  [source tokens idx]
  (let [seg (fn [j]
              (let [tok (nth tokens j nil)]
                (if (dynamic-key-token? tok)
                  (let [[key end] (parse-dynamic-attr-key source tokens j)]
                    [key end [(first (:span tok))
                              (second (:span (nth tokens (dec end))))]])
                  [(parse-attr-key source tok) (inc j) (:span tok)])))
        [k0 end0 span0] (seg idx)]
    (loop [path [k0] spans [span0] i end0]
      (let [tok (nth tokens i nil)]
        (if (and (= :punct (:kind tok)) (= "." (:text tok)))
          (let [[k seg-end span] (seg (inc i))]
            (recur (conj path k) (conj spans span) seg-end))
          [path i spans])))))

(defn- parse-inherit
  [source tokens idx]
  (let [open (nth tokens idx)]
    (let [[source-expr i0]
          (if (= "(" (:text (nth tokens (inc idx) nil)))
            (let [[expr i-expr] (binding [*allow-call* true]
                                  (parse-expr source tokens (+ idx 2)))
                  i-close (expect-text tokens i-expr ")")]
              [expr i-close])
            [nil (inc idx)])]
      (loop [i i0
           attrs []]
        (let [tok (nth tokens i nil)]
          (cond
            (nil? tok)
            (throw (ex-info "unterminated inherit"
                            {:index idx
                             :span (:span open)}))

            (= ";" (:text tok))
            [attrs (inc i)]

            (not= :ident (:kind tok))
            (throw (ex-info "expected inherit name"
                            {:token tok
                             :span (:span tok)}))

            :else
            (let [name (:text tok)
                  value (if source-expr
                          (ast source :select nil
                               {:target source-expr
                                :attr name
                                :attr-span (:span tok)
                                :span [(first (ast-span source-expr))
                                       (second (:span tok))]})
                          (ast source :var nil
                               {:name name
                                :span (:span tok)}))]
              (recur (inc i)
                     (conj attrs {:key name
                                  :key-span (:span tok)
                                  :value value
                                  ;; Plain `inherit x` copies x from the
                                  ;; enclosing scope; `inherit (e) x` selects
                                  ;; from e in the current scope. Consumers that
                                  ;; build a recursive env (let) use this flag to
                                  ;; resolve plain inherits against the outer env
                                  ;; and avoid a self-reference cycle.
                                  :from-enclosing (nil? source-expr)})))))))))

(defn- literal-attrset?
  [node]
  (= :attrset (:op node)))

(declare merge-attr-bindings)

(defn- path->nested
  "Convert a dotted binding `k1.k2...kn = v` into a plain `k1 = ...` entry
  whose value is a synthetic nested attrset literal, so it can merge with
  sibling literals exactly like real Nix's parse-time addAttr. D21: segments
  may be DYNAMIC (`a.${k}.b = v`) -- each becomes the dynamic key of its own
  nested literal, which is exactly Nix's nested-lazy semantics: a dynamic
  sub-key evaluates only when its PARENT set is forced ({ a.${1} = 1; }
  passes unforced; attrNames sees only 'a'), and literal/path siblings merge
  in both orders. A dynamic FIRST segment yields a dynamic-key binding, which
  stays unmergeable at parse (evaluated at construction, D20 checks apply)."
  [source {:keys [path path-spans value]}]
  (let [[k & ks] path
        span (first path-spans)
        ;; Nix threads the first attrpath segment's pos through every nested
        ;; ExprAttrs for `a.b.c = v` (cljs/hy oracle).
        rest-spans (when (seq ks) (vec (repeat (count ks) span)))]
    {:key k
     :key-span span
     :value (if (empty? ks)
              value
              (ast source :attrset nil
                   {:attrs [(path->nested
                             source
                             {:path (vec ks) :path-spans rest-spans :value value})]
                    :recursive false
                    :span (ast-span value)}))}))

(defn- merge-attr-bindings
  "Real Nix merges attrset bindings AT PARSE TIME (addAttr): a static dotted
  path nests, and two bindings of the same key merge iff BOTH values are
  attrset LITERALS (any :recursive flag; the merged set keeps rec if either
  side had it — oracle-confirmed on nix-instantiate 2.34.7, D10). Anything
  else — an expression, a variable, an inherit — is `attribute already
  defined`, a parse error. Dynamic KEYS cannot merge statically and pass
  through for the evaluator; dynamic-SEGMENT paths desugar to nested lazy
  literals (D21) so their static prefixes merge like any literal."
  [source entries]
  (loop [remaining entries
         order []
         by-key {}]
    (if-let [e (first remaining)]
      (let [e (if (:path e)
                ;; D21: EVERY dotted path desugars to nested literals here
                ;; (dynamic segments included) -- no :path binding survives
                ;; to the evaluator from an attrset.
                (path->nested source e)
                e)
            k (:key e)]
        (if-not (string? k)
          ;; dynamic key / dynamic-segment path: not statically mergeable
          (recur (rest remaining) (conj order [::dynamic e]) by-key)
          (if-let [existing (get by-key k)]
            (let [v1 (:value existing)
                  v2 (:value e)]
              (when-not (and (literal-attrset? v1) (literal-attrset? v2)
                             (not (:from-enclosing existing))
                             (not (:from-enclosing e)))
                (throw (ex-info (str "attribute `" k "` already defined "
                                     "(Nix merges only attrset literals)")
                                {:attr k
                                 :span (or (:key-span e) (ast-span (:value e)))})))
              (let [merged (ast source :attrset nil
                                {:attrs (merge-attr-bindings
                                         source
                                         (into (vec (:attrs v1)) (:attrs v2)))
                                 :recursive (boolean (or (:recursive v1)
                                                         (:recursive v2)))
                                 :span [(first (ast-span v1))
                                        (second (ast-span v2))]})]
                (recur (rest remaining)
                       order
                       (assoc by-key k (assoc existing :value merged)))))
            (recur (rest remaining)
                   (conj order [::static k])
                   (assoc by-key k e)))))
      (mapv (fn [[kind x]]
              (if (= ::static kind) (get by-key x) x))
            order))))

(defn- parse-attrset
  ([source tokens idx]
   (parse-attrset source tokens idx false))
  ([source tokens idx recursive?]
   (let [open (nth tokens idx)]
    (loop [i (inc idx)
           attrs []]
      (let [tok (nth tokens i nil)]
        (cond
          (nil? tok)
          (throw (ex-info "unterminated attrset"
                          {:index idx
                           :span (:span open)}))

          (= "}" (:text tok))
          [(ast source :attrset nil {:attrs (merge-attr-bindings source attrs)
                                     :recursive recursive?
                                     :span [(first (:span open))
                                            (second (:span tok))]})
           (inc i)]

          (and (= :ident (:kind tok))
               (= "inherit" (:text tok)))
          (let [[inherited i'] (parse-inherit source tokens i)]
            (recur i' (into attrs inherited)))

          :else
          (let [[path key-end path-spans] (parse-attr-path source tokens i)
                i= (expect-text tokens key-end "=")
                [v iv] (binding [*allow-call* true]
                         (parse-expr source tokens i=))
                i-semi (expect-text tokens iv ";")]
            ;; A single key keeps the flat {:key ...} shape (unchanged eval
            ;; path); a dotted path carries {:path ...} for the evaluator to
            ;; merge into nested attrsets.
            (recur i-semi (conj attrs (if (= 1 (count path))
                                        {:key (first path)
                                         :key-span (first path-spans)
                                         :value v}
                                        {:path path
                                         :path-spans path-spans
                                         :value v}))))))))))

(defn- parse-let
  "D22 (oracle-gated): let bindings are the SAME binds production as attrsets
  in real Nix -- dotted paths nest and MERGE at parse (let a.b = 1; a.c = 2;
  in a.c is 2; literal<->path both orders), a duplicate name is a PARSE error
  (let x = 1; x = 2 -- the pre-D22 parser silently let the later binding
  shadow, oracle-wrong), dynamic SUB-segments ride inside the desugared
  nested attrset value (D21 lazy semantics), and a TOP key still dynamic
  after the literal fold is real Nix's 'dynamic attributes not allowed in
  let'."
  [source tokens idx]
  (let [open (nth tokens idx)
        finish (fn [entries body i']
                 (let [merged (merge-attr-bindings source entries)
                       bindings
                       (mapv (fn [{:keys [key key-span value from-enclosing]}]
                               (when-not (string? key)
                                 (throw (ex-info
                                         "dynamic attributes not allowed in let"
                                         {:key key :span key-span})))
                               {:name key
                                :name-span key-span
                                :value value
                                :from-enclosing from-enclosing})
                             merged)]
                   [(ast source :let nil {:bindings bindings
                                          :body body
                                          :span [(first (:span open))
                                                 (second (ast-span body))]})
                    i']))]
    (loop [i (inc idx)
           entries []]
      (let [tok (nth tokens i nil)]
        (cond
          (nil? tok)
          (throw (ex-info "unterminated let"
                          {:index idx
                           :span (:span open)}))

          (and (= :ident (:kind tok))
               (= "in" (:text tok)))
          (let [[body i'] (parse-expr source tokens (inc i))]
            (finish entries body i'))

          (and (= :ident (:kind tok))
               (= "inherit" (:text tok)))
          (let [[inherited i'] (parse-inherit source tokens i)]
            ;; inherit entries are already attr-entry shaped ({:key ...
            ;; :from-enclosing ...}); a collision with them is 'attribute
            ;; already defined' via the shared merge, exactly like attrsets.
            (recur i' (into entries
                            (map #(assoc % :key-span (:span tok)))
                            inherited)))

          :else
          (let [[path key-end path-spans] (parse-attr-path source tokens i)
                i= (expect-text tokens key-end "=")
                [value iv] (binding [*allow-call* true]
                             (parse-expr source tokens i=))
                i-semi (expect-text tokens iv ";")]
            (recur i-semi
                   (conj entries
                         (if (= 1 (count path))
                           {:key (first path)
                            :key-span (first path-spans)
                            :value value}
                           {:path path
                            :path-spans path-spans
                            :value value})))))))))

(defn- parse-if
  [source tokens idx]
  (let [open (nth tokens idx)
        [condition i-condition] (binding [*allow-call* true]
                                  (parse-expr source tokens (inc idx)))
        i-then (expect-text tokens i-condition "then")
        [then-expr i-then-expr] (binding [*allow-call* true]
                                  (parse-expr source tokens i-then))
        i-else (expect-text tokens i-then-expr "else")
        [else-expr i-else-expr] (binding [*allow-call* true]
                                  (parse-expr source tokens i-else))]
    [(ast source :if nil {:condition condition
                          :then then-expr
                          :else else-expr
                          :span [(first (:span open))
                                 (second (ast-span else-expr))]})
     i-else-expr]))

(defn- parse-assert
  [source tokens idx]
  (let [open (nth tokens idx)
        [condition i-cond] (binding [*allow-call* true]
                             (parse-expr source tokens (inc idx)))
        i-semi (expect-text tokens i-cond ";")
        [body i-body] (binding [*allow-call* true]
                        (parse-expr source tokens i-semi))]
    [(ast source :assert nil {:condition condition
                              :body body
                              :span [(first (:span open))
                                     (second (ast-span body))]})
     i-body]))

(defn- parse-with
  [source tokens idx]
  (let [open (nth tokens idx)
        [env-expr i-env] (binding [*allow-call* true]
                           (parse-expr source tokens (inc idx)))
        i-semi (expect-text tokens i-env ";")
        [body i-body] (binding [*allow-call* true]
                        (parse-expr source tokens i-semi))]
    [(ast source :with nil {:env-expr env-expr
                            :body body
                            :span [(first (:span open))
                                   (second (ast-span body))]})
     i-body]))

(defn- parse-lambda
  [source tokens idx]
  (let [param (nth tokens idx)
        i-body (expect-text tokens (inc idx) ":")
        [body i'] (parse-expr source tokens i-body)]
    [(ast source :lambda nil {:param (:text param)
                              :param-span (:span param)
                              :body body
                              :span [(first (:span param))
                                     (second (ast-span body))]})
     i']))

(defn- matching-brace-index
  [tokens idx]
  (loop [i idx
         depth 0]
    (let [tok (nth tokens i nil)]
      (cond
        (nil? tok) nil
        (= "{" (:text tok)) (recur (inc i) (inc depth))
        (= "}" (:text tok)) (if (= 1 depth)
                              i
                              (recur (inc i) (dec depth)))
        :else (recur (inc i) depth)))))

(defn- paramset-lambda-start?
  [tokens idx]
  (when-let [close-idx (matching-brace-index tokens idx)]
    (let [after (nth tokens (inc close-idx) nil)]
      (or (= ":" (:text after))
          ;; `{ ... }@name:` -- attr pattern with a whole-arg `@` binding.
          (and (= "@" (:text after))
               (= :ident (:kind (nth tokens (+ close-idx 2) nil)))
               (= ":" (:text (nth tokens (+ close-idx 3) nil))))))))

(defn- parse-param-default
  [source tokens idx]
  (binding [*allow-call* true]
    (parse-expr source tokens idx)))

(defn- parse-paramset-lambda
  ([source tokens idx]
   (parse-paramset-lambda source tokens idx nil))
  ([source tokens idx leading-as]
  (let [open (nth tokens idx)]
    (loop [i (inc idx)
           params []
           ellipsis? false]
      (let [tok (nth tokens i nil)]
        (cond
          (nil? tok)
          (throw (ex-info "unterminated parameter set"
                          {:index idx
                           :span (:span open)}))

          (= "}" (:text tok))
          ;; after `}` accept `: body`, or `@ name : body` when there is no
          ;; leading `name@` binding already.
          (let [after (nth tokens (inc i) nil)
                [as-name colon-idx]
                (cond
                  leading-as
                  [leading-as (inc i)]

                  (= "@" (:text after))
                  (let [name-tok (nth tokens (+ i 2) nil)]
                    (when-not (= :ident (:kind name-tok))
                      (throw (ex-info "expected name after @ in pattern"
                                      {:token name-tok
                                       :span (or (:span name-tok)
                                                 (:span after))})))
                    [(:text name-tok) (+ i 3)])

                  :else
                  [nil (inc i)])
                i-body (expect-text tokens colon-idx ":")
                [body i'] (parse-expr source tokens i-body)]
            [(ast source :lambda nil
                  {:param-pattern {:kind :attr-pattern
                                   :params params
                                   :ellipsis? ellipsis?
                                   :as as-name}
                   :body body
                   :span [(first (:span open))
                          (second (ast-span body))]})
             i'])

          (= "," (:text tok))
          (recur (inc i) params ellipsis?)

          (= "..." (:text tok))
          (recur (inc i) params true)

          (not= :ident (:kind tok))
          (throw (ex-info "expected parameter name"
                          {:token tok
                           :span (:span tok)}))

          :else
          (let [name (:text tok)
                next-token (nth tokens (inc i) nil)]
            (if (= "?" (:text next-token))
              (let [[default i-default] (parse-param-default source
                                                             tokens
                                                             (+ i 2))]
                (recur i-default
                       (conj params {:name name
                                     :name-span (:span tok)
                                     :default default})
                       ellipsis?))
              (recur (inc i)
                     (conj params {:name name
                                   :name-span (:span tok)})
                     ellipsis?)))))))))

(defn- import-end-token?
  [tok]
  (or (nil? tok)
      (and (= :punct (:kind tok))
           (contains? #{";" ")" "]" "}"} (:text tok)))
      (and (= :ident (:kind tok))
           (contains? #{"in" "then" "else"} (:text tok)))))

(defn- parse-import
  [source tokens idx]
  (let [open (nth tokens idx)]
    (loop [i (inc idx)
           target-tokens []]
      (let [tok (nth tokens i nil)]
        (cond
          (and (empty? target-tokens) (import-end-token? tok))
          (throw (ex-info "missing import target"
                          {:index idx
                           :span (:span open)}))

          (import-end-token? tok)
          (let [target (apply str (map :text target-tokens))]
            [(ast source :import nil
                  {:target target
                   :target-span [(first (:span (first target-tokens)))
                                 (second (:span (last target-tokens)))]
                   :span [(first (:span open))
                          (second (:span (last target-tokens)))]})
             i])

          :else
          (recur (inc i) (conj target-tokens tok)))))))

(defn- decode-string-fragment
  [fragment]
  (edn/read-string (str "\"" fragment "\"")))

(defn- parse-template-expr
  [expr-source]
  (binding [*allow-call* true]
    (parse-root (str/trim expr-source))))

(def ^:private absolute-path-pattern
  #"/[A-Za-z0-9._+-]+(?:/[A-Za-z0-9._+-]+)*")

(defn- absolute-path-end
  [source tokens idx]
  (let [tok (nth tokens idx nil)]
    (when (and (= :punct (:kind tok))
               (= "/" (:text tok)))
      (let [start (first (:span tok))
            matcher (re-matcher absolute-path-pattern (subs source start))]
        (when (.lookingAt matcher)
          (+ start (.end matcher)))))))

(defn- token-index-after-end
  [tokens idx end]
  (loop [i idx]
    (let [tok (nth tokens i nil)]
      (if (and tok (<= (second (:span tok)) end))
        (recur (inc i))
        i))))

(defn- parse-absolute-path
  [source tokens idx]
  (when-let [end (absolute-path-end source tokens idx)]
    (let [start (first (:span (nth tokens idx)))]
      [(ast source :path (subs source start end) {:span [start end]})
       (token-index-after-end tokens idx end)])))

(defn- absolute-path-argument-start?
  [source tokens idx]
  (when-let [end (absolute-path-end source tokens idx)]
    (let [prev-token (nth tokens (dec idx) nil)
          slash-token (nth tokens idx)]
      (and prev-token
           (< (second (:span prev-token)) (first (:span slash-token)))
           end))))

(defn- template-expr-source
  [content start close]
  (str/replace (subs content start close) "\\\"" "\""))

(defn- interpolation-close-index
  "Index of the `}` closing a splice whose expression starts at expr-start,
  or nil when unterminated. Uses the same balanced scanner as the tokenizer
  (strings of both kinds, nested splices, comments, brace depth — D7), so the
  splitter and the lexer always agree on the splice extent."
  [content expr-start]
  (try
    (dec (long (scan-splice-end content expr-start)))
    (catch clojure.lang.ExceptionInfo _ nil)))

(defn- parse-string-template-token
  [source tok]
  (let [text (:text tok)
        content (subs text 1 (dec (count text)))]
    (loop [idx 0
           parts []]
      (if-let [open (str/index-of content "${" idx)]
        (let [close (interpolation-close-index content (+ open 2))]
          (when-not close
            (throw (ex-info "unterminated string interpolation"
                            {:token tok
                             :span (:span tok)})))
          (let [literal (subs content idx open)
                expr-source (template-expr-source content (+ open 2) close)
                expr (parse-template-expr expr-source)]
            (recur (inc close)
                   (cond-> parts
                     (not (str/blank? literal))
                     (conj {:kind :text
                            :value (decode-string-fragment literal)})

                     true
                     (conj {:kind :expr
                            :expr expr})))))
        (let [literal (subs content idx)]
          (ast source :string-template nil
               {:parts (cond-> parts
                         (not (str/blank? literal))
                         (conj {:kind :text
                                :value (decode-string-fragment literal)}))
                :span (:span tok)}))))))

(defn- parse-indented-template-token
  "Like parse-string-template-token, but for indented `'' ''` strings: the body
  is already dedented and its literal fragments use Nix indented-string escape
  decoding. Whitespace-only fragments are significant."
  [source tok]
  (let [content (:value tok)
        n (count content)]
    (loop [idx 0
           literal ""
           parts []]
      (cond
        (>= idx n)
        (ast source :string-template nil
             {:parts (cond-> parts
                       (seq literal)
                       (conj {:kind :text :value literal}))
              :span (:span tok)})

        (indented-escape content idx)
        (let [[decoded advance] (indented-escape content idx)]
          (recur (+ idx advance) (str literal decoded) parts))

        (and (< (inc idx) n)
             (= \$ (.charAt content idx))
             (= \{ (.charAt content (inc idx))))
        (let [close (interpolation-close-index content (+ idx 2))]
          (when-not close
            (throw (ex-info "unterminated string interpolation"
                            {:token tok
                             :span (:span tok)})))
          (let [expr (parse-template-expr (template-expr-source content (+ idx 2) close))]
            (recur (inc close)
                   ""
                   (cond-> parts
                     (seq literal)
                     (conj {:kind :text :value literal})

                     true
                     (conj {:kind :expr :expr expr})))))

        :else
        (recur (inc idx) (str literal (.charAt content idx)) parts)))))

(defn- parse-primary
  [source tokens idx]
  (let [tok (nth tokens idx nil)]
    (when-not tok
      (throw (ex-info "missing expression"
                      {:index idx
                       :span [idx idx]})))
    (case (:kind tok)
      :int (int-token-ast source tok idx)
      :float [(ast source :float (:value tok) {:span (:span tok)}) (inc idx)]
      :string [(ast source :string (:value tok) {:span (:span tok)}) (inc idx)]
      :uri [(ast source :string (:value tok) {:span (:span tok)}) (inc idx)]
      :path [(ast source :path (:value tok) {:span (:span tok)}) (inc idx)]
      :string-template [(parse-string-template-token source tok) (inc idx)]
      :indented-template [(parse-indented-template-token source tok) (inc idx)]
      :ident (case (:text tok)
               "true" [(ast source :bool true {:span (:span tok)}) (inc idx)]
               "false" [(ast source :bool false {:span (:span tok)}) (inc idx)]
               "null" [(ast source :null nil {:span (:span tok)}) (inc idx)]
               "let" (parse-let source tokens idx)
               "if" (parse-if source tokens idx)
               "import" (parse-import source tokens idx)
               "assert" (parse-assert source tokens idx)
               "with" (parse-with source tokens idx)
               "rec" (if (= "{" (:text (nth tokens (inc idx) nil)))
                       (parse-attrset source tokens (inc idx) true)
                       [(ast source :var nil {:name (:text tok)
                                              :span (:span tok)})
                        (inc idx)])
               (let [next-tok (nth tokens (inc idx) nil)]
                 (cond
                   (= ":" (:text next-tok))
                   (parse-lambda source tokens idx)

                   ;; `name@{ ... }: body` -- bind the whole arg as `name` and
                   ;; destructure the attr pattern.
                   (and (= "@" (:text next-tok))
                        (= "{" (:text (nth tokens (+ idx 2) nil))))
                   (parse-paramset-lambda source tokens (+ idx 2) (:text tok))

                   :else
                   [(ast source :var nil {:name (:text tok)
                                          :span (:span tok)})
                    (inc idx)])))
      :punct (case (:text tok)
               "[" (parse-list source tokens idx)
               "{" (if (paramset-lambda-start? tokens idx)
                     (parse-paramset-lambda source tokens idx)
                     (parse-attrset source tokens idx))
               "(" (let [[expr i'] (binding [*allow-call* true]
                                    (parse-expr source tokens (inc idx)))
                         close-idx (expect-text tokens i' ")")]
                     [expr close-idx])
               "/" (if-let [path (parse-absolute-path source tokens idx)]
                     path
                     (throw (ex-info "unexpected punctuation"
                                     {:token tok
                                      :span (:span tok)})))
               (throw (ex-info "unexpected punctuation"
                               {:token tok
                                :span (:span tok)})))
      (throw (ex-info "unsupported token kind"
                      {:token tok
                       :span (:span tok)})))))

(defn- call-start-token?
  [source tokens idx]
  (let [tok (nth tokens idx nil)]
    (boolean
     (and tok
          ;; Interpolated strings (`f "a${b}c"`) and indented strings are valid
          ;; call arguments in Nix, same as plain strings.
          (or (contains? #{:int :float :path :string :uri
                           :string-template :indented-template}
                         (:kind tok))
              (and (= :ident (:kind tok))
                   (not (contains? #{"in" "then" "else"} (:text tok))))
              (absolute-path-argument-start? source tokens idx)
              (and (= :punct (:kind tok))
                   (contains? #{"(" "[" "{"} (:text tok))))))))

(defn- parse-attr-segment
  "Parse one attribute segment after `.` or `?`, where `j` is the index of the
  attr token. Handles ident/string/string-template names and a bare `${ expr }`
  dynamic key. Returns [attr seg-end attr-span] where seg-end is the index past
  the segment and attr-span covers the whole segment."
  [source tokens j what]
  (let [attr-token (nth tokens j nil)]
    (cond
      (dynamic-key-token? attr-token)
      (let [[attr seg-end] (parse-dynamic-attr-key source tokens j)
            close-tok (nth tokens (dec seg-end) nil)]
        [attr seg-end [(first (:span attr-token)) (second (:span close-tok))]])

      (contains? #{:ident :string :string-template} (:kind attr-token))
      [(case (:kind attr-token)
         :ident (:text attr-token)
         :string (:value attr-token)
         :string-template {:kind :dynamic-attr-key
                           :expr (parse-string-template-token source attr-token)})
       (inc j)
       (:span attr-token)]

      :else
      (throw (ex-info (str "expected " what)
                      {:token attr-token
                       :span (or (:span attr-token)
                                 (:span (nth tokens (dec j) nil)))})))))

(declare parse-select-default-postfix)

(defn- select-node
  "Build AST for `target . seg1.seg2…segN (or default)?`.

  Nix's ExprSelect carries a multi-segment attrPath: missing *any* segment of
  a continuous path is caught by `or`. Parentheses break the path into
  separate selects (`({}.a).b or 9` errors on `.a`; `{}.a.b or 9` yields 9).
  A one-segment path keeps the historical `:attr` key; longer paths use
  `:attrs` so the evaluator can walk the path as a single select."
  [source target path spans default]
  (let [end-span (if default
                   (second (ast-span default))
                   (second (last spans)))
        span [(first (ast-span target)) end-span]
        base (if (= 1 (count path))
               {:target target
                :attr (first path)
                :attr-span (first spans)
                :span span}
               {:target target
                :attrs (vec path)
                :attr-spans (vec spans)
                :span span})]
    (ast source :select nil
         (cond-> base
           default (assoc :default default)))))

(defn- parse-select-attr-path
  "Parse a continuous attr path starting at the first segment token index
  (caller has already consumed the leading `.`). Returns [path spans end-idx]."
  [source tokens idx]
  (let [[attr0 end0 span0]
        (parse-attr-segment source tokens idx "attr path segment")]
    (loop [path [attr0]
           spans [span0]
           i end0]
      (let [tok (nth tokens i nil)]
        (if (and (= :punct (:kind tok))
                 (= "." (:text tok)))
          (let [[attr seg-end span]
                (parse-attr-segment source tokens (inc i) "attr path segment")]
            (recur (conj path attr) (conj spans span) seg-end))
          [path spans i])))))

(defn- parse-select-default-primary
  "Parse the tight `or` fallback in an attr selection. Nix does not parse this
  as a full expression: calls and infix operators bind outside the select, and
  special forms/lambdas need parentheses."
  [source tokens idx]
  (let [tok (nth tokens idx nil)]
    (when-not tok
      (throw (ex-info "missing select default"
                      {:index idx
                       :span [idx idx]})))
    (case (:kind tok)
      :int (int-token-ast source tok idx)
      :float [(ast source :float (:value tok) {:span (:span tok)}) (inc idx)]
      :string [(ast source :string (:value tok) {:span (:span tok)}) (inc idx)]
      :uri [(ast source :string (:value tok) {:span (:span tok)}) (inc idx)]
      :path [(ast source :path (:value tok) {:span (:span tok)}) (inc idx)]
      :string-template [(parse-string-template-token source tok) (inc idx)]
      :indented-template [(parse-indented-template-token source tok) (inc idx)]
      :ident (case (:text tok)
               "true" [(ast source :bool true {:span (:span tok)}) (inc idx)]
               "false" [(ast source :bool false {:span (:span tok)}) (inc idx)]
               "null" [(ast source :null nil {:span (:span tok)}) (inc idx)]
               "rec" (if (= "{" (:text (nth tokens (inc idx) nil)))
                       (parse-attrset source tokens (inc idx) true)
                       (throw (ex-info "invalid select default"
                                       {:token tok
                                        :span (:span tok)})))
               ("let" "if" "assert" "with" "in" "then" "else")
               (throw (ex-info "invalid select default"
                               {:token tok
                                :span (:span tok)}))
               (let [next-tok (nth tokens (inc idx) nil)]
                 (if (or (= ":" (:text next-tok))
                         (and (= "@" (:text next-tok))
                              (= "{" (:text (nth tokens (+ idx 2) nil)))))
                   (throw (ex-info "invalid select default"
                                   {:token tok
                                    :span (:span tok)}))
                   [(ast source :var nil {:name (:text tok)
                                          :span (:span tok)})
                    (inc idx)])))
      :punct (case (:text tok)
               "[" (parse-list source tokens idx)
               "{" (if (paramset-lambda-start? tokens idx)
                     (throw (ex-info "invalid select default"
                                     {:token tok
                                      :span (:span tok)}))
                     (parse-attrset source tokens idx))
               "(" (let [[expr i'] (binding [*allow-call* true]
                                    (parse-expr source tokens (inc idx)))
                         close-idx (expect-text tokens i' ")")]
                     [expr close-idx])
               "/" (if-let [path (parse-absolute-path source tokens idx)]
                     path
                     (throw (ex-info "invalid select default"
                                     {:token tok
                                      :span (:span tok)})))
               (throw (ex-info "invalid select default"
                               {:token tok
                                :span (:span tok)})))
      (throw (ex-info "unsupported token kind"
                      {:token tok
                       :span (:span tok)})))))

(defn- parse-select-default-postfix
  [source tokens idx]
  (loop [[expr i] (parse-select-default-primary source tokens idx)]
    (let [tok (nth tokens i nil)]
      (cond
        (and (= :punct (:kind tok))
             (= "." (:text tok)))
        ;; Continuous attrPath (same as parse-postfix): `or a.b.c` keeps one
        ;; select so intermediate misses stay on that path.
        (let [[path spans path-end]
              (parse-select-attr-path source tokens (inc i))
              or-tok (nth tokens path-end nil)]
          (if (and (= :ident (:kind or-tok))
                   (= "or" (:text or-tok)))
            (let [[default-expr i']
                  (parse-select-default-postfix source tokens (inc path-end))]
              (recur [(select-node source expr path spans default-expr) i']))
            (recur [(select-node source expr path spans nil) path-end])))

        ;; NB: no `?` here — Nix's select-or default is expr_select, and `?`
        ;; is an operator level above (D6): `a.b or c ? d` = `(a.b or c) ? d`.

        :else
        [expr i]))))

(defn- parse-postfix
  [source tokens idx]
  (loop [[expr i] (parse-primary source tokens idx)]
    (let [tok (nth tokens i nil)]
      (cond
        (and (= :punct (:kind tok))
             (= "." (:text tok)))
        ;; Nix ExprSelect attrPath: continuous `.a.b.c or d` is ONE select
        ;; (or catches any segment miss). A parenthesized intermediate is a
        ;; separate primary, so `({}.a).b or 9` hard-fails on `.a`.
        (let [[path spans path-end]
              (parse-select-attr-path source tokens (inc i))
              or-tok (nth tokens path-end nil)]
          (if (and (= :ident (:kind or-tok))
                   (= "or" (:text or-tok)))
            (let [[default-expr i']
                  (parse-select-default-postfix source tokens (inc path-end))]
              (recur [(select-node source expr path spans default-expr) i']))
            (recur [(select-node source expr path spans nil) path-end])))

        ;; NB: `?` (has-attr) is NOT postfix — it is a precedence-level
        ;; OPERATOR in Nix, looser than application (oracle-confirmed, D6:
        ;; `f { } ? a` is `(f { }) ? a`); it lives in parse-has-attr.

        (and *allow-call*
             (call-start-token? source tokens i))
        (let [_ (guard-operator-operand tokens i "a function argument")
              [arg i'] (binding [*allow-call* false]
                         (parse-postfix source tokens i))]
          (recur [(ast source :call nil {:fn expr
                                         :arg arg
                                         :span (combine-span expr arg)})
                  i']))

        :else
        [expr i]))))

(def ^:private expr-level-keywords
  "Constructs Nix confines to the expression level: they may START an
  expression (and greedily swallow the rest) but may NOT appear as an
  unparenthesized OPERAND of an operator (`1 + let ... in x` is a Nix syntax
  error). Oracle-confirmed on nix-instantiate for all four."
  #{"let" "if" "with" "assert"})

(defn- guard-operator-operand
  "Reject an expr-level keyword at a position where real Nix rejects it:
  operand-after-operator, function-argument, and list-element positions.
  The LEFTMOST/expression-entry position is never guarded: there the keyword
  legally swallows the whole rest of the expression (same parse tree as
  Nix's expr_function > expr_op hierarchy). `pos-text` names the position
  for the error message."
  [tokens idx pos-text]
  (let [tok (nth tokens idx nil)]
    (when (and (= :ident (:kind tok))
               (contains? expr-level-keywords (:text tok)))
      (throw (ex-info (str "syntax error: unexpected `" (:text tok)
                           "` as " pos-text
                           " (Nix rejects this; parenthesize the "
                           (:text tok) " expression)")
                      {:index idx
                       :span (:span tok)})))))

(declare parse-add)

(defn- parse-unary
  [source tokens idx]
  (let [tok (nth tokens idx nil)]
    (cond
      (and (= :punct (:kind tok))
           (= "!" (:text tok)))
      ;; Nix `!` sits at precedence 8, LOOSER than `+ - * / ++ ?` — its
      ;; operand absorbs those (oracle-confirmed, D6: `! { } ? a` is
      ;; `!({ } ? a)` and `! a + b` is `!(a + b)`), so it parses at the
      ;; parse-add level, not parse-unary.
      (let [_ (guard-operator-operand tokens (inc idx) "operand of `!`")
            [expr i'] (parse-add source tokens (inc idx))]
        [(ast source :not nil {:expr expr
                               :span [(first (:span tok))
                                      (second (ast-span expr))]})
         i'])

      (and (= :punct (:kind tok))
           (= "-" (:text tok))
           (= :int (:kind (nth tokens (inc idx) nil)))
           (= min-i64-magnitude-text (:text (nth tokens (inc idx)))))
      ;; `-9223372036854775808` is Long/MIN_VALUE -- its unsigned magnitude
      ;; alone never fits a positive long, so fold sign+magnitude into one
      ;; literal here rather than negating an already-rejected token.
      (let [int-tok (nth tokens (inc idx))]
        [(ast source :int Long/MIN_VALUE
              {:span [(first (:span tok)) (second (:span int-tok))]})
         (+ idx 2)])

      (and (= :punct (:kind tok))
           (= "-" (:text tok)))
      (let [_ (guard-operator-operand tokens (inc idx) "operand of `-`")
            [expr i'] (parse-unary source tokens (inc idx))]
        [(ast source :neg nil {:expr expr
                               :span [(first (:span tok))
                                      (second (ast-span expr))]})
         i'])

      :else
      (parse-postfix source tokens idx))))

(defn- operator?
  [tokens idx ops]
  (let [tok (nth tokens idx nil)]
    (and (= :punct (:kind tok))
         (contains? ops (:text tok)))))

(defn- parse-binary-left
  [source tokens idx subparser ops]
  (loop [[left i] (subparser source tokens idx)]
    (if (operator? tokens i ops)
      (let [op-token (nth tokens i)
            _ (guard-operator-operand tokens (inc i) (str "operand of `" (:text op-token) "`"))
            [right i'] (subparser source tokens (inc i))]
        (recur [(ast source :binary nil {:operator (:text op-token)
                                         :left left
                                         :right right
                                         :span (combine-span left right)})
                i']))
      [left i])))

(defn- has-attr-node
  "Build the AST for `target ? seg1.seg2...segN` (oracle-confirmed, D6: the
  `?` RHS is an ATTRPATH). A single segment stays the plain :has-attr node;
  a longer path desugars to existing ops with Nix's semantics:
  e ? a.b == if e ? a then e.a ? b else false."
  [source target [seg & more] [seg-span & more-spans] end-span]
  (let [span [(first (ast-span target)) end-span]]
    (if (empty? more)
      (ast source :has-attr nil
           {:target target
            :attr seg
            :attr-span seg-span
            :span span})
      (ast source :if nil
           {:condition (ast source :has-attr nil
                            {:target target
                             :attr seg
                             :attr-span seg-span
                             :span span})
            :then (has-attr-node source
                                 (ast source :select nil
                                      {:target target
                                       :attr seg
                                       :attr-span seg-span
                                       :span span})
                                 more more-spans end-span)
            :else (ast source :bool false {:span span})
            :span span}))))

(defn- parse-has-attr
  "`?` (has-attr) as a Nix precedence level: looser than unary minus,
  selection, and application; RHS is an attrpath. Chains (`e ? a ? b`) are
  legal in real Nix 2.34 and associate left (oracle-confirmed, D6)."
  [source tokens idx]
  (loop [[expr i] (parse-unary source tokens idx)]
    (let [tok (nth tokens i nil)]
      (if (and (= :punct (:kind tok))
               (= "?" (:text tok)))
        (let [[path i' spans] (parse-attr-path source tokens (inc i))
              end-span (second (last spans))]
          (recur [(has-attr-node source expr path spans end-span) i']))
        [expr i]))))

(defn- parse-mul
  [source tokens idx]
  (parse-binary-left source tokens idx parse-has-attr #{"*" "/"}))

(defn- parse-add
  [source tokens idx]
  (parse-binary-left source tokens idx parse-mul #{"+" "-"}))

(defn- parse-merge
  [source tokens idx]
  (parse-binary-left source tokens idx parse-add #{"//" "++"}))

(defn- parse-binary-nonassoc
  "One optional binary op from `ops`: real Nix declares == != and the
  relational operators %nonassoc, so a chain (`1 == 1 == true`,
  `1 < 2 < 3`) is a SYNTAX error, not a left fold (oracle-confirmed, D6)."
  [source tokens idx subparser ops]
  (let [[left i] (subparser source tokens idx)]
    (if (operator? tokens i ops)
      (let [op-token (nth tokens i)
            _ (guard-operator-operand tokens (inc i) (str "operand of `" (:text op-token) "`"))
            [right i'] (subparser source tokens (inc i))]
        (when (operator? tokens i' ops)
          (throw (ex-info (str "syntax error: `" (:text (nth tokens i'))
                               "` is non-associative (Nix rejects the chain; "
                               "parenthesize)")
                          {:index i'
                           :span (:span (nth tokens i'))})))
        [(ast source :binary nil {:operator (:text op-token)
                                  :left left
                                  :right right
                                  :span (combine-span left right)})
         i'])
      [left i])))

(defn- parse-rel
  [source tokens idx]
  (parse-binary-nonassoc source tokens idx parse-merge #{"<" ">" "<=" ">="}))

(defn- parse-eq
  ;; == and != bind LOOSER than the relational operators in Nix
  ;; (oracle-confirmed, D6: `true == 1 < 2` is `true == (1 < 2)`).
  [source tokens idx]
  (parse-binary-nonassoc source tokens idx parse-rel #{"==" "!="}))

(defn- parse-and
  [source tokens idx]
  (parse-binary-left source tokens idx parse-eq #{"&&"}))

(defn- parse-or
  [source tokens idx]
  (parse-binary-left source tokens idx parse-and #{"||"}))

(defn- parse-impl
  [source tokens idx]
  ;; `->` (logical implication) is the lowest-precedence operator and is
  ;; right-associative: a -> b -> c parses as a -> (b -> c).
  (let [[left i] (parse-or source tokens idx)]
    (if (operator? tokens i #{"->"})
      (let [op-token (nth tokens i)
            _ (guard-operator-operand tokens (inc i) "operand of `->`")
            [right i'] (parse-impl source tokens (inc i))]
        [(ast source :binary nil {:operator (:text op-token)
                                  :left left
                                  :right right
                                  :span (combine-span left right)})
         i'])
      [left i])))

(defn- parse-expr
  [source tokens idx]
  (parse-impl source tokens idx))

(defn- parse-root
  [source]
  (let [tokens (tokenize source)
        [parsed idx] (parse-expr source tokens 0)]
    (when-not (= idx (count tokens))
      (throw (ex-info "trailing tokens" {:index idx
                                         :remaining (subvec (vec tokens) idx)
                                         :span (:span (nth tokens idx))})))
    parsed))

(defn- unsupported-ledger
  [source throwable]
  (let [data (ex-data throwable)
        syntax-span (or (:span data)
                        (get-in data [:token :span])
                        [0 (count source)])]
    [{:kind :unsupported-syntax
      :reason :unsupported-syntax
      :span syntax-span
      :message (.getMessage throwable)
      :data data}]))

(defn- parse-source*
  [s source-hash cache-key]
  (try
    {:status :ok
     :source s
     :source-hash source-hash
     :cache-key cache-key
     :ast (binding [*source-hash* source-hash] (parse-root s))}
    (catch StackOverflowError _
      ;; honest label: parser recursion depth, not a syntax problem
      {:status :failed
       :reason :parse-stack-overflow
       :source s
       :source-hash source-hash
       :cache-key cache-key
       :span [0 (count s)]
       :error {:phase :parse
               :class :unsupported-expression
               :reason :parse-stack-overflow
               :evidence {:resource :parser-stack}}})
    (catch Throwable t
      (let [ledger (unsupported-ledger s t)
            stable-data (select-keys (or (ex-data t) {})
                                     [:reason :position :span :token :construct])]
        {:status :failed
         :reason :unsupported-syntax
         :source s
         :source-hash source-hash
         :cache-key cache-key
         :span (:span (first ledger))
         :unsupported-syntax ledger
         :error {:phase :parse
                 :class :syntax-error
                 :reason :unsupported-syntax
                 :data stable-data}}))))

(defn parse-source
  "Parse the current pnix-clj expression slice. Unsupported syntax is reported
  as structured Failed data instead of guessed semantics."
  [source]
  (let [s (normalized-source source)
        source-hash (hash/sha256 s)
        cache-key {:schema :pnix-clj.parse-cache-key.v0
                   :source-hash source-hash}]
    (if-let [cached (get @parse-cache source-hash)]
      (do
        (swap! parse-cache-stats* update :hits inc)
        cached)
      (let [parsed (parse-source* s source-hash cache-key)]
        (swap! parse-cache-stats* update :misses inc)
        (swap! parse-cache assoc source-hash parsed)
        parsed))))
