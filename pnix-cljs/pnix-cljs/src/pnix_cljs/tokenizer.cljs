(ns pnix-cljs.tokenizer)

(def keywords
  {"let" :let
   "in" :in
   "if" :if
   "then" :then
   "else" :else
   "true" :true
   "false" :false
   "null" :null
   "rec" :rec
   "inherit" :inherit
   "import" :import
   "scopedImport" :scopedImport
   "assert" :assert
   "with" :with})

(def two-character-tokens
  {"==" :equal-equal
   "!=" :not-equal
   "<=" :less-equal
   ">=" :greater-equal
   "++" :concat
   "//" :update
   "&&" :and
   "||" :or
   "${" :dynamic-start})

(def one-character-tokens
  {"(" :left-paren
   ")" :right-paren
   "[" :left-bracket
   "]" :right-bracket
   "{" :left-brace
   "}" :right-brace
   ";" :semicolon
   "=" :equal
   ":" :colon
   "." :dot
   "," :comma
   "+" :plus
   "-" :minus
   "*" :star
   "/" :slash
   "!" :bang
   "?" :question
   "@" :at
   "<" :less
   ">" :greater})

(defn parse-failure! [detail-class offset evidence]
  (throw (ex-info detail-class
                  {"pnix_error" true
                   "phase" "parse"
                   "class" "syntax-error"
                   "evidence" (assoc evidence
                                     "detail_class" detail-class
                                     "offset" offset)})))

(defn numeric-literal-failure! [detail-class offset evidence]
  (throw (ex-info detail-class
                  {"pnix_error" true
                   "phase" "eval"
                   "class" detail-class
                   "evidence" (assoc evidence
                                     "detail_class" detail-class
                                     "offset" offset)})))

(defn character [source index]
  (.charAt source index))

(defn whitespace? [value]
  (boolean (re-matches #"\s" value)))

(defn digit? [value]
  (and (not= "" value)
       (let [code (.charCodeAt value 0)]
         (<= 48 code 57))))

(def float-pattern
  #"^(?:[0-9]+\.[0-9]*(?:[eE][+-]?[0-9]+)?|\.[0-9]+(?:[eE][+-]?[0-9]+)?)")

(defn read-float [source start]
  (let [matched (re-find float-pattern (subs source start))]
    (when matched
      (let [text (if (vector? matched) (first matched) matched)
            dot (.indexOf text ".")
            integer-part (subs text 0 dot)
            value (js/Number text)
            nonzero-mantissa?
            (boolean (re-find #"[1-9]" (first (.split text #"[eE]"))))]
        (when (and (not= integer-part "")
                   (> (count integer-part) 1)
                   (= "0" (subs integer-part 0 1)))
          (numeric-literal-failure! "invalid-float-literal"
                                    start
                                    {"lexeme" text}))
        (when (or (not (js/Number.isFinite value))
                  (and (zero? value) nonzero-mantissa?)
                  (and (not (zero? value))
                       (< (js/Math.abs value)
                          2.2250738585072014e-308)))
          (parse-failure! "invalid-float-literal" start {"lexeme" text}))
        {:text text :value value}))))

(defn identifier-start? [value]
  (boolean (re-matches #"[A-Za-z_]" value)))

(defn identifier-part? [value]
  (boolean (re-matches #"[A-Za-z0-9_'-]" value)))

(defn read-while [source start predicate]
  (loop [index start]
    (if (and (< index (count source))
             (predicate (character source index)))
      (recur (inc index))
      index)))

(declare starts-with-at? read-interpolation-source append-text-part)

(defn read-string-token [source start]
  (loop [index (inc start)
         value ""
         parts []]
    (when (>= index (count source))
      (parse-failure! "unterminated-string" start {}))
    (let [current (character source index)]
      (cond
        (= current "\"")
        [(if (empty? parts)
           {:kind :string :value value :offset start}
           {:kind :interpolated-string
            :parts (append-text-part parts value)
            :offset start})
         (inc index)]

        (= current "\\")
        (let [next-index (inc index)]
          (when (>= next-index (count source))
            (parse-failure! "unterminated-string-escape" index {}))
          (let [escaped (character source next-index)
                decoded (get {"n" "\n" "r" "\r" "t" "\t"
                              "\"" "\"" "\\" "\\"}
                             escaped
                             escaped)]
            (recur (+ index 2) (str value decoded) parts)))

        (starts-with-at? source index "${")
        (let [[expression-source end]
              (read-interpolation-source source (+ index 2))]
          (recur end
                 ""
                 (conj (append-text-part parts value)
                       {:kind :expression
                        :source expression-source})))

        :else
        (recur (inc index) (str value current) parts)))))

(defn starts-with-at? [source index value]
  (and (<= (+ index (count value)) (count source))
       (= value (subs source index (+ index (count value))))))

(defn read-interpolation-source [source start]
  (loop [index start
         depth 1
         quoted? false
         escaped? false]
    (when (>= index (count source))
      (parse-failure! "unterminated-string-interpolation" start {}))
    (let [current (character source index)]
      (cond
        quoted?
        (cond
          escaped? (recur (inc index) depth true false)
          (= current "\\") (recur (inc index) depth true true)
          (= current "\"") (recur (inc index) depth false false)
          :else (recur (inc index) depth true false))

        (= current "\"")
        (recur (inc index) depth true false)

        (= current "{")
        (recur (inc index) (inc depth) false false)

        (= current "}")
        (if (= depth 1)
          [(subs source start index) (inc index)]
          (recur (inc index) (dec depth) false false))

        :else
        (recur (inc index) depth false false)))))

(defn append-text-part [parts text]
  (if (= text "")
    parts
    (conj parts {:kind :text :value text})))

(defn read-indented-string-token [source start]
  (loop [index (+ start 2)
         text ""
         parts []]
    (when (>= index (count source))
      (parse-failure! "unterminated-indented-string" start {}))
    (cond
      (starts-with-at? source index "''${")
      (recur (+ index 4) (str text "${") parts)

      (starts-with-at? source index "'''")
      (recur (+ index 3) (str text "''") parts)

      (starts-with-at? source index "''$")
      (recur (+ index 3) (str text "$") parts)

      (starts-with-at? source index "''\\n")
      (recur (+ index 4) (str text "\n") parts)

      (starts-with-at? source index "''\\r")
      (recur (+ index 4) (str text "\r") parts)

      (starts-with-at? source index "''\\t")
      (recur (+ index 4) (str text "\t") parts)

      (starts-with-at? source index "''")
      [{:kind :indented-string
        :parts (append-text-part parts text)
        :offset start}
       (+ index 2)]

      (starts-with-at? source index "${")
      (let [[expression-source end]
            (read-interpolation-source source (+ index 2))]
        (recur end
               ""
               (conj (append-text-part parts text)
                     {:kind :expression
                      :source expression-source})))

      :else
      (recur (inc index)
             (str text (character source index))
             parts))))

(def uri-prefix-pattern #"^[A-Za-z][A-Za-z0-9+.-]*:")

(defn uri-character? [value]
  (boolean (re-matches #"[A-Za-z0-9%/?:@&=+$,_.!~*'-]" value)))

(defn read-uri [source start]
  (when-let [prefix (re-find uri-prefix-pattern (subs source start))]
    (let [body-start (+ start (count prefix))]
      (when (and (< body-start (count source))
                 (uri-character? (character source body-start)))
        (let [end (read-while source body-start uri-character?)]
          (subs source start end))))))

(defn relative-path-start? [source index]
  (or (and (<= (+ index 2) (count source))
           (= "./" (subs source index (+ index 2))))
      (and (<= (+ index 3) (count source))
           (= "../" (subs source index (+ index 3))))))

;; A bare `/` starts an absolute path literal (e.g. `import /a/b.px`,
;; `builtins.isPath /tmp/x`) UNLESS it immediately follows a number, where
;; it's division (`1/0`) -- same disambiguation already used for pnix-clr
;; and pnix-hy's lexers.
(defn absolute-path-start? [source index tokens]
  (and (= "/" (character source index))
       ;; `//` is the attrset-merge operator, never a path literal.
       (not= "/" (character source (inc index)))
       (not (contains? #{:integer :float} (:kind (peek tokens))))))

(defn path-character? [value]
  (and (not (whitespace? value))
       (not (contains? #{";" "(" ")" "[" "]" "{" "}"} value))))

(defn skip-block-comment [source start]
  (loop [index (+ start 2)]
    (when (>= (inc index) (count source))
      (parse-failure! "unterminated-block-comment" start {}))
    (if (= "*/" (subs source index (+ index 2)))
      (+ index 2)
      (recur (inc index)))))

(defn tokenize [source]
  (loop [index 0
         tokens []]
    (if (>= index (count source))
      (conj tokens {:kind :eof :offset index})
      (let [current (character source index)
            next-two (when (< (inc index) (count source))
                       (subs source index (+ index 2)))
            next-three (when (< (+ index 2) (count source))
                         (subs source index (+ index 3)))]
        (cond
          (whitespace? current)
          (recur (inc index) tokens)

          (= current "#")
          (let [end (read-while source index #(not= "\n" %))]
            (recur end tokens))

          (= next-two "/*")
          (recur (skip-block-comment source index) tokens)

          (= current "\"")
          (let [[token end] (read-string-token source index)]
            (recur end (conj tokens token)))

          (= next-two "''")
          (let [[token end] (read-indented-string-token source index)]
            (recur end (conj tokens token)))

          (= next-three "...")
          (recur (+ index 3)
                 (conj tokens {:kind :ellipsis
                               :value "..."
                               :offset index}))

          (or (digit? current)
              (and (= current ".")
                   (digit? (character source (inc index)))))
          (if-let [{:keys [text value]} (read-float source index)]
            (recur (+ index (count text))
                   (conj tokens {:kind :float
                                 :value value
                                 :offset index}))
            (let [end (read-while source index digit?)
                  text (subs source index end)
                  value (js/BigInt text)]
              (when (#{"e" "E"} (character source end))
                (parse-failure! "invalid-float-literal" index
                                {"lexeme" (subs source index (inc end))}))
              (recur end (conj tokens {:kind :integer
                                       :value value
                                       :offset index}))))

          (or (relative-path-start? source index)
              (absolute-path-start? source index tokens))
          (let [end (read-while source index path-character?)
                value (subs source index end)]
            (recur end (conj tokens {:kind :path
                                     :value value
                                     :offset index})))

          (read-uri source index)
          (let [value (read-uri source index)]
            (recur (+ index (count value))
                   (conj tokens {:kind :uri
                                 :value value
                                 :offset index})))

          (identifier-start? current)
          (let [end (read-while source index identifier-part?)
                text (subs source index end)
                kind (get keywords text :identifier)]
            (recur end (conj tokens {:kind kind
                                     :value text
                                     :offset index})))

          (contains? two-character-tokens next-two)
          (recur (+ index 2)
                 (conj tokens {:kind (get two-character-tokens next-two)
                               :value next-two
                               :offset index}))

          (contains? one-character-tokens current)
          (recur (inc index)
                 (conj tokens {:kind (get one-character-tokens current)
                               :value current
                               :offset index}))

          :else
          (parse-failure! "unexpected-character"
                          index
                          {"character" current}))))))
