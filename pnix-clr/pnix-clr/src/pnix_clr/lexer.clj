(ns pnix-clr.lexer
  (:require [pnix-clr.outcome :as outcome]))

(def ^:private keyword-kinds
  {"let" :let
   "in" :in
   "rec" :rec
   "import" :import
   "true" :true
   "false" :false
   "null" :null
   "if" :if
   "then" :then
   "else" :else
   "with" :with
   "assert" :assert
   "inherit" :inherit})

(def ^:private single-kinds
  {\{ :lbrace \} :rbrace
   \[ :lbracket \] :rbracket
   \( :lparen \) :rparen
   \; :semicolon \: :colon
   \= :assign \. :dot
   \+ :plus \- :minus
   \* :multiply \/ :divide
   \? :question \, :comma \@ :at})

(defn- whitespace?
  [ch]
  (contains? #{\space \tab \newline \return} ch))

(defn- ascii-letter?
  [ch]
  (let [n (int ch)]
    (or (<= (int \a) n (int \z))
        (<= (int \A) n (int \Z)))))

(defn- ascii-digit?
  [ch]
  (let [n (int ch)]
    (<= (int \0) n (int \9))))

(defn- ident-start?
  [ch]
  (or (ascii-letter? ch) (= ch \_)))

(defn- ident-part?
  [ch]
  (or (ident-start? ch) (ascii-digit? ch) (= ch \-) (= ch \')))

(defn- path-start?
  [source index]
  (let [ch (nth source index)
        next-ch (when (< (inc index) (count source))
                  (nth source (inc index)))]
    (or (and (= ch \/)
             next-ch
             (not (whitespace? next-ch))
             (not (contains? #{\( \) \[ \] \{ \} \;} next-ch)))
        (and (= ch \.) (contains? #{\. \/} next-ch)))))

(defn- uri-scheme-char?
  [ch]
  (or (ascii-letter? ch)
      (ascii-digit? ch)
      (contains? #{\+ \- \.} ch)))

(defn- uri-body-char?
  [ch]
  (or (ascii-letter? ch)
      (ascii-digit? ch)
      (contains? #{\% \/ \? \: \@ \& \= \+ \$ \, \- \_ \. \! \~ \* \'} ch)))

(defn- scan-uri-end
  "Nix 2.34.7 URI atom: alpha scheme chars, `:`, nonempty body. Nil if not URI."
  [source start]
  (let [size (count source)]
    (when (and (< start size) (ascii-letter? (nth source start)))
      (let [colon (loop [i (inc start)]
                    (if (and (< i size) (uri-scheme-char? (nth source i)))
                      (recur (inc i))
                      i))]
        (when (and (< colon size) (= \: (nth source colon)))
          (let [body-start (inc colon)
                end (loop [i body-start]
                      (if (and (< i size) (uri-body-char? (nth source i)))
                        (recur (inc i))
                        i))]
            (when (> end body-start) end)))))))

(defn- path-char?
  [ch]
  (not (or (whitespace? ch)
           (contains? #{\( \) \[ \] \{ \} \;} ch))))

(defn- scan-while
  [source start predicate]
  (loop [index start]
    (if (and (< index (count source)) (predicate (nth source index)))
      (recur (inc index))
      index)))

(defn- syntax-error!
  [reason evidence]
  (outcome/fail! :parse :syntax-error
                 (assoc evidence :reason reason)))

(def ^:private double-min-normal 2.2250738585072014E-308)

(defn- parse-nix-float
  "Parse float token; reject non-zero mantissa subnormal/overflow (Nix)."
  [text offset]
  (let [mantissa (first (clojure.string/split text #"[eE]" 2))
        nonzero-mantissa? (boolean (re-find #"[1-9]" mantissa))
        value (try
                (System.Double/Parse
                 text
                 System.Globalization.CultureInfo/InvariantCulture)
                (catch System.Exception _
                  (syntax-error! "invalid-float-literal"
                                 {:offset offset :literal text})))]
    (when (and nonzero-mantissa?
               (or (not (Double/IsFinite value))
                   (< (Math/Abs value) double-min-normal)))
      (syntax-error! "invalid-float-literal"
                     {:offset offset :literal text}))
    value))

(def min-i64-magnitude-text
  ;; abs(Int64.MinValue) as unsigned text -- one more than Int64.MaxValue, so
  ;; it can never parse as a positive Int64. Only valid when a unary `-`
  ;; immediately negates it (see parse-unary's fold in parser.clj); a bare
  ;; occurrence is still rejected there.
  "9223372036854775808")

(defn- parse-nix-int
  [text offset]
  (try
    (System.Int64/Parse text)
    (catch System.OverflowException _
      (if (= text min-i64-magnitude-text)
        System.Int64/MinValue
        (syntax-error! "integer-literal-out-of-range"
                       {:offset offset :literal text})))))

(declare tokenize scan-string scan-indented)

(defn- scan-interpolation-end
  "Return index after the closing `}` of a `${...}` form, brace-balanced."
  [source open-brace-index]
  (loop [index (inc open-brace-index)
         depth 1]
    (when (>= index (count source))
      (syntax-error! "unterminated-string-interpolation" {:offset open-brace-index}))
    (let [ch (nth source index)
          pair (when (< (inc index) (count source))
                 (subs source index (+ index 2)))]
      (cond
        (= ch \{) (recur (inc index) (inc depth))
        (= ch \}) (if (= 1 depth)
                    (inc index)
                    (recur (inc index) (dec depth)))
        (= ch \") (let [[end _] (scan-string source index)]
                    (recur end depth))
        (= pair "''") (let [[end _] (scan-indented source index)]
                        (recur end depth))
        :else (recur (inc index) depth)))))

(defn- scan-string
  "Scan a double-quoted string. Returns [end-index token-payload] where
  payload is either a plain string or {:parts [...]} for interpolations."
  [source start]
  (loop [index (inc start)
         parts []
         lit ""]
    (when (>= index (count source))
      (syntax-error! "unterminated-string" {:offset start}))
    (let [ch (nth source index)]
      (cond
        (= ch \")
        (let [end (inc index)
              parts' (if (seq lit) (conj parts {:kind :lit :value lit}) parts)]
          (cond
            ;; Bare `""` is a plain empty string, not an interp skeleton.
            (empty? parts')
            [end ""]

            (and (= 1 (count parts'))
                 (= :lit (:kind (first parts'))))
            [end (:value (first parts'))]

            :else
            [end {:parts parts'}]))

        (= ch \\)
        (let [escape-index (inc index)]
          (when (>= escape-index (count source))
            (syntax-error! "unterminated-string-escape" {:offset index}))
          (let [escaped (nth source escape-index)
                value (get {\" \" \\ \\
                            \n \newline \r \return \t \tab
                            \$ \$}
                           escaped)]
            (when (nil? value)
              (syntax-error! "unsupported-string-escape"
                             {:offset index :escape (str escaped)}))
            (recur (+ index 2) parts (str lit value))))

        (and (= ch \$)
             (< (inc index) (count source))
             (= \{ (nth source (inc index))))
        (let [expr-start (+ index 2)
              after-close (scan-interpolation-end source (inc index))
              expr-source (subs source expr-start (dec after-close))
              expr-tokens (vec (butlast (tokenize expr-source)))
              parts' (cond-> parts
                       (seq lit) (conj {:kind :lit :value lit})
                       true (conj {:kind :interp :tokens expr-tokens}))]
          (recur after-close parts' ""))

        :else
        (recur (inc index) parts (str lit ch))))))

(defn- dedent-indented-string
  "Nix `'' ''`: drop leading newline; strip common leading SPACE indent
  (tabs are not indentation)."
  [body]
  (let [body (if (clojure.string/starts-with? body "\n")
               (subs body 1)
               body)
        lines (clojure.string/split body #"\n" -1)
        indents (->> lines
                     (remove clojure.string/blank?)
                     (map (fn [line]
                            (count (take-while #(= % \space) line)))))
        min-indent (if (seq indents) (apply min indents) 0)
        strip (fn [line]
                (subs line (min min-indent (count line))))]
    (clojure.string/join "\n" (map strip lines))))

(defn- decode-indented-body
  "Decode indented-string body after dedent: ''$ '' ' ''\\n and ${interp}."
  [body]
  (let [n (count body)]
    (loop [index 0
           parts []
           lit ""]
      (if (>= index n)
        (let [parts' (if (seq lit) (conj parts {:kind :lit :value lit}) parts)]
          (cond
            (empty? parts') ""
            (and (= 1 (count parts')) (= :lit (:kind (first parts'))))
            (:value (first parts'))
            :else {:parts parts'}))
        (let [ch (nth body index)
              ch2 (when (< (inc index) n) (nth body (inc index)))
              ch3 (when (< (+ index 2) n) (nth body (+ index 2)))]
          (cond
            (and (= ch \') (= ch2 \') (= ch3 \$))
            (recur (+ index 3) parts (str lit "$"))

            (and (= ch \') (= ch2 \') (= ch3 \'))
            (recur (+ index 3) parts (str lit "''"))

            (and (= ch \') (= ch2 \') (= ch3 \\))
            (let [esc (when (< (+ index 3) n) (nth body (+ index 3)))]
              (if esc
                (let [value (get {\n \newline \r \return \t \tab} esc (str esc))]
                  (recur (+ index 4) parts (str lit value)))
                (recur (+ index 3) parts (str lit "\\"))))

            (and (= ch \$) (= ch2 \{))
            (let [after (scan-interpolation-end body (inc index))
                  expr-source (subs body (+ index 2) (dec after))
                  expr-tokens (vec (butlast (tokenize expr-source)))
                  parts' (cond-> parts
                           (seq lit) (conj {:kind :lit :value lit})
                           true (conj {:kind :interp :tokens expr-tokens}))]
              (recur after parts' ""))

            :else
            (recur (inc index) parts (str lit ch))))))))

(defn- scan-indented
  "Scan `''...''` indented string. Returns [end-index payload] like scan-string."
  [source start]
  (let [size (count source)]
    (loop [index (+ start 2)]
      (when (>= (inc index) size)
        (syntax-error! "unterminated-indented-string" {:offset start}))
      (let [ch (nth source index)
            ch2 (nth source (inc index))]
        (cond
          (and (= ch \') (= ch2 \'))
          (let [ch3 (when (< (+ index 2) size) (nth source (+ index 2)))]
            (case ch3
              \$ (recur (+ index 3))
              \' (recur (+ index 3))
              \\ (recur (+ index (if (< (+ index 3) size) 4 3)))
              ;; closing ''
              (let [end (+ index 2)
                    body (subs source (+ start 2) index)
                    dedented (dedent-indented-string body)
                    payload (decode-indented-body dedented)]
                [end payload])))

          (and (= ch \$) (= ch2 \{))
          (recur (scan-interpolation-end source (inc index)))

          :else
          (recur (inc index)))))))

(defn tokenize
  [source]
  (let [source (str source)
        size (count source)]
    (loop [index 0
           tokens []]
      (if (>= index size)
        (conj tokens {:kind :eof :text "" :offset index})
        (let [ch (nth source index)
              pair (when (< (inc index) size)
                     (subs source index (+ index 2)))]
          (cond
            (whitespace? ch)
            (recur (inc index) tokens)

            (= ch \#)
            (let [end (scan-while source index #(not= % \newline))]
              (recur end tokens))

            (contains? {"&&" :and "||" :or "==" :eq "!=" :neq
                        "<=" :le ">=" :ge "//" :update "++" :concat} pair)
            (recur (+ index 2)
                   (conj tokens {:kind (get {"&&" :and "||" :or
                                             "==" :eq "!=" :neq
                                             "<=" :le ">=" :ge
                                             "//" :update "++" :concat} pair)
                                 :text pair :offset index}))

            ;; Ellipsis `...` (pattern formals) before single `.`.
            (and (< (+ index 2) size)
                 (= \. ch)
                 (= \. (nth source (inc index)))
                 (= \. (nth source (+ index 2))))
            (recur (+ index 3)
                   (conj tokens {:kind :ellipsis :text "..." :offset index}))

            (= ch \<)
            (recur (inc index)
                   (conj tokens {:kind :lt :text "<" :offset index}))

            (= ch \>)
            (recur (inc index)
                   (conj tokens {:kind :gt :text ">" :offset index}))

            (= ch \!)
            (recur (inc index)
                   (conj tokens {:kind :not :text "!" :offset index}))

            (= ch \")
            (let [[end value] (scan-string source index)]
              (if (string? value)
                (recur end
                       (conj tokens {:kind :string
                                     :text (subs source index end)
                                     :value value
                                     :offset index}))
                (recur end
                       (conj tokens {:kind :string-interp
                                     :text (subs source index end)
                                     :parts (:parts value)
                                     :offset index}))))

            ;; Indented string `'' ... ''` (Nix).
            (and (= ch \')
                 (< (inc index) size)
                 (= \' (nth source (inc index))))
            (let [[end value] (scan-indented source index)]
              (if (string? value)
                (recur end
                       (conj tokens {:kind :string
                                     :text (subs source index end)
                                     :value value
                                     :offset index}))
                (recur end
                       (conj tokens {:kind :string-interp
                                     :text (subs source index end)
                                     :parts (:parts value)
                                     :offset index}))))

            ;; Nix number munch (flex D4):
            ;; float = ([1-9][0-9]*\.[0-9]* | 0?\.[0-9]+)([Ee][+-]?[0-9]+)?
            ;;       | [1-9][0-9]*\.([Ee][+-]?[0-9]+)?   (trailing-dot forms)
            ;; int   = [0-9]+
            ;; so `00.5` is int `00` then float `.5`; `1e3` is int `1` then ident.
            (or (ascii-digit? ch)
                (and (= ch \.)
                     (< (inc index) size)
                     (ascii-digit? (nth source (inc index)))))
            (let [scan-exp
                  (fn [from]
                    (if (and (< from size)
                             (contains? #{\e \E} (nth source from))
                             (let [i (inc from)]
                               (and (< i size)
                                    (or (ascii-digit? (nth source i))
                                        (and (contains? #{\+ \-} (nth source i))
                                             (< (inc i) size)
                                             (ascii-digit?
                                              (nth source (inc i))))))))
                      (let [i (inc from)
                            i (if (contains? #{\+ \-} (nth source i)) (inc i) i)]
                        (scan-while source i ascii-digit?))
                      from))
                  ;; Prefer a true float form before a bare integer.
                  float-match
                  (cond
                    ;; `.5` / `.5e2`
                    (and (= ch \.)
                         (< (inc index) size)
                         (ascii-digit? (nth source (inc index))))
                    (let [frac-end (scan-while source (inc index) ascii-digit?)]
                      (scan-exp frac-end))

                    ;; `1.2` / `1.` / `1.2e3` / `12.e-1` — must start [1-9]
                    ;; The digit run may reach end-of-input (`20`), so the
                    ;; position of the would-be `.` must be bounds-checked
                    ;; before it is read.
                    (and (ascii-digit? ch)
                         (not= \0 ch)
                         (let [int-end (scan-while source index ascii-digit?)]
                           (and (< int-end size)
                                (= \. (nth source int-end)))))
                    (let [int-end (scan-while source index ascii-digit?)
                          ;; optional fraction digits
                          frac-end (if (and (< (inc int-end) size)
                                            (ascii-digit?
                                             (nth source (inc int-end))))
                                     (scan-while source (inc int-end)
                                                 ascii-digit?)
                                     (inc int-end))]
                      (scan-exp frac-end))

                    ;; `0.5` / `0.5e2` only (single leading zero + fraction)
                    (and (= ch \0)
                         (< (inc index) size)
                         (= \. (nth source (inc index)))
                         (< (+ index 2) size)
                         (ascii-digit? (nth source (+ index 2))))
                    (let [frac-end (scan-while source (+ index 2) ascii-digit?)]
                      (scan-exp frac-end))

                    :else nil)]
              (if float-match
                (let [text (subs source index float-match)
                      value (parse-nix-float text index)]
                  (recur float-match
                         (conj tokens {:kind :float :text text
                                       :value value :offset index})))
                ;; Integer: maximal [0-9]+ (so `00` is one int token).
                (let [int-end (scan-while source index ascii-digit?)
                      text (subs source index int-end)
                      value (parse-nix-int text index)]
                  (recur int-end
                         (conj tokens {:kind :int :text text
                                       :value value :offset index})))))

            (path-start? source index)
            (let [end (scan-while source index path-char?)
                  text (subs source index end)]
              (recur end (conj tokens {:kind :path :text text :offset index})))

            ;; URI before ident so `http://…` / `a:b` win over `a` `:` `b`.
            (scan-uri-end source index)
            (let [uri-end (scan-uri-end source index)
                  text (subs source index uri-end)]
              (recur uri-end
                     (conj tokens {:kind :uri :text text
                                   :value text :offset index})))

            (ident-start? ch)
            (let [end (scan-while source index ident-part?)
                  text (subs source index end)]
              (recur end
                     (conj tokens {:kind (get keyword-kinds text :ident)
                                   :text text :offset index})))

            (contains? single-kinds ch)
            (recur (inc index)
                   (conj tokens {:kind (get single-kinds ch)
                                 :text (str ch) :offset index}))

            :else
            (syntax-error! "unsupported-syntax"
                           {:offset index :token (str ch)})))))))
