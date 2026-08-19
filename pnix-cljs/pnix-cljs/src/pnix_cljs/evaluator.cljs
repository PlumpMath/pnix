(ns pnix-cljs.evaluator
  (:require [pnix-cljs.parser :as parser]
            [clojure.string :as str]
            goog.crypt.Md5
            goog.crypt.Sha1
            goog.crypt.Sha256
            goog.crypt.Sha512))

(declare evaluate-expression evaluate-tail equal-values equal-values*
         equal-values-in-container ordered-less checked-integer
         evaluation-failure! force-cell integer-value? json-parse-value
         to-json-value)
(declare materialize)

(defrecord ClosureValue [parameter body environment])
(defrecord BuiltinValue [operation arguments])
(defrecord AttrsetValue [fields])
(defrecord ByteStringValue [bytes])
(defrecord Cell [expression environment state])

(def module-context-key ::module-context)
(def force-dependency-key ::force-dependency)
(def ^:dynamic *forcing-cells* false)
(def ^:dynamic *argument-cell-cache* nil)
(def ^:dynamic *force-depth* 0)
(def force-stack-threshold 256)

(def min-i64 (js/BigInt "-9223372036854775808"))
(def max-i64 (js/BigInt "9223372036854775807"))

(def utf8-encoder (js/TextEncoder.))

(defn string-value? [value]
  (or (string? value)
      (instance? ByteStringValue value)))

(defn string-bytes [value]
  (if (instance? ByteStringValue value)
    (:bytes value)
    (.encode utf8-encoder value)))

(defn equal-bytes? [left right]
  (and (= (.-length left) (.-length right))
       (loop [index 0]
         (if (= index (.-length left))
           true
           (and (= (aget left index) (aget right index))
                (recur (inc index)))))))

(defn compare-bytes [left right]
  (loop [index 0]
    (cond
      (= index (.-length left))
      (if (= index (.-length right)) 0 -1)

      (= index (.-length right)) 1
      (< (aget left index) (aget right index)) -1
      (> (aget left index) (aget right index)) 1
      :else (recur (inc index)))))

(defn compare-strings-bytes [left right]
  (compare-bytes (string-bytes left) (string-bytes right)))

(defn sorted-field-names [fields]
  (sort compare-strings-bytes (keys fields)))

(defn decode-byte-string [bytes]
  (try
    (.decode (js/TextDecoder. "utf-8" #js {:fatal true}) bytes)
    (catch :default _
      (->ByteStringValue bytes))))

(defn concatenate-byte-arrays [arrays]
  (let [total (reduce + 0 (map #(.-length %) arrays))
        result (js/Uint8Array. total)]
    (loop [remaining arrays
           offset 0]
      (if (empty? remaining)
        result
        (let [bytes (first remaining)]
          (.set result bytes offset)
          (recur (rest remaining) (+ offset (.-length bytes))))))))

(def posix-class-source
  {"alnum" "A-Za-z0-9"
   "alpha" "A-Za-z"
   "blank" "\\x09\\x20"
   "cntrl" "\\x00-\\x1F\\x7F"
   "digit" "0-9"
   "graph" "\\x21-\\x7E"
   "lower" "a-z"
   "print" "\\x20-\\x7E"
   "punct" "\\x21-\\x2F\\x3A-\\x40\\x5B-\\x60\\x7B-\\x7E"
   "space" "\\x09-\\x0D\\x20"
   "upper" "A-Z"
   "xdigit" "A-Fa-f0-9"})

(defn translate-posix-pattern [pattern]
  (loop [index 0
         in-class? false
         translated []]
    (if (>= index (count pattern))
      (apply str translated)
      (let [current (.charAt pattern index)
            next (if (< (inc index) (count pattern))
                   (.charAt pattern (inc index))
                   "")]
        (cond
          (= current "\\")
          (if (= next "")
            (recur (inc index) in-class? (conj translated current))
            (recur (+ index 2) in-class?
                   (conj translated current next)))

          (and in-class? (= current "[") (= next ":"))
          (let [end (.indexOf pattern ":]" (+ index 2))]
            (when (= -1 end)
              (evaluation-failure! "invalid-regex"
                                   {"detail_class" "unterminated-posix-class"}))
            (let [class-name (subs pattern (+ index 2) end)
                  replacement (get posix-class-source class-name)]
              (when-not replacement
                (evaluation-failure! "invalid-regex"
                                     {"detail_class" "unknown-posix-class"
                                      "class_name" class-name}))
              (recur (+ end 2) in-class? (conj translated replacement))))

          (= current "[")
          (recur (inc index) true (conj translated current))

          (and in-class? (= current "]"))
          (recur (inc index) false (conj translated current))

          :else
          (recur (inc index) in-class? (conj translated current)))))))

(defn compile-regex [pattern full-match?]
  (try
    (let [translated (translate-posix-pattern pattern)]
      (js/RegExp. (if full-match?
                    (str "^(?:" translated ")$")
                    translated)))
    (catch :default cause
      (if (true? (get (ex-data cause) "pnix_error"))
        (throw cause)
        (evaluation-failure! "invalid-regex"
                             {"detail_class" "invalid-pattern"})))))

(defn match-value [pattern value]
  (when-not (and (string? pattern) (string? value))
    (evaluation-failure! "type-error" {"operation" "match"}))
  (let [result (.exec (compile-regex pattern true) value)]
    (when result
      (loop [index 1
             captures []]
        (if (>= index (.-length result))
          captures
          (let [capture (aget result index)]
            (recur (inc index)
                   (conj captures
                         (if (undefined? capture) nil capture)))))))))

(defn split-value [pattern value]
  (when-not (and (string? pattern) (string? value))
    (evaluation-failure! "type-error" {"operation" "split"}))
  (let [regex (try
                (js/RegExp. (translate-posix-pattern pattern) "g")
                (catch :default cause
                  (if (true? (get (ex-data cause) "pnix_error"))
                    (throw cause)
                    (evaluation-failure! "invalid-regex"
                                         {"detail_class" "invalid-pattern"}))))]
    (loop [last-index 0
           parts []]
      (let [match (.exec regex value)]
        (if-not match
          (conj parts (subs value last-index))
          (let [match-index (.-index match)
                matched (aget match 0)
                match-end (+ match-index (.-length matched))
                captures
                (loop [index 1 result []]
                  (if (>= index (.-length match))
                    result
                    (let [capture (aget match index)]
                      (recur (inc index)
                             (conj result
                                   (if (undefined? capture)
                                     nil
                                     capture))))))]
            (when (and (= match-index match-end)
                       (= (.-lastIndex regex) match-index))
              (set! (.-lastIndex regex) (inc match-index)))
            (recur match-end
                   (conj parts
                         (subs value last-index match-index)
                         captures))))))))

(defn bytes-start-with? [source index needle]
  (and (<= (+ index (.-length needle)) (.-length source))
       (loop [needle-index 0]
         (if (= needle-index (.-length needle))
           true
           (and (= (aget source (+ index needle-index))
                   (aget needle needle-index))
                (recur (inc needle-index)))))))

(defn replace-strings-value [from-values to-values value]
  (when-not (and (vector? from-values)
                 (vector? to-values)
                 (= (count from-values) (count to-values))
                 (string-value? value))
    (evaluation-failure! "type-error" {"operation" "replaceStrings"}))
  (let [from-bytes
        (mapv (fn [entry]
                (let [entry (force-cell entry)]
                  (if (string-value? entry)
                    (string-bytes entry)
                    (evaluation-failure! "type-error"
                                         {"operation" "replaceStrings"}))))
              from-values)
        to-bytes
        (mapv (fn [entry]
                (let [entry (force-cell entry)]
                  (if (string-value? entry)
                    (string-bytes entry)
                    (evaluation-failure! "type-error"
                                         {"operation" "replaceStrings"}))))
              to-values)
        source (string-bytes value)
        source-length (.-length source)]
    (loop [index 0
           output []]
      (let [match-index
            (loop [candidate 0]
              (cond
                (= candidate (count from-bytes)) nil
                (bytes-start-with? source index (nth from-bytes candidate))
                candidate
                :else (recur (inc candidate))))]
        (cond
          (some? match-index)
          (let [needle (nth from-bytes match-index)
                replacement (nth to-bytes match-index)
                needle-length (.-length needle)]
            (if (zero? needle-length)
              (if (= index source-length)
                (decode-byte-string
                 (concatenate-byte-arrays (conj output replacement)))
                (recur (inc index)
                       (conj output
                             replacement
                             (.slice source index (inc index)))))
              (recur (+ index needle-length)
                     (conj output replacement))))

          (= index source-length)
          (decode-byte-string (concatenate-byte-arrays output))

          :else
          (recur (inc index)
                 (conj output (.slice source index (inc index)))))))))

(def json-number-pattern
  #"^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?")

(defn json-whitespace? [value]
  (contains? #{" " "\t" "\n" "\r"} value))

(defn json-skip-whitespace [source start]
  (loop [index start]
    (if (and (< index (count source))
             (json-whitespace? (.charAt source index)))
      (recur (inc index))
      index)))

(defn invalid-json! [detail-class index]
  (evaluation-failure! "invalid-json"
                       {"detail_class" detail-class
                        "offset" index}))

(defn json-parse-string [source start]
  (loop [index (inc start)
         escaped? false]
    (when (>= index (count source))
      (invalid-json! "unterminated-string" start))
    (let [current (.charAt source index)]
      (cond
        escaped?
        (recur (inc index) false)

        (= current "\\")
        (recur (inc index) true)

        (= current "\"")
        (let [end (inc index)
              token (subs source start end)]
          (try
            {:value (js/JSON.parse token) :index end}
            (catch :default _
              (invalid-json! "invalid-string" start))))

        (< (.charCodeAt current 0) 32)
        (invalid-json! "control-character-in-string" index)

        :else
        (recur (inc index) false)))))

(defn json-parse-number [source start]
  (let [matched (re-find json-number-pattern (subs source start))
        lexeme (if (vector? matched) (first matched) matched)]
    (when-not lexeme
      (invalid-json! "invalid-number" start))
    (if (re-find #"[.eE]" lexeme)
      (let [value (js/Number lexeme)]
        (when-not (js/Number.isFinite value)
          (invalid-json! "number-out-of-range" start))
        {:value value :index (+ start (count lexeme))})
      (let [value (js/BigInt lexeme)]
        (when (or (< value min-i64) (> value max-i64))
          (invalid-json! "integer-out-of-range" start))
        {:value value :index (+ start (count lexeme))}))))

(defn json-parse-array [source start]
  (loop [index (json-skip-whitespace source (inc start))
         values []]
    (when (>= index (count source))
      (invalid-json! "unterminated-array" start))
    (if (= "]" (.charAt source index))
      {:value values :index (inc index)}
      (let [{value :value next-index :index}
            (json-parse-value source index)
            delimiter-index (json-skip-whitespace source next-index)
            delimiter (.charAt source delimiter-index)]
        (cond
          (= delimiter ",")
          (let [next-index
                (json-skip-whitespace source (inc delimiter-index))]
            (when (= "]" (.charAt source next-index))
              (invalid-json! "trailing-array-comma" delimiter-index))
            (recur next-index (conj values value)))

          (= delimiter "]")
          {:value (conj values value) :index (inc delimiter-index)}

          :else
          (invalid-json! "array-delimiter-required" delimiter-index))))))

(defn json-parse-object [source start]
  (loop [index (json-skip-whitespace source (inc start))
         fields {}]
    (when (>= index (count source))
      (invalid-json! "unterminated-object" start))
    (if (= "}" (.charAt source index))
      {:value (->AttrsetValue fields) :index (inc index)}
      (do
        (when-not (= "\"" (.charAt source index))
          (invalid-json! "object-key-required" index))
        (let [{key :value key-end :index} (json-parse-string source index)
              colon-index (json-skip-whitespace source key-end)]
          (when-not (= ":" (.charAt source colon-index))
            (invalid-json! "object-colon-required" colon-index))
          (let [{value :value value-end :index}
                (json-parse-value
                 source
                 (json-skip-whitespace source (inc colon-index)))
                delimiter-index (json-skip-whitespace source value-end)
                delimiter (.charAt source delimiter-index)
                next-fields (assoc fields key value)]
            (cond
              (= delimiter ",")
              (let [next-index
                    (json-skip-whitespace source (inc delimiter-index))]
                (when (= "}" (.charAt source next-index))
                  (invalid-json! "trailing-object-comma" delimiter-index))
                (recur next-index next-fields))

              (= delimiter "}")
              {:value (->AttrsetValue next-fields)
               :index (inc delimiter-index)}

              :else
              (invalid-json! "object-delimiter-required"
                             delimiter-index))))))))

(defn json-parse-value [source start]
  (let [index (json-skip-whitespace source start)
        remaining (subs source index)
        current (.charAt source index)]
    (cond
      (= current "\"") (json-parse-string source index)
      (= current "[") (json-parse-array source index)
      (= current "{") (json-parse-object source index)
      (or (= current "-") (and (not= current "")
                                 (<= 48 (.charCodeAt current 0) 57)))
      (json-parse-number source index)
      (.startsWith remaining "true") {:value true :index (+ index 4)}
      (.startsWith remaining "false") {:value false :index (+ index 5)}
      (.startsWith remaining "null") {:value nil :index (+ index 4)}
      :else (invalid-json! "value-required" index))))

(defn from-json-value [source]
  (when-not (string? source)
    (evaluation-failure! "type-error" {"operation" "fromJSON"}))
  (let [{value :value index :index} (json-parse-value source 0)
        end (json-skip-whitespace source index)]
    (when-not (= end (count source))
      (invalid-json! "trailing-input" end))
    value))

(defn nix-json-float [value]
  (when-not (js/Number.isFinite value)
    (evaluation-failure! "invalid-guest-value"
                         {"operation" "toJSON"
                          "detail_class" "non-finite-float"}))
  (let [negative-zero? (js/Object.is value -0)
        magnitude (js/Math.abs value)]
    (cond
      (or (zero? value) negative-zero?) "0.0"

      (or (>= magnitude 1.0e15)
          (< magnitude 1.0e-4))
      (let [[mantissa exponent] (.split (.toExponential value) "e")
            sign (.charAt exponent 0)
            digits (subs exponent 1)
            digits (if (and (= sign "-") (= 1 (count digits)))
                     (str "0" digits)
                     digits)]
        (str mantissa "e" sign digits))

      (js/Number.isInteger value)
      (str (.toString value) ".0")

      :else (.toString value))))

(defn json-cycle! []
  (evaluation-failure! "cycle-detected"
                       {"operation" "toJSON"}))

(defn to-json-active [active identity render]
  (when (.has active identity)
    (json-cycle!))
  (.add active identity)
  (try
    (render)
    (finally
      (.delete active identity))))

(defn to-json-value [value active]
  (if (instance? Cell value)
    (to-json-active active value
                    #(to-json-value (force-cell value) active))
    (cond
      (nil? value) "null"
      (boolean? value) (if value "true" "false")
      (integer-value? value) (str value)
      (number? value) (nix-json-float value)
      (string? value) (js/JSON.stringify value)

      (instance? ByteStringValue value)
      (evaluation-failure! "invalid-guest-value"
                           {"operation" "toJSON"
                            "detail_class" "invalid-utf8"})

      (vector? value)
      (to-json-active
       active value
       #(str "["
             (apply str
                    (interpose ","
                               (map (fn [item]
                                      (to-json-value item active))
                                    value)))
             "]"))

      (instance? AttrsetValue value)
      (to-json-active
       active value
       #(str "{"
             (apply str
                    (interpose
                     ","
                     (map (fn [name]
                            (str (js/JSON.stringify name)
                                 ":"
                                 (to-json-value
                                  (get (:fields value) name)
                                  active)))
                          (sorted-field-names (:fields value)))))
             "}"))

      :else
      (evaluation-failure! "type-error"
                           {"operation" "toJSON"}))))

(defn to-json [value]
  (to-json-value value (js/Set.)))

(defn integer-value? [value]
  (js* "typeof ~{} === 'bigint'" value))

(defn numeric-value? [value]
  (or (integer-value? value) (number? value)))

(defn as-double [value]
  (if (integer-value? value) (js/Number value) value))

(defn integer-zero? [value]
  (js* "~{} === 0n" value))

(defn integer-negative? [value]
  (js* "~{} < 0n" value))

(defn integer-add [left right] (js* "(~{} + ~{})" left right))
(defn integer-subtract [left right] (js* "(~{} - ~{})" left right))
(defn integer-multiply [left right] (js* "(~{} * ~{})" left right))
(defn integer-divide [left right] (js* "(~{} / ~{})" left right))
(defn integer-negate [value] (js* "(-~{})" value))

(defn evaluation-error [class evidence]
  (ex-info class
           {"pnix_error" true
            "phase" "eval"
            "class" class
            "evidence" evidence}))

(defn evaluation-failure! [class evidence]
  (throw (evaluation-error class evidence)))

(defn cell [expression environment]
  (->Cell expression environment (atom {:tag :unevaluated})))

(defn value-cell [value]
  (->Cell nil nil (atom {:tag :evaluated :value value})))

(defn pattern-call-environment [closure argument-cell]
  (let [pattern (:parameter closure)
        argument (force-cell argument-cell)]
    (when-not (instance? AttrsetValue argument)
      (evaluation-failure! "type-error"
                           {"operation" "attrset-pattern-call"}))
    (let [argument-fields (:fields argument)
          pattern-names (set (map :name (:fields pattern)))]
      (when (and (not (:ellipsis pattern))
                 (some #(not (contains? pattern-names %))
                       (keys argument-fields)))
        (evaluation-failure! "type-error"
                             {"operation" "attrset-pattern-extra-argument"}))
      (let [environment-reference (atom nil)
            bindings
            (into {}
                  (map (fn [{:keys [name] :as field}]
                         [name
                          (if (contains? argument-fields name)
                            (get argument-fields name)
                            (if (contains? field :default)
                              (cell (:default field) environment-reference)
                              (evaluation-failure!
                               "type-error"
                               {"operation" "attrset-pattern-missing-argument"
                                "name" name})))]))
                  (:fields pattern))
            result (cond-> (merge (:environment closure) bindings)
                     (:capture pattern)
                     (assoc (:capture pattern) argument-cell))]
        (reset! environment-reference result)
        result))))

(defn closure-call-environment [closure argument-cell]
  (if (string? (:parameter closure))
    (assoc (:environment closure) (:parameter closure) argument-cell)
    (pattern-call-environment closure argument-cell)))

(defn leading-space-count [value]
  (loop [index 0]
    (if (and (< index (count value))
             (= " " (.charAt value index)))
      (recur (inc index))
      index)))

(defn normalize-indented-string [value]
  (let [initial-lines (vec (.split value "\n"))
        lines (if (and (seq initial-lines)
                       (boolean (re-matches #" *" (first initial-lines))))
                (subvec initial-lines 1)
                initial-lines)
        nonblank (filter #(not (boolean (re-matches #" *" %))) lines)
        indentation (if (seq nonblank)
                      (apply min (map leading-space-count nonblank))
                      0)]
    (apply str
           (interpose "\n"
                      (map (fn [line]
                             (subs line (min indentation
                                             (leading-space-count line))))
                           lines)))))

(defn evaluate-string-segments [segments environment]
  (apply str
         (map (fn [segment]
                (if (= :text (:kind segment))
                  (:value segment)
                  (let [value (evaluate-expression (:value segment)
                                                   environment)]
                    (if (string? value)
                      value
                      (evaluation-failure!
                       "type-error"
                       {"operation" "string-interpolation"})))))
              segments)))

(defn evaluate-indented-string [segments environment]
  (normalize-indented-string
   (evaluate-string-segments segments environment)))

(defn evaluate-attribute-name [field environment]
  (if (contains? field :name)
    (:name field)
    (let [value (evaluate-expression (:name-expression field) environment)]
      (if (string? value)
        value
        (evaluation-failure! "type-error"
                             {"operation" "dynamic-attribute-name"})))))

(defn maybe-attribute-name [field environment]
  (if (contains? field :name)
    (:name field)
    (let [value (evaluate-expression (:name-expression field) environment)]
      (when (string? value) value))))

(defn session-cached [expression environment create]
  (if-not *argument-cell-cache*
    (create)
    (let [by-environment
          (or (.get *argument-cell-cache* expression)
              (let [created (js/Map.)]
                (.set *argument-cell-cache* expression created)
                created))]
      (if (.has by-environment environment)
        (.get by-environment environment)
        (let [created (create)]
          (.set by-environment environment created)
          created)))))

(defn call-argument-cell [call-expression environment]
  (session-cached
   call-expression
   environment
   #(cell (:argument call-expression) (atom environment))))

(defn force-cell-explicit [value]
  (binding [*forcing-cells* true
            *argument-cell-cache* (js/Map.)]
    (let [active (doto (js/Set.) (.add value))]
      (loop [pending [value]]
        (let [current (peek pending)
              current-snapshot @(:state current)]
          (case (:tag current-snapshot)
            :evaluated
            (if (= 1 (count pending))
              (:value current-snapshot)
              (do
                (.delete active current)
                (recur (pop pending))))

            :failed
            (let [cause (:cause current-snapshot)]
              (doseq [cell pending]
                (reset! (:state cell) {:tag :failed :cause cause}))
              (throw cause))

            :evaluating
            (let [cause (evaluation-error "cycle-detected" {})]
              (doseq [cell pending]
                (reset! (:state cell) {:tag :failed :cause cause}))
              (throw cause))

            :unevaluated
            (do
              (reset! (:state current) {:tag :evaluating})
              (let [attempt
                    (try
                      {:tag :value
                       :value
                       (evaluate-expression
                        (:expression current)
                        @(:environment current))}
                      (catch :default cause
                        {:tag :cause :cause cause}))]
                (if (= :value (:tag attempt))
                  (do
                    (reset! (:state current)
                            {:tag :evaluated :value (:value attempt)})
                    (recur pending))
                  (let [cause (:cause attempt)
                        dependency
                        (get (ex-data cause) force-dependency-key)]
                    (if dependency
                      (do
                        (reset! (:state current) {:tag :unevaluated})
                        (if (.has active dependency)
                          (let [cycle
                                (evaluation-error "cycle-detected" {})]
                            (doseq [cell pending]
                              (reset! (:state cell)
                                      {:tag :failed :cause cycle}))
                            (throw cycle))
                          (do
                            (.add active dependency)
                            (recur (conj pending dependency)))))
                      (do
                        (reset! (:state current)
                                {:tag :failed :cause cause})
                        (recur pending)))))))))))))

(defn force-cell [value]
  (if-not (instance? Cell value)
    value
    (let [snapshot @(:state value)]
      (case (:tag snapshot)
        :evaluated (:value snapshot)
        :evaluating (evaluation-failure! "cycle-detected" {})
        :failed (throw (:cause snapshot))
        :unevaluated
        (if *forcing-cells*
          (throw (ex-info "force dependency"
                          {force-dependency-key value}))
          (if (< *force-depth* force-stack-threshold)
            (do
              (reset! (:state value) {:tag :evaluating})
              (try
                (let [result
                      (binding [*force-depth* (inc *force-depth*)]
                        (evaluate-expression (:expression value)
                                             @(:environment value)))]
                  (reset! (:state value) {:tag :evaluated :value result})
                  result)
                (catch :default cause
                  (reset! (:state value) {:tag :failed :cause cause})
                  (throw cause))))
            (force-cell-explicit value)))))))

(defn lookup [environment name]
  (if (contains? environment name)
    (force-cell (get environment name))
    (evaluation-failure! "unknown-variable" {"name" name})))

(defn require-boolean [value]
  (if (boolean? value)
    value
    (evaluation-failure! "type-error"
                         {"expected" {"kind" "boolean"}
                          "actual" {"kind" (cond
                                              (number? value) "integer"
                                              (string? value) "string"
                                              (nil? value) "null"
                                              :else "other")}})))

(defn require-condition [value]
  (if (boolean? value)
    value
    (evaluation-failure! "non-boolean-condition"
                         {"actual" {"kind" (cond
                                              (integer-value? value) "integer"
                                              (string? value) "string"
                                              (nil? value) "null"
                                              :else "other")}})))

(defn require-integer [value]
  (if (integer-value? value)
    value
    (evaluation-failure! "type-error"
                         {"expected" {"kind" "i64"}
                          "actual" {"kind" (cond
                                              (boolean? value) "boolean"
                                              (string? value) "string"
                                              (nil? value) "null"
                                              :else "other")}})))

(defn require-number [value]
  (if (numeric-value? value)
    value
    (evaluation-failure! "type-error"
                         {"expected" {"kind" "number"}
                          "actual" {"kind" (cond
                                              (boolean? value) "boolean"
                                              (string-value? value) "string"
                                              (nil? value) "null"
                                              :else "other")}})))

(defn numeric-binary [operation left right]
  (if (and (= operation :add)
           (string-value? left)
           (string-value? right))
    (decode-byte-string
     (concatenate-byte-arrays [(string-bytes left) (string-bytes right)]))
    (let [left (require-number left)
          right (require-number right)]
      (if (and (integer-value? left) (integer-value? right))
        (case operation
          :add (checked-integer (integer-add left right))
          :subtract (checked-integer (integer-subtract left right))
          :multiply (checked-integer (integer-multiply left right))
          :divide (do
                    (when (integer-zero? right)
                      (evaluation-failure! "division-by-zero" {}))
                    (checked-integer (integer-divide left right))))
        (let [left (as-double left)
              right (as-double right)]
          (when (and (= operation :divide) (zero? right))
            (evaluation-failure! "division-by-zero" {}))
          (case operation
            :add (+ left right)
            :subtract (- left right)
            :multiply (* left right)
            :divide (/ left right)))))))

(defn numeric-compare [operation left right]
  (let [left (require-number left)
        right (require-number right)
        [left right] (if (or (number? left) (number? right))
                       [(as-double left) (as-double right)]
                       [left right])]
    (case operation
      :less (< left right)
      :less-equal (not (> left right))
      :greater (> left right)
      :greater-equal (not (< left right)))))

(defn ordered-compare [operation left right]
  (case operation
    :less (ordered-less left right)
    :less-equal (not (ordered-less right left))
    :greater (ordered-less right left)
    :greater-equal (not (ordered-less left right))))

(defn apply-value [function-value argument-cell]
  (evaluate-expression
   {:op :call-value
    :function function-value
    :argument-cell argument-cell}
   {}))

(defn sort-less? [comparator left-cell right-cell]
  (let [partial (apply-value comparator left-cell)
        result (apply-value partial right-cell)]
    (require-boolean result)))

(defn merge-sorted [comparator left right]
  (loop [left-index 0
         right-index 0
         result []]
    (cond
      (= left-index (count left))
      (into result (subvec right right-index))

      (= right-index (count right))
      (into result (subvec left left-index))

      (sort-less? comparator
                  (nth right right-index)
                  (nth left left-index))
      (recur left-index
             (inc right-index)
             (conj result (nth right right-index)))

      :else
      (recur (inc left-index)
             right-index
             (conj result (nth left left-index))))))

(defn stable-sort [comparator values]
  (if (< (count values) 2)
    values
    (let [middle (quot (count values) 2)]
      (merge-sorted comparator
                    (stable-sort comparator (subvec values 0 middle))
                    (stable-sort comparator (subvec values middle))))))

(defn checked-integer [value]
  (if (and (integer-value? value)
           (js* "~{} >= ~{} && ~{} <= ~{}"
                value min-i64 value max-i64))
    value
    (evaluation-failure! "integer-overflow" {})))

(defn numeric-round [operation value]
  (cond
    (integer-value? value)
    (let [as-number (js/Number value)]
      (if (and (js/Number.isFinite as-number)
               (= (js/BigInt as-number) value))
        value
        (evaluation-failure! "integer-precision-loss"
                             {"operation" (name operation)})))

    (number? value)
    (let [rounded (case operation
                    :ceil (js/Math.ceil value)
                    :floor (js/Math.floor value))]
      (checked-integer (js/BigInt (.toFixed rounded 0))))

    :else
    (evaluation-failure! "type-error"
                         {"operation" (name operation)})))

(defn numeric-abs [value]
  (cond
    (integer-value? value)
    (if (integer-negative? value) (integer-negate value) value)

    (number? value)
    (js/Math.abs value)

    :else
    (evaluation-failure! "type-error" {"operation" "abs"})))

(defn nix-to-string [value]
  (cond
    (string-value? value) value
    (integer-value? value) (str value)
    (number? value) (cond
                      (js/Number.isNaN value) "nan"
                      (= js/Number.POSITIVE_INFINITY value) "inf"
                      (= js/Number.NEGATIVE_INFINITY value) "-inf"
                      (and (zero? value)
                           (= js/Number.NEGATIVE_INFINITY (/ 1 value)))
                      "-0.000000"
                      :else (.toFixed value 6))
    (true? value) "1"
    (or (false? value) (nil? value)) ""

    (vector? value)
    (let [strings (mapv #(nix-to-string (force-cell %)) value)]
      (decode-byte-string
       (concatenate-byte-arrays
        (vec
         (mapcat (fn [[index string-value]]
                   (if (zero? index)
                     [(string-bytes string-value)]
                     [(.encode utf8-encoder " ")
                      (string-bytes string-value)]))
                 (map-indexed vector strings))))))

    (instance? AttrsetValue value)
    (let [fields (:fields value)]
      (if (contains? fields "outPath")
        (nix-to-string (force-cell (get fields "outPath")))
        (evaluation-failure! "type-error"
                             {"operation" "toString"})))

    :else
    (evaluation-failure! "type-error"
                         {"operation" "toString"})))

(def catchable-evaluation-classes
  #{"assertion-failed" "explicit-throw"})

(defn try-eval-cell [argument-cell]
  (try
    (->AttrsetValue
     {"success" true
      "value" (force-cell argument-cell)})
    (catch :default error
      (let [data (ex-data error)
            class (get data "class")]
        (if (and (true? (get data "pnix_error"))
                 (contains? catchable-evaluation-classes class))
          (->AttrsetValue {"success" false "value" false})
          (throw error))))))

(def hash-algorithms
  #{"md5" "sha1" "sha256" "sha512"})

(defn byte-hex [value]
  (let [text (.toString (bit-and value 255) 16)]
    (if (= 1 (.-length text))
      (str "0" text)
      text)))

(defn hash-bytes [algorithm bytes]
  (let [hasher (case algorithm
                 "md5" (goog.crypt.Md5.)
                 "sha1" (goog.crypt.Sha1.)
                 "sha256" (goog.crypt.Sha256.)
                 "sha512" (goog.crypt.Sha512.))]
    (.update hasher (js/Array.from bytes))
    (apply str (map byte-hex (.digest hasher)))))

(defn parse-derivation-name [value]
  (when-not (string? value)
    (evaluation-failure! "type-error" {"operation" "parseDrvName"}))
  (let [matched (.match value #"^(.*?)-([0-9].*)$")]
    (if matched
      (->AttrsetValue {"name" (aget matched 1)
                       "version" (aget matched 2)})
      (->AttrsetValue {"name" value "version" ""}))))

(defn normalize-absolute-path [value]
  (let [segments (.split value "/")
        normalized
        (reduce (fn [result segment]
                  (cond
                    (or (= segment "") (= segment ".")) result
                    (= segment "..") (if (seq result) (pop result) result)
                    :else (conj result segment)))
                []
                segments)]
    (str "/" (apply str (interpose "/" normalized)))))


(defn node-require [module-name]
  (try
    (js/require module-name)
    (catch :default _
      nil)))

(defn path-string [value operation]
  (cond
    (string-value? value)
    (let [s (if (instance? ByteStringValue value)
              (.decode (js/TextDecoder. "utf-8") (:bytes value))
              value)]
      (cond
        (= s "~") (or (.-HOME (.-env js/process)) s)
        (.startsWith s "~/") (str (or (.-HOME (.-env js/process)) "")
                                  (subs s 1))
        :else s))
    :else
    (evaluation-failure! "type-error" {"operation" operation})))

(defn require-string-arg [value operation]
  (if (string-value? value)
    value
    (evaluation-failure! "type-error" {"operation" operation})))

(defn string-text [value]
  (if (instance? ByteStringValue value)
    (.decode (js/TextDecoder. "utf-8") (:bytes value))
    value))

(defn bytes-has-prefix? [source prefix]
  (and (<= (.-length prefix) (.-length source))
       (loop [index 0]
         (if (= index (.-length prefix))
           true
           (and (= (aget source index) (aget prefix index))
                (recur (inc index)))))))

(defn bytes-has-suffix? [source suffix]
  (let [source-length (.-length source)
        suffix-length (.-length suffix)]
    (and (<= suffix-length source-length)
         (loop [index 0]
           (if (= index suffix-length)
             true
             (and (= (aget source (+ (- source-length suffix-length) index))
                     (aget suffix index))
                  (recur (inc index))))))))

(defn apply-value2 [function-value left right]
  (apply-value (apply-value function-value (if (instance? Cell left)
                                             left
                                             (value-cell left)))
               (if (instance? Cell right)
                 right
                 (value-cell right))))

(defn apply-value3 [function-value a b c]
  (apply-value (apply-value2 function-value a b)
               (if (instance? Cell c) c (value-cell c))))

(defn flatten-value [value]
  (let [value (force-cell value)]
    (if (vector? value)
      (vec (mapcat flatten-value value))
      [(if (instance? Cell value) value (value-cell value))])))

(defn recursive-update-value [left right]
  (let [left (force-cell left)
        right (force-cell right)]
    (if (and (instance? AttrsetValue left)
             (instance? AttrsetValue right))
      (let [left-fields (:fields left)
            right-fields (:fields right)]
        (->AttrsetValue
         (reduce (fn [fields [name right-cell]]
                   (if (and (contains? left-fields name)
                            (instance? AttrsetValue (force-cell (get left-fields name)))
                            (instance? AttrsetValue (force-cell right-cell)))
                     (assoc fields
                            name
                            (value-cell
                             (recursive-update-value (get left-fields name)
                                                     right-cell)))
                     (assoc fields name right-cell)))
                 left-fields
                 right-fields)))
      right)))

(defn get-attr-from-path [path attrs]
  (when-not (vector? path)
    (evaluation-failure! "type-error" {"operation" "getAttrFromPath"}))
  (loop [remaining path
         current attrs]
    (if (empty? remaining)
      current
      (let [name (force-cell (first remaining))
            current (force-cell current)]
        (when-not (string? name)
          (evaluation-failure! "type-error" {"operation" "getAttrFromPath"}))
        (when-not (instance? AttrsetValue current)
          (evaluation-failure! "type-error" {"operation" "getAttrFromPath"}))
        (if (contains? (:fields current) name)
          (recur (rest remaining)
                 (force-cell (get (:fields current) name)))
          (evaluation-failure! "attribute-missing" {"name" name}))))))

(defn has-attr-by-path [path attrs]
  (when-not (vector? path)
    (evaluation-failure! "type-error" {"operation" "hasAttrByPath"}))
  (loop [remaining path
         current attrs]
    (if (empty? remaining)
      true
      (let [name (force-cell (first remaining))
            current (force-cell current)]
        (when-not (string? name)
          (evaluation-failure! "type-error" {"operation" "hasAttrByPath"}))
        (if-not (and (instance? AttrsetValue current)
                     (contains? (:fields current) name))
          false
          (recur (rest remaining)
                 (get (:fields current) name)))))))

(defn attr-by-path [path default attrs]
  (when-not (vector? path)
    (evaluation-failure! "type-error" {"operation" "attrByPath"}))
  (loop [remaining path
         current attrs]
    (if (empty? remaining)
      (force-cell current)
      (let [name (force-cell (first remaining))
            current (force-cell current)]
        (when-not (string? name)
          (evaluation-failure! "type-error" {"operation" "attrByPath"}))
        (if-not (and (instance? AttrsetValue current)
                     (contains? (:fields current) name))
          default
          (recur (rest remaining)
                 (get (:fields current) name)))))))

(defn escape-xml [s]
  (-> s
      (.split "&") (.join "&amp;")
      (.split "<") (.join "&lt;")
      (.split ">") (.join "&gt;")
      (.split "\"") (.join "&quot;")))

(defn to-xml-value [value]
  (let [value (force-cell value)]
    (cond
      (nil? value) "<null />"
      (true? value) "<bool value=\"true\" />"
      (false? value) "<bool value=\"false\" />"
      (integer-value? value) (str "<int>" value "</int>")
      (number? value) (str "<float>" value "</float>")
      (string-value? value)
      (str "<string value=\"" (escape-xml (string-text value)) "\" />")
      (vector? value)
      (str "<list>"
           (apply str (map to-xml-value value))
           "</list>")
      (instance? AttrsetValue value)
      (str "<attrs>"
           (apply str
                  (map (fn [name]
                         (str "<attr name=\""
                              (escape-xml name)
                              "\">"
                              (to-xml-value (get (:fields value) name))
                              "</attr>"))
                       (sorted-field-names (:fields value))))
           "</attrs>")
      (or (instance? ClosureValue value)
          (instance? BuiltinValue value))
      "<function />"
      :else
      (evaluation-failure! "type-error" {"operation" "toXML"}))))

(defn to-xml [value]
  (str "<?xml version='1.0' encoding='utf-8'?>\n"
       "<expr>"
       (to-xml-value value)
       "</expr>\n"))

(defn node-write-file [name content]
  (let [fs (node-require "fs")
        path (node-require "path")
        os (node-require "os")]
    (when-not (and fs path os)
      (evaluation-failure! "io-unavailable" {"operation" "toFile"}))
    (let [dir (.mkdtempSync fs (.join path (.tmpdir os) "pnix-tofile-"))
          file (.join path dir name)]
      (.writeFileSync fs file content "utf8")
      file)))

(defn node-read-file [path]
  (let [fs (node-require "fs")]
    (when-not fs
      (evaluation-failure! "io-unavailable" {"operation" "readFile"}))
    (try
      (.readFileSync fs path "utf8")
      (catch :default error
        (evaluation-failure! "io-error"
                             {"operation" "readFile"
                              "path" path
                              "message" (str error)})))))

(defn node-path-exists [path]
  (let [fs (node-require "fs")]
    (when-not fs
      (evaluation-failure! "io-unavailable" {"operation" "pathExists"}))
    (.existsSync fs path)))

(defn node-read-dir [path]
  (let [fs (node-require "fs")]
    (when-not fs
      (evaluation-failure! "io-unavailable" {"operation" "readDir"}))
    (try
      (let [entries (.readdirSync fs path #js {:withFileTypes true})]
        (->AttrsetValue
         (into {}
               (map (fn [entry]
                      (let [name (.-name entry)
                            kind (cond
                                   (.isFile entry) "regular"
                                   (.isDirectory entry) "directory"
                                   (.isSymbolicLink entry) "symlink"
                                   :else "unknown")]
                        [name kind]))
                    entries))))
      (catch :default error
        (evaluation-failure! "io-error"
                             {"operation" "readDir"
                              "path" path
                              "message" (str error)})))))

(defn node-fetch-url [url]
  (let [fs (node-require "fs")
        path (node-require "path")
        os (node-require "os")
        cp (node-require "child_process")]
    (when-not (and fs path os cp)
      (evaluation-failure! "io-unavailable" {"operation" "fetchurl"}))
    (let [file (.join path
                      (.tmpdir os)
                      (str "pnix-fetch-" (.now js/Date) "-"
                           (.toString (.random js/Math) 36) ".out"))
          script
          (str "const fs=require('fs');"
               "fetch(process.argv[1]).then(r=>{"
               "if(!r.ok)throw new Error('HTTP '+r.status);"
               "return r.arrayBuffer()}).then(b=>{"
               "fs.writeFileSync(process.argv[2],Buffer.from(b));"
               "}).catch(e=>{console.error(e);process.exit(1);})")]
      (try
        (.execFileSync cp "node" #js ["-e" script url file]
                       #js {:stdio "pipe"})
        file
        (catch :default error
          (evaluation-failure! "io-error"
                               {"operation" "fetchurl"
                                "url" url
                                "message" (str error)}))))))

(defn fetch-url-argument [argument]
  (cond
    (string-value? argument) (string-text argument)
    (instance? AttrsetValue argument)
    (let [fields (:fields argument)]
      (if (contains? fields "url")
        (string-text (force-cell (get fields "url")))
        (evaluation-failure! "type-error" {"operation" "fetchurl"})))
    :else
    (evaluation-failure! "type-error" {"operation" "fetchurl"})))

(defn split-string-value [separator source]
  (let [sep (string-text separator)
        s (string-text source)]
    (if (zero? (count sep))
      (mapv str s)
      (let [parts (.split s sep)]
        (vec parts)))))

(defn filter-attrs-value [function-value attrs]
  (when-not (instance? AttrsetValue attrs)
    (evaluation-failure! "type-error" {"operation" "filterAttrs"}))
  (->AttrsetValue
   (into {}
         (keep (fn [[name attribute-cell]]
                 (when (require-boolean
                        (apply-value2 function-value name attribute-cell))
                   [name attribute-cell]))
               (:fields attrs)))))

(defn remove-attrs-value [attrs names]
  (when-not (instance? AttrsetValue attrs)
    (evaluation-failure! "type-error" {"operation" "removeAttrs"}))
  (when-not (vector? names)
    (evaluation-failure! "type-error" {"operation" "removeAttrs"}))
  (let [remove-set (into #{} (map (fn [c] (string-text (force-cell c))) names))]
    (->AttrsetValue
     (into {} (remove (fn [[k _]] (contains? remove-set k)) (:fields attrs))))))

(defn intersect-attrs-value [e1 e2]
  (when-not (and (instance? AttrsetValue e1) (instance? AttrsetValue e2))
    (evaluation-failure! "type-error" {"operation" "intersectAttrs"}))
  (->AttrsetValue
   (into {} (filter (fn [[k _]] (contains? (:fields e1) k)) (:fields e2)))))

(defn cat-attrs-value [attribute attrsets]
  (when-not (string? attribute)
    (evaluation-failure! "type-error" {"operation" "catAttrs"}))
  (when-not (vector? attrsets)
    (evaluation-failure! "type-error" {"operation" "catAttrs"}))
  (into []
        (keep (fn [c]
                (let [attrs (force-cell c)]
                  (when-not (instance? AttrsetValue attrs)
                    (evaluation-failure! "type-error" {"operation" "catAttrs"}))
                  (when (contains? (:fields attrs) attribute)
                    (get (:fields attrs) attribute)))))
        attrsets))

(defn unique-values [values]
  (when-not (vector? values)
    (evaluation-failure! "type-error" {"operation" "unique"}))
  (loop [remaining values
         accepted []]
    (if (empty? remaining)
      accepted
      (let [item (first remaining)
            seen? (some #(equal-values-in-container item %) accepted)]
        (recur (rest remaining)
               (if seen? accepted (conj accepted item)))))))

(defn partition-values [function-value values]
  (when-not (vector? values)
    (evaluation-failure! "type-error" {"operation" "partition"}))
  (loop [remaining values
         right []
         wrong []]
    (if (empty? remaining)
      (->AttrsetValue {"right" right "wrong" wrong})
      (let [item (first remaining)
            keep? (require-boolean (apply-value function-value item))]
        (if keep?
          (recur (rest remaining) (conj right item) wrong)
          (recur (rest remaining) right (conj wrong item)))))))

(defn zip-lists-with [function-value left right]
  (when-not (and (vector? left) (vector? right))
    (evaluation-failure! "type-error" {"operation" "zipListsWith"}))
  (mapv (fn [a b]
          (cell {:op :apply-values
                 :function function-value
                 :argument-cells [(if (instance? Cell a) a (value-cell a))
                                  (if (instance? Cell b) b (value-cell b))]}
                (atom {})))
        left
        right))

(defn foldl-values [function-value initial values]
  (when-not (vector? values)
    (evaluation-failure! "type-error" {"operation" "foldl"}))
  (loop [accumulator initial
         remaining values]
    (if (empty? remaining)
      accumulator
      (let [partial (apply-value function-value
                                 (if (instance? Cell accumulator)
                                   accumulator
                                   (value-cell accumulator)))
            next-value (apply-value partial (first remaining))]
        (recur next-value (rest remaining))))))

(defn foldr-values [function-value initial values]
  (when-not (vector? values)
    (evaluation-failure! "type-error" {"operation" "foldr"}))
  (loop [accumulator initial
         remaining (rseq (vec values))]
    (if (nil? remaining)
      accumulator
      (let [item (first remaining)
            next-value (apply-value2 function-value item accumulator)]
        (recur next-value (next remaining))))))

(defn pipe-values [initial functions]
  (when-not (vector? functions)
    (evaluation-failure! "type-error" {"operation" "pipe"}))
  (loop [accumulator initial
         remaining functions]
    (if (empty? remaining)
      accumulator
      (let [function-value (force-cell (first remaining))
            next-value (apply-value function-value
                                    (if (instance? Cell accumulator)
                                      accumulator
                                      (value-cell accumulator)))]
        (recur next-value (rest remaining))))))

(defn numeric-min-max [operation left right]
  (let [left (require-number left)
        right (require-number right)]
    (if (and (integer-value? left) (integer-value? right))
      (if (case operation
            :min (< left right)
            :max (> left right))
        left
        right)
      (let [left (as-double left)
            right (as-double right)]
        (case operation
          :min (js/Math.min left right)
          :max (js/Math.max left right))))))

(defn range-values [from to]
  (let [from (require-integer from)
        to (require-integer to)]
    (if (> from to)
      []
      (let [start (js/Number from)
            end (js/Number to)]
        (when (or (not (js/Number.isSafeInteger start))
                  (not (js/Number.isSafeInteger end))
                  (not= (js/BigInt start) from)
                  (not= (js/BigInt end) to))
          (evaluation-failure! "integer-overflow" {"operation" "range"}))
        (mapv (fn [n] (js/BigInt (str n)))
              (range start (inc end)))))))

(defn intersect-lists [left right]
  (when-not (and (vector? left) (vector? right))
    (evaluation-failure! "type-error" {"operation" "intersectLists"}))
  (vec (filter (fn [item]
                 (some #(equal-values-in-container item %) right))
               left)))

(defn subtract-lists [left right]
  (when-not (and (vector? left) (vector? right))
    (evaluation-failure! "type-error" {"operation" "subtractLists"}))
  (vec (remove (fn [item]
                 (some #(equal-values-in-container item %) right))
               left)))

(defn zip-lists [left right]
  (when-not (and (vector? left) (vector? right))
    (evaluation-failure! "type-error" {"operation" "zipLists"}))
  (mapv (fn [a b]
          (->AttrsetValue {"fst" (if (instance? Cell a) a (value-cell a))
                           "snd" (if (instance? Cell b) b (value-cell b))}))
        left
        right))

(defn map-attrs-to-list [function-value attrs]
  (when-not (instance? AttrsetValue attrs)
    (evaluation-failure! "type-error" {"operation" "mapAttrsToList"}))
  (mapv (fn [name]
          (cell {:op :apply-values
                 :function function-value
                 :argument-cells [(value-cell name)
                                  (get (:fields attrs) name)]}
                (atom {})))
        (sorted-field-names (:fields attrs))))

(defn zip-attrs [attr-list]
  (when-not (vector? attr-list)
    (evaluation-failure! "type-error" {"operation" "zipAttrs"}))
  (let [attrs (mapv force-cell attr-list)]
    (when-not (every? #(instance? AttrsetValue %) attrs)
      (evaluation-failure! "type-error" {"operation" "zipAttrs"}))
    (let [names (set (mapcat #(keys (:fields %)) attrs))]
      (->AttrsetValue
       (into {}
             (map (fn [name]
                    [name
                     (vec (keep (fn [attr]
                                  (get (:fields attr) name))
                                attrs))])
                  names))))))

(defn filter-attrs-recursive [function-value attrs]
  (when-not (instance? AttrsetValue attrs)
    (evaluation-failure! "type-error" {"operation" "filterAttrsRecursive"}))
  (->AttrsetValue
   (into {}
         (keep (fn [[name attribute-cell]]
                 (when (require-boolean
                        (apply-value2 function-value name attribute-cell))
                   (let [value (force-cell attribute-cell)]
                     (if (instance? AttrsetValue value)
                       [name (value-cell (filter-attrs-recursive function-value value))]
                       [name attribute-cell]))))
               (:fields attrs)))))

(defn map-attrs-recursive [function-value attrs]
  (letfn [(walk [path current]
            (let [current (force-cell current)]
              (if (instance? AttrsetValue current)
                (->AttrsetValue
                 (into {}
                       (map (fn [[name attribute-cell]]
                              [name
                               (value-cell
                                (walk (conj path name) attribute-cell))])
                            (:fields current))))
                (apply-value2 function-value
                              (mapv identity path)
                              current))))]
    (when-not (instance? AttrsetValue attrs)
      (evaluation-failure! "type-error" {"operation" "mapAttrsRecursive"}))
    (walk [] attrs)))

(defn fix-value [function-value]
  (let [self-cell (->Cell nil nil (atom {:tag :evaluating}))
        result (apply-value function-value self-cell)]
    (reset! (:state self-cell) {:tag :evaluated :value result})
    result))

(defn update-many-attrs [updates base]
  (when-not (vector? updates)
    (evaluation-failure! "type-error" {"operation" "updateManyAttrs"}))
  (when-not (instance? AttrsetValue base)
    (evaluation-failure! "type-error" {"operation" "updateManyAttrs"}))
  (reduce (fn [acc update-cell]
            (recursive-update-value acc update-cell))
          base
          updates))

(defn get-name-value [value]
  (let [value (force-cell value)]
    (cond
      (string-value? value)
      (force-cell (get (:fields (parse-derivation-name (string-text value)))
                       "name"))
      (instance? AttrsetValue value)
      (let [fields (:fields value)]
        (cond
          (contains? fields "name")
          (let [name (force-cell (get fields "name"))]
            (if (string-value? name)
              (force-cell (get (:fields (parse-derivation-name (string-text name)))
                               "name"))
              (evaluation-failure! "type-error" {"operation" "getName"})))
          (contains? fields "pname")
          (force-cell (get fields "pname"))
          :else
          (evaluation-failure! "attribute-missing" {"name" "name"})))
      :else
      (evaluation-failure! "type-error" {"operation" "getName"}))))

(defn get-version-value [value]
  (let [value (force-cell value)]
    (cond
      (string-value? value)
      (force-cell (get (:fields (parse-derivation-name (string-text value)))
                       "version"))
      (instance? AttrsetValue value)
      (let [fields (:fields value)]
        (cond
          (contains? fields "version")
          (force-cell (get fields "version"))
          (contains? fields "name")
          (let [name (force-cell (get fields "name"))]
            (if (string-value? name)
              (force-cell (get (:fields (parse-derivation-name (string-text name)))
                               "version"))
              (evaluation-failure! "type-error" {"operation" "getVersion"})))
          :else
          (evaluation-failure! "attribute-missing" {"name" "version"})))
      :else
      (evaluation-failure! "type-error" {"operation" "getVersion"}))))

(defn concat-map-strings-sep [separator function-value values]
  (when-not (string-value? separator)
    (evaluation-failure! "type-error" {"operation" "concatMapStringsSep"}))
  (when-not (vector? values)
    (evaluation-failure! "type-error" {"operation" "concatMapStringsSep"}))
  (let [strings
        (mapv (fn [item]
                (nix-to-string (apply-value function-value item)))
              values)]
    (decode-byte-string
     (concatenate-byte-arrays
      (vec
       (mapcat (fn [[index value]]
                 (if (zero? index)
                   [(string-bytes value)]
                   [(string-bytes separator) (string-bytes value)]))
               (map-indexed vector strings)))))))


(defn invoke-builtin [builtin argument]
  (let [arguments (conj (:arguments builtin) argument)]
    (case (:operation builtin)
      :identity argument
      :parseDrvName (parse-derivation-name argument)
      :add (if (< (count arguments) 2)
             (->BuiltinValue :add arguments)
             (numeric-binary :add (nth arguments 0) (nth arguments 1)))
      :sub (if (< (count arguments) 2)
             (->BuiltinValue :sub arguments)
             (numeric-binary :subtract (nth arguments 0) (nth arguments 1)))
      :mul (if (< (count arguments) 2)
             (->BuiltinValue :mul arguments)
             (numeric-binary :multiply (nth arguments 0) (nth arguments 1)))
      :div (if (< (count arguments) 2)
             (->BuiltinValue :div arguments)
             (let [[left right] arguments]
               (numeric-binary :divide left right)))
      :lessThan (if (< (count arguments) 2)
                  (->BuiltinValue :lessThan arguments)
                  (ordered-less (nth arguments 0) (nth arguments 1)))
      :ceil (numeric-round :ceil argument)
      :floor (numeric-round :floor argument)
      :abs (numeric-abs argument)
      :throw (if (string-value? argument)
               (evaluation-failure! "explicit-throw" {"message" (string-text argument)})
               (evaluation-failure! "type-error"
                                    {"operation" "throw"}))
      :abort (if (string-value? argument)
               (evaluation-failure! "abort" {"message" (string-text argument)})
               (evaluation-failure! "type-error"
                                    {"operation" "abort"}))
      :tryEval (try-eval-cell argument)
      :hashString
      (if (< (count arguments) 2)
        (->BuiltinValue :hashString arguments)
        (let [algorithm (nth arguments 0)
              payload-cell (nth arguments 1)]
          (when-not (and (string? algorithm)
                         (contains? hash-algorithms algorithm))
            (evaluation-failure! "invalid-hash-algorithm"
                                 {"algorithm" (if (string? algorithm)
                                                algorithm
                                                "non-string")}))
          (let [payload (force-cell payload-cell)]
            (when-not (string-value? payload)
              (evaluation-failure! "type-error"
                                   {"operation" "hashString"}))
            (hash-bytes algorithm (string-bytes payload)))))
      :isAttrs (instance? AttrsetValue argument)
      :isBool (boolean? argument)
      :isFloat (number? argument)
      :isFunction (or (instance? ClosureValue argument)
                      (instance? BuiltinValue argument))
      :isInt (integer-value? argument)
      :isList (vector? argument)
      :isNull (nil? argument)
      :isPath false
      :isString (string-value? argument)
      :typeOf (cond
                (integer-value? argument) "int"
                (number? argument) "float"
                (boolean? argument) "bool"
                (string-value? argument) "string"
                (nil? argument) "null"
                (vector? argument) "list"
                (instance? AttrsetValue argument) "set"
                (or (instance? ClosureValue argument)
                    (instance? BuiltinValue argument)) "lambda"
                :else (evaluation-failure! "type-error"
                                           {"operation" "typeOf"}))
      :toString (nix-to-string argument)
      :toPath (if (and (string? argument)
                       (.startsWith argument "/"))
                (normalize-absolute-path argument)
                (evaluation-failure! "type-error"
                                     {"operation" "toPath"}))
      :stringLength (if (string-value? argument)
                      (checked-integer
                       (js/BigInt
                        (str (.-length (string-bytes argument)))))
                      (evaluation-failure! "type-error"
                                           {"operation" "stringLength"}))
      :substring (if (< (count arguments) 3)
                   (->BuiltinValue :substring arguments)
                   (let [start (require-integer (nth arguments 0))
                         length (require-integer (nth arguments 1))
                         value (nth arguments 2)]
                     (when-not (string-value? value)
                       (evaluation-failure! "type-error"
                                            {"operation" "substring"}))
                     (when (< start (js/BigInt "0"))
                       (evaluation-failure! "type-error"
                                            {"operation" "substring"
                                             "detail_class" "negative-start"}))
                     (let [bytes (string-bytes value)
                           size (js/BigInt (str (.-length bytes)))
                           first-byte (if (> start size) size start)
                           end-byte (if (< length (js/BigInt "0"))
                                      size
                                      (let [candidate
                                            (integer-add start length)]
                                        (if (> candidate size)
                                          size
                                          candidate)))]
                       (decode-byte-string
                        (.slice bytes
                                (js/Number first-byte)
                                (js/Number end-byte))))))
      :concatStringsSep
      (if (< (count arguments) 2)
        (->BuiltinValue :concatStringsSep arguments)
        (let [separator (nth arguments 0)
              values (nth arguments 1)]
          (when-not (string-value? separator)
            (evaluation-failure! "type-error"
                                 {"operation" "concatStringsSep"}))
          (when-not (vector? values)
            (evaluation-failure! "type-error"
                                 {"operation" "concatStringsSep"}))
          (let [strings (mapv force-cell values)]
            (when-not (every? string-value? strings)
              (evaluation-failure! "type-error"
                                   {"operation" "concatStringsSep"}))
            (decode-byte-string
             (concatenate-byte-arrays
              (vec
               (mapcat (fn [[index value]]
                         (if (zero? index)
                           [(string-bytes value)]
                           [(string-bytes separator)
                            (string-bytes value)]))
                       (map-indexed vector strings))))))))
      :concatLists
      (if-not (vector? argument)
        (evaluation-failure! "type-error"
                             {"operation" "concatLists"})
        (reduce (fn [result list-cell]
                  (let [list-value (force-cell list-cell)]
                    (if (vector? list-value)
                      (into result list-value)
                      (evaluation-failure! "type-error"
                                           {"operation" "concatLists"}))))
                []
                argument))
      :match (if (< (count arguments) 2)
               (->BuiltinValue :match arguments)
               (match-value (nth arguments 0) (nth arguments 1)))
      :split (if (< (count arguments) 2)
               (->BuiltinValue :split arguments)
               (split-value (nth arguments 0) (nth arguments 1)))
      :replaceStrings
      (if (< (count arguments) 3)
        (->BuiltinValue :replaceStrings arguments)
        (replace-strings-value (nth arguments 0)
                               (nth arguments 1)
                               (nth arguments 2)))
      :fromJSON (from-json-value argument)
      :toJSON (to-json argument)
      :sort (if (< (count arguments) 2)
              (->BuiltinValue :sort arguments)
              (let [comparator (nth arguments 0)
                    values (nth arguments 1)]
                (when-not (vector? values)
                  (evaluation-failure! "type-error"
                                       {"operation" "sort"}))
                (stable-sort comparator values)))
      :genList (if (< (count arguments) 2)
                 (->BuiltinValue :genList arguments)
                 (let [function-value (nth arguments 0)
                       length (require-integer (nth arguments 1))]
                   (when (< length (js/BigInt "0"))
                     (evaluation-failure! "type-error"
                                          {"operation" "genList"}))
                   (let [numeric-length (js/Number length)]
                     (when (or (not (js/Number.isSafeInteger numeric-length))
                               (not= (js/BigInt numeric-length) length))
                       (evaluation-failure! "integer-overflow"
                                            {"operation" "genList"}))
                     (mapv (fn [index]
                             (let [argument-cell
                                   (cell {:op :integer
                                          :value (js/BigInt (str index))}
                                         (atom {}))]
                               (cell {:op :call-value
                                      :function function-value
                                      :argument-cell argument-cell}
                                     (atom {}))))
                           (range numeric-length)))))
      :length (if (vector? argument)
                (checked-integer (js/BigInt (str (count argument))))
                (evaluation-failure! "type-error"
                                     {"operation" "length"}))
      :head (if (and (vector? argument) (seq argument))
              (force-cell (first argument))
              (evaluation-failure! "type-error"
                                   {"operation" "head"}))
      :tail (if (and (vector? argument) (seq argument))
              (subvec argument 1)
              (evaluation-failure! "type-error"
                                   {"operation" "tail"}))
      :attrNames (if (instance? AttrsetValue argument)
                   (vec (sorted-field-names (:fields argument)))
                   (evaluation-failure! "type-error"
                                        {"operation" "attrNames"}))
      :attrValues (if (instance? AttrsetValue argument)
                    (mapv #(get (:fields argument) %)
                          (sorted-field-names (:fields argument)))
                    (evaluation-failure! "type-error"
                                         {"operation" "attrValues"}))
      :hasAttr (if (< (count arguments) 2)
                 (->BuiltinValue :hasAttr arguments)
                 (let [attribute (nth arguments 0)
                       attrs (nth arguments 1)]
                   (when-not (string? attribute)
                     (evaluation-failure! "type-error"
                                          {"operation" "hasAttr"}))
                   (if (instance? AttrsetValue attrs)
                     (contains? (:fields attrs) attribute)
                     (evaluation-failure! "type-error"
                                          {"operation" "hasAttr"}))))
      :getAttr (if (< (count arguments) 2)
                 (->BuiltinValue :getAttr arguments)
                 (let [attribute (nth arguments 0)
                       attrs (nth arguments 1)]
                   (when-not (string? attribute)
                     (evaluation-failure! "type-error"
                                          {"operation" "getAttr"}))
                   (if-not (instance? AttrsetValue attrs)
                     (evaluation-failure! "type-error"
                                          {"operation" "getAttr"})
                     (if (contains? (:fields attrs) attribute)
                       (force-cell (get (:fields attrs) attribute))
                       (evaluation-failure! "attribute-missing"
                                            {"name" attribute})))))
      :map (if (< (count arguments) 2)
             (->BuiltinValue :map arguments)
             (let [function-value (nth arguments 0)
                   values (nth arguments 1)]
               (if (vector? values)
                 (mapv (fn [argument-cell]
                         (cell {:op :call-value
                                :function function-value
                                :argument-cell argument-cell}
                               (atom {})))
                       values)
                 (evaluation-failure! "type-error"
                                      {"operation" "map"}))))
      :concatMap
      (if (< (count arguments) 2)
        (->BuiltinValue :concatMap arguments)
        (let [function-value (nth arguments 0)
              values (nth arguments 1)]
          (when-not (vector? values)
            (evaluation-failure! "type-error"
                                 {"operation" "concatMap"}))
          (reduce (fn [result argument-cell]
                    (let [mapped (apply-value function-value argument-cell)]
                      (if (vector? mapped)
                        (into result mapped)
                        (evaluation-failure! "type-error"
                                             {"operation" "concatMap"}))))
                  []
                  values)))
      :mapAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :mapAttrs arguments)
        (let [function-value (nth arguments 0)
              attrs (nth arguments 1)]
          (if-not (instance? AttrsetValue attrs)
            (evaluation-failure! "type-error"
                                 {"operation" "mapAttrs"})
            (->AttrsetValue
             (into {}
                   (map (fn [[name attribute-cell]]
                          [name
                           (cell {:op :apply-values
                                  :function function-value
                                  :argument-cells [(value-cell name)
                                                   attribute-cell]}
                                 (atom {}))])
                        (:fields attrs)))))))
      :zipAttrsWith
      (if (< (count arguments) 2)
        (->BuiltinValue :zipAttrsWith arguments)
        (let [function-value (nth arguments 0)
              attr-list (nth arguments 1)]
          (when-not (vector? attr-list)
            (evaluation-failure! "type-error"
                                 {"operation" "zipAttrsWith"}))
          (let [attrs (mapv force-cell attr-list)]
            (when-not (every? #(instance? AttrsetValue %) attrs)
              (evaluation-failure! "type-error"
                                   {"operation" "zipAttrsWith"}))
            (let [names (set (mapcat #(keys (:fields %)) attrs))]
              (->AttrsetValue
               (into {}
                     (map (fn [name]
                            (let [values
                                  (vec
                                   (keep (fn [attr]
                                           (get (:fields attr) name))
                                         attrs))]
                              [name
                               (cell {:op :apply-values
                                      :function function-value
                                      :argument-cells [(value-cell name)
                                                       (value-cell values)]}
                                     (atom {}))]))
                          names)))))))
      :all (if (< (count arguments) 2)
             (->BuiltinValue :all arguments)
             (let [function-value (nth arguments 0)
                   values (nth arguments 1)]
               (if-not (vector? values)
                 (evaluation-failure! "type-error"
                                      {"operation" "all"})
                 (loop [remaining values]
                   (if (empty? remaining)
                     true
                     (if (require-boolean
                          (evaluate-expression
                           {:op :call-value
                            :function function-value
                            :argument-cell (first remaining)}
                           {}))
                       (recur (rest remaining))
                       false))))))
      :any (if (< (count arguments) 2)
             (->BuiltinValue :any arguments)
             (let [function-value (nth arguments 0)
                   values (nth arguments 1)]
               (if-not (vector? values)
                 (evaluation-failure! "type-error"
                                      {"operation" "any"})
                 (loop [remaining values]
                   (if (empty? remaining)
                     false
                     (if (require-boolean
                          (apply-value function-value (first remaining)))
                       true
                       (recur (rest remaining))))))))
      :foldl' (if (< (count arguments) 3)
                (->BuiltinValue :foldl' arguments)
                (let [function-value (nth arguments 0)
                      initial (nth arguments 1)
                      values (nth arguments 2)]
                  (if-not (vector? values)
                    (evaluation-failure! "type-error"
                                         {"operation" "foldl'"})
                    (loop [accumulator initial
                           remaining values]
                      (if (empty? remaining)
                        accumulator
                        (let [partial
                              (evaluate-expression
                               {:op :call-value
                                :function function-value
                                :argument-cell
                                (->Cell nil nil
                                        (atom {:tag :evaluated
                                               :value accumulator}))}
                               {})
                              next-value
                              (evaluate-expression
                               {:op :call-value
                                :function partial
                                :argument-cell (first remaining)}
                               {})]
                          (recur next-value (rest remaining))))))))
      :filter (if (< (count arguments) 2)
                (->BuiltinValue :filter arguments)
                (let [function-value (nth arguments 0)
                      values (nth arguments 1)]
                  (if-not (vector? values)
                    (evaluation-failure! "type-error"
                                         {"operation" "filter"})
                    (loop [accepted []
                           remaining values]
                      (if (empty? remaining)
                        accepted
                        (let [argument-cell (first remaining)
                              keep? (require-boolean
                                     (evaluate-expression
                                      {:op :call-value
                                       :function function-value
                                       :argument-cell argument-cell}
                                      {}))]
                          (recur (if keep?
                                   (conj accepted argument-cell)
                                   accepted)
                                 (rest remaining))))))))
      :elem (if (< (count arguments) 2)
              (->BuiltinValue :elem arguments)
              (let [needle (nth arguments 0)
                    values (nth arguments 1)]
                (if-not (vector? values)
                  (evaluation-failure! "type-error"
                                       {"operation" "elem"})
                  (loop [remaining values]
                    (cond
                      (empty? remaining) false
                      (equal-values-in-container needle (first remaining)) true
                      :else (recur (rest remaining)))))))
      :elemAt (if (< (count arguments) 2)
                (->BuiltinValue :elemAt arguments)
                (let [values (nth arguments 0)
                      index (nth arguments 1)]
                  (when-not (vector? values)
                    (evaluation-failure! "type-error"
                                         {"operation" "elemAt"}))
                  (when-not (integer-value? index)
                    (evaluation-failure! "type-error"
                                         {"operation" "elemAt"}))
                  (let [size (js/BigInt (str (count values)))]
                    (if (or (< index (js/BigInt "0"))
                            (>= index size))
                      (evaluation-failure! "index-out-of-bounds"
                                           {"operation" "elemAt"})
                      (force-cell (nth values (js/Number index)))))))
      :listToAttrs (if-not (vector? argument)
                     (evaluation-failure! "type-error"
                                          {"operation" "listToAttrs"})
                     (->AttrsetValue
                      (reduce
                       (fn [fields row-cell]
                         (let [row (force-cell row-cell)]
                           (when-not (instance? AttrsetValue row)
                             (evaluation-failure! "type-error"
                                                  {"operation" "listToAttrs"}))
                           (when-not (and (contains? (:fields row) "name")
                                          (contains? (:fields row) "value"))
                             (evaluation-failure! "type-error"
                                                  {"operation" "listToAttrs"}))
                           (let [attribute
                                 (force-cell (get (:fields row) "name"))]
                             (when-not (string? attribute)
                               (evaluation-failure!
                                "type-error"
                                {"operation" "listToAttrs"}))
                             (if (contains? fields attribute)
                               fields
                               (assoc fields
                                      attribute
                                      (get (:fields row) "value"))))))
                       {}
                       argument)))

      :trace
      (if (< (count arguments) 2)
        (->BuiltinValue :trace arguments)
        (do
          (binding [*print-fn* *print-err-fn*]
            (println (str "trace: " (nix-to-string (nth arguments 0)))))
          (nth arguments 1)))

      :warn
      (if (< (count arguments) 2)
        (->BuiltinValue :warn arguments)
        (do
          (binding [*print-fn* *print-err-fn*]
            (println (str "warning: " (nix-to-string (nth arguments 0)))))
          (nth arguments 1)))

      :toXML (to-xml argument)

      :toFile
      (if (< (count arguments) 2)
        (->BuiltinValue :toFile arguments)
        (let [name (nth arguments 0)
              content (nth arguments 1)]
          (when-not (and (string-value? name) (string-value? content))
            (evaluation-failure! "type-error" {"operation" "toFile"}))
          (node-write-file (string-text name) (string-text content))))

      :readFile (node-read-file (path-string argument "readFile"))
      :readDir (node-read-dir (path-string argument "readDir"))
      :pathExists (node-path-exists (path-string argument "pathExists"))

      :fetchurl (node-fetch-url (fetch-url-argument argument))
      :fetchTarball (node-fetch-url (fetch-url-argument argument))
      :fetchGit
      (let [url (fetch-url-argument argument)
            rev (if (and (instance? AttrsetValue argument)
                         (contains? (:fields argument) "rev"))
                  (force-cell (get (:fields argument) "rev"))
                  "")
            out-path (node-fetch-url url)]
        (->AttrsetValue
         {"outPath" out-path
          "rev" (if (string-value? rev) (string-text rev) "")
          "shortRev" (let [r (if (string-value? rev) (string-text rev) "")]
                       (if (>= (count r) 7) (subs r 0 7) r))
          "revCount" (js/BigInt "0")
          "narHash" ""
          "submodules" false}))

      :getAttrFromPath
      (if (< (count arguments) 2)
        (->BuiltinValue :getAttrFromPath arguments)
        (get-attr-from-path (nth arguments 0) (nth arguments 1)))

      :hasAttrByPath
      (if (< (count arguments) 2)
        (->BuiltinValue :hasAttrByPath arguments)
        (has-attr-by-path (nth arguments 0) (nth arguments 1)))

      :attrByPath
      (if (< (count arguments) 3)
        (->BuiltinValue :attrByPath arguments)
        (attr-by-path (nth arguments 0) (nth arguments 1) (nth arguments 2)))

      :getAttrFromPathOr
      (if (< (count arguments) 3)
        (->BuiltinValue :getAttrFromPathOr arguments)
        (attr-by-path (nth arguments 1)
                      (nth arguments 2)
                      (nth arguments 0)))

      :filterAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :filterAttrs arguments)
        (filter-attrs-value (nth arguments 0) (nth arguments 1)))

      :removeAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :removeAttrs arguments)
        (remove-attrs-value (nth arguments 0) (nth arguments 1)))

      :intersectAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :intersectAttrs arguments)
        (intersect-attrs-value (nth arguments 0) (nth arguments 1)))

      :catAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :catAttrs arguments)
        (cat-attrs-value (nth arguments 0) (nth arguments 1)))

      :filterAttrsRecursive
      (if (< (count arguments) 2)
        (->BuiltinValue :filterAttrsRecursive arguments)
        (filter-attrs-recursive (nth arguments 0) (nth arguments 1)))

      :mapAttrsRecursive
      (if (< (count arguments) 2)
        (->BuiltinValue :mapAttrsRecursive arguments)
        (map-attrs-recursive (nth arguments 0) (nth arguments 1)))

      :mapAttrsToList
      (if (< (count arguments) 2)
        (->BuiltinValue :mapAttrsToList arguments)
        (map-attrs-to-list (nth arguments 0) (nth arguments 1)))

      :zipAttrs (zip-attrs argument)

      :last
      (if (and (vector? argument) (seq argument))
        (force-cell (peek argument))
        (evaluation-failure! "type-error" {"operation" "last"}))

      :init
      (if (and (vector? argument) (seq argument))
        (subvec argument 0 (dec (count argument)))
        (evaluation-failure! "type-error" {"operation" "init"}))

      :flatten (vec (flatten-value argument))

      :concatMapStringsSep
      (if (< (count arguments) 3)
        (->BuiltinValue :concatMapStringsSep arguments)
        (concat-map-strings-sep (nth arguments 0)
                                (nth arguments 1)
                                (nth arguments 2)))

      :removePrefix
      (if (< (count arguments) 2)
        (->BuiltinValue :removePrefix arguments)
        (let [prefix (require-string-arg (nth arguments 0) "removePrefix")
              value (require-string-arg (nth arguments 1) "removePrefix")
              prefix-bytes (string-bytes prefix)
              value-bytes (string-bytes value)]
          (if (bytes-has-prefix? value-bytes prefix-bytes)
            (decode-byte-string
             (.slice value-bytes (.-length prefix-bytes)))
            value)))

      :removeSuffix
      (if (< (count arguments) 2)
        (->BuiltinValue :removeSuffix arguments)
        (let [suffix (require-string-arg (nth arguments 0) "removeSuffix")
              value (require-string-arg (nth arguments 1) "removeSuffix")
              suffix-bytes (string-bytes suffix)
              value-bytes (string-bytes value)]
          (if (bytes-has-suffix? value-bytes suffix-bytes)
            (decode-byte-string
             (.slice value-bytes
                     0
                     (- (.-length value-bytes) (.-length suffix-bytes))))
            value)))

      :hasPrefix
      (if (< (count arguments) 2)
        (->BuiltinValue :hasPrefix arguments)
        (let [prefix (require-string-arg (nth arguments 0) "hasPrefix")
              value (require-string-arg (nth arguments 1) "hasPrefix")]
          (bytes-has-prefix? (string-bytes value) (string-bytes prefix))))

      :hasSuffix
      (if (< (count arguments) 2)
        (->BuiltinValue :hasSuffix arguments)
        (let [suffix (require-string-arg (nth arguments 0) "hasSuffix")
              value (require-string-arg (nth arguments 1) "hasSuffix")]
          (bytes-has-suffix? (string-bytes value) (string-bytes suffix))))

      :splitString
      (if (< (count arguments) 2)
        (->BuiltinValue :splitString arguments)
        (let [separator (require-string-arg (nth arguments 0) "splitString")
              value (require-string-arg (nth arguments 1) "splitString")]
          (split-string-value separator value)))

      :toLower
      (if (string-value? argument)
        (.toLowerCase (string-text argument))
        (evaluation-failure! "type-error" {"operation" "toLower"}))

      :toUpper
      (if (string-value? argument)
        (.toUpperCase (string-text argument))
        (evaluation-failure! "type-error" {"operation" "toUpper"}))

      :boolToString
      (if (boolean? argument)
        (if argument "true" "false")
        (evaluation-failure! "type-error" {"operation" "boolToString"}))

      :optional
      (if (< (count arguments) 2)
        (->BuiltinValue :optional arguments)
        (if (require-boolean (nth arguments 0))
          [(value-cell (nth arguments 1))]
          []))

      :optionals
      (if (< (count arguments) 2)
        (->BuiltinValue :optionals arguments)
        (if (require-boolean (nth arguments 0))
          (let [values (nth arguments 1)]
            (if (vector? values)
              values
              (evaluation-failure! "type-error" {"operation" "optionals"})))
          []))

      :optionalAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :optionalAttrs arguments)
        (if (require-boolean (nth arguments 0))
          (let [attrs (nth arguments 1)]
            (if (instance? AttrsetValue attrs)
              attrs
              (evaluation-failure! "type-error" {"operation" "optionalAttrs"})))
          (->AttrsetValue {})))

      :optionalString
      (if (< (count arguments) 2)
        (->BuiltinValue :optionalString arguments)
        (if (require-boolean (nth arguments 0))
          (nix-to-string (nth arguments 1))
          ""))

      :when
      (if (< (count arguments) 2)
        (->BuiltinValue :when arguments)
        (if (require-boolean (nth arguments 0))
          (nth arguments 1)
          nil))

      :implies
      (if (< (count arguments) 2)
        (->BuiltinValue :implies arguments)
        (if (require-boolean (nth arguments 0))
          (require-boolean (nth arguments 1))
          true))

      :id argument

      :const
      (if (< (count arguments) 2)
        (->BuiltinValue :const arguments)
        (nth arguments 0))

      :flip
      (if (< (count arguments) 3)
        (->BuiltinValue :flip arguments)
        (apply-value2 (nth arguments 0)
                      (nth arguments 2)
                      (nth arguments 1)))

      :pipe
      (if (< (count arguments) 2)
        (->BuiltinValue :pipe arguments)
        (pipe-values (nth arguments 0) (nth arguments 1)))

      :foldl
      (if (< (count arguments) 3)
        (->BuiltinValue :foldl arguments)
        (foldl-values (nth arguments 0) (nth arguments 1) (nth arguments 2)))

      :foldr
      (if (< (count arguments) 3)
        (->BuiltinValue :foldr arguments)
        (foldr-values (nth arguments 0) (nth arguments 1) (nth arguments 2)))

      :min
      (if (< (count arguments) 2)
        (->BuiltinValue :min arguments)
        (numeric-min-max :min (nth arguments 0) (nth arguments 1)))

      :max
      (if (< (count arguments) 2)
        (->BuiltinValue :max arguments)
        (numeric-min-max :max (nth arguments 0) (nth arguments 1)))

      :range
      (if (< (count arguments) 2)
        (->BuiltinValue :range arguments)
        (range-values (nth arguments 0) (nth arguments 1)))

      :unique (unique-values argument)

      :recursiveUpdate
      (if (< (count arguments) 2)
        (->BuiltinValue :recursiveUpdate arguments)
        (recursive-update-value (nth arguments 0) (nth arguments 1)))

      :updateManyAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :updateManyAttrs arguments)
        (update-many-attrs (nth arguments 0) (nth arguments 1)))

      :partition
      (if (< (count arguments) 2)
        (->BuiltinValue :partition arguments)
        (partition-values (nth arguments 0) (nth arguments 1)))

      :zipListsWith
      (if (< (count arguments) 3)
        (->BuiltinValue :zipListsWith arguments)
        (zip-lists-with (nth arguments 0) (nth arguments 1) (nth arguments 2)))

      :zipLists
      (if (< (count arguments) 2)
        (->BuiltinValue :zipLists arguments)
        (zip-lists (nth arguments 0) (nth arguments 1)))

      :intersectLists
      (if (< (count arguments) 2)
        (->BuiltinValue :intersectLists arguments)
        (intersect-lists (nth arguments 0) (nth arguments 1)))

      :subtractLists
      (if (< (count arguments) 2)
        (->BuiltinValue :subtractLists arguments)
        (subtract-lists (nth arguments 0) (nth arguments 1)))

      :sum
      (if-not (vector? argument)
        (evaluation-failure! "type-error" {"operation" "sum"})
        (foldl-values (->BuiltinValue :add [])
                      (js/BigInt "0")
                      argument))

      :product
      (if-not (vector? argument)
        (evaluation-failure! "type-error" {"operation" "product"})
        (foldl-values (->BuiltinValue :mul [])
                      (js/BigInt "1")
                      argument))

      :fix (fix-value argument)

      :getName (get-name-value argument)
      :getVersion (get-version-value argument)

      ;; ---- Extended builtins (maturity pass 2026-08-11) ----
      :pow
      (if (< (count arguments) 2)
        (->BuiltinValue :pow arguments)
        (let [a (nth arguments 0) b (nth arguments 1)
              result (js/Math.pow (as-double a) (as-double b))]
          (if (and (integer-value? a) (integer-value? b))
            (checked-integer (js/BigInt (.toFixed result 0)))
            result)))

      ;; Truncated remainder (C `%`, matches Nix builtins.mod), paired with
      ;; the truncating integer-divide above -- sign follows the dividend.
      :mod
      (if (< (count arguments) 2)
        (->BuiltinValue :mod arguments)
        (let [a (require-number (nth arguments 0))
              b (require-number (nth arguments 1))]
          (if (and (integer-value? a) (integer-value? b))
            (if (integer-zero? b)
              (evaluation-failure! "division-by-zero" {"operation" "mod"})
              (checked-integer (js* "(~{} % ~{})" a b)))
            (let [a (as-double a) b (as-double b)]
              (if (zero? b)
                (evaluation-failure! "division-by-zero" {"operation" "mod"})
                (- a (* b (js/Math.trunc (/ a b)))))))))

      :sqrt (js/Math.sqrt (as-double argument))
      :exp (js/Math.exp (as-double argument))
      :ln (js/Math.log (as-double argument))
      :sin (js/Math.sin (as-double argument))
      :cos (js/Math.cos (as-double argument))
      :atan2
      (if (< (count arguments) 2)
        (->BuiltinValue :atan2 arguments)
        (js/Math.atan2 (as-double (nth arguments 0)) (as-double (nth arguments 1))))

      :bitAnd
      (if (< (count arguments) 2)
        (->BuiltinValue :bitAnd arguments)
        (checked-integer (js* "(~{} & ~{})" (require-integer (nth arguments 0))
                             (require-integer (nth arguments 1)))))
      :bitOr
      (if (< (count arguments) 2)
        (->BuiltinValue :bitOr arguments)
        (checked-integer (js* "(~{} | ~{})" (require-integer (nth arguments 0))
                             (require-integer (nth arguments 1)))))
      :bitXor
      (if (< (count arguments) 2)
        (->BuiltinValue :bitXor arguments)
        (checked-integer (js* "(~{} ^ ~{})" (require-integer (nth arguments 0))
                             (require-integer (nth arguments 1)))))

      :and
      (if (< (count arguments) 2)
        (->BuiltinValue :and arguments)
        (boolean (and (require-boolean (nth arguments 0))
                      (require-boolean (nth arguments 1)))))
      :or
      (if (< (count arguments) 2)
        (->BuiltinValue :or arguments)
        (boolean (or (require-boolean (nth arguments 0))
                     (require-boolean (nth arguments 1)))))
      :not (not (require-boolean argument))
      :eq
      (if (< (count arguments) 2)
        (->BuiltinValue :eq arguments)
        (equal-values (nth arguments 0) (nth arguments 1)))
      :lt
      (if (< (count arguments) 2)
        (->BuiltinValue :lt arguments)
        (ordered-less (nth arguments 0) (nth arguments 1)))
      :le
      (if (< (count arguments) 2)
        (->BuiltinValue :le arguments)
        (not (ordered-less (nth arguments 1) (nth arguments 0))))
      :gt
      (if (< (count arguments) 2)
        (->BuiltinValue :gt arguments)
        (ordered-less (nth arguments 1) (nth arguments 0)))
      :ge
      (if (< (count arguments) 2)
        (->BuiltinValue :ge arguments)
        (not (ordered-less (nth arguments 0) (nth arguments 1))))
      :neg
      (if (integer-value? argument)
        (checked-integer (integer-negate (require-integer argument)))
        (- (as-double argument)))

      :get
      (if (< (count arguments) 2)
        (->BuiltinValue :get arguments)
        (let [attrs (nth arguments 0) attr (require-string-arg (nth arguments 1) "get")]
          (when-not (instance? AttrsetValue attrs)
            (evaluation-failure! "type-error" {"operation" "get"}))
          (when-not (contains? (:fields attrs) attr)
            (evaluation-failure! "attribute-missing" {"name" attr}))
          (force-cell (get (:fields attrs) attr))))
      :set
      (if (< (count arguments) 3)
        (->BuiltinValue :set arguments)
        (let [attrs (nth arguments 0) attr (require-string-arg (nth arguments 1) "set")]
          (when-not (instance? AttrsetValue attrs)
            (evaluation-failure! "type-error" {"operation" "set"}))
          (->AttrsetValue (assoc (:fields attrs) attr (nth arguments 2)))))
      :keys
      (if-not (instance? AttrsetValue argument)
        (evaluation-failure! "type-error" {"operation" "keys"})
        (vec (sorted-field-names (:fields argument))))
      :values
      (if-not (instance? AttrsetValue argument)
        (evaluation-failure! "type-error" {"operation" "values"})
        (mapv (fn [n] (force-cell (get (:fields argument) n)))
              (sorted-field-names (:fields argument))))
      :merge
      (if (< (count arguments) 2)
        (->BuiltinValue :merge arguments)
        (let [left (nth arguments 0) right (nth arguments 1)]
          (when-not (and (instance? AttrsetValue left) (instance? AttrsetValue right))
            (evaluation-failure! "type-error" {"operation" "merge"}))
          (->AttrsetValue (merge (:fields left) (:fields right)))))

      :genAttrs
      (if (< (count arguments) 2)
        (->BuiltinValue :genAttrs arguments)
        (let [names (nth arguments 0) f (nth arguments 1)]
          (when-not (vector? names)
            (evaluation-failure! "type-error" {"operation" "genAttrs"}))
          (->AttrsetValue
           (into {}
                 (map (fn [n]
                        (let [n (require-string-arg (force-cell n) "genAttrs")]
                          [n (value-cell (apply-value f (value-cell n)))])))
                 names))))

      :nameValuePair
      (if (< (count arguments) 2)
        (->BuiltinValue :nameValuePair arguments)
        (->AttrsetValue {"name" (value-cell (nth arguments 0)) "value" (nth arguments 1)}))

      :foldlAttrs
      (if (< (count arguments) 3)
        (->BuiltinValue :foldlAttrs arguments)
        (let [f (nth arguments 0) acc (nth arguments 1) attrs (nth arguments 2)]
          (when-not (instance? AttrsetValue attrs)
            (evaluation-failure! "type-error" {"operation" "foldlAttrs"}))
          (loop [remaining (sorted-field-names (:fields attrs)) acc acc]
            (if (empty? remaining)
              acc
              (recur (rest remaining)
                     (apply-value3 f acc (first remaining)
                                   (get (:fields attrs) (first remaining))))))))

      :unsafeGetAttrPos nil

      :seq
      (if (< (count arguments) 2)
        (->BuiltinValue :seq arguments)
        (do (force-cell (nth arguments 0)) (nth arguments 1)))
      :deepSeq
      (letfn [(deep! [v]
                (let [v (force-cell v)]
                  (cond
                    (vector? v) (run! deep! v)
                    (instance? AttrsetValue v) (run! deep! (vals (:fields v))))
                  v))]
        (if (< (count arguments) 2)
          (->BuiltinValue :deepSeq arguments)
          (do (deep! (nth arguments 0)) (nth arguments 1))))

      :drop
      (if (< (count arguments) 2)
        (->BuiltinValue :drop arguments)
        (let [n (require-integer (nth arguments 0)) xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "drop"}))
          (vec (drop (js/Number n) xs))))
      :take
      (if (< (count arguments) 2)
        (->BuiltinValue :take arguments)
        (let [n (require-integer (nth arguments 0)) xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "take"}))
          (vec (take (js/Number n) xs))))
      :cons
      (if (< (count arguments) 2)
        (->BuiltinValue :cons arguments)
        (let [xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "cons"}))
          (vec (cons (value-cell (nth arguments 0)) xs))))
      :append
      (if (< (count arguments) 2)
        (->BuiltinValue :append arguments)
        (let [left (nth arguments 0) right (nth arguments 1)]
          (when-not (and (vector? left) (vector? right))
            (evaluation-failure! "type-error" {"operation" "append"}))
          (vec (concat left right))))
      :zip
      ;; [a b] pairs (distinct from zipLists, which returns {fst=a; snd=b;}).
      (if (< (count arguments) 2)
        (->BuiltinValue :zip arguments)
        (let [left (nth arguments 0) right (nth arguments 1)
              as-cell (fn [v] (if (instance? Cell v) v (value-cell v)))]
          (when-not (and (vector? left) (vector? right))
            (evaluation-failure! "type-error" {"operation" "zip"}))
          (mapv (fn [a b] [(as-cell a) (as-cell b)]) left right)))
      :reverseList
      (if-not (vector? argument)
        (evaluation-failure! "type-error" {"operation" "reverseList"})
        (vec (reverse argument)))
      :replicate
      (if (< (count arguments) 2)
        (->BuiltinValue :replicate arguments)
        (let [n (require-integer (nth arguments 0))]
          (vec (repeat (js/Number n) (nth arguments 1)))))
      :findFirst
      (if (< (count arguments) 3)
        (->BuiltinValue :findFirst arguments)
        (let [pred (nth arguments 0) default (nth arguments 1) xs (nth arguments 2)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "findFirst"}))
          (loop [remaining xs]
            (if (empty? remaining)
              default
              (if (require-boolean (apply-value pred (first remaining)))
                (force-cell (first remaining))
                (recur (rest remaining)))))))
      :find
      (if (< (count arguments) 2)
        (->BuiltinValue :find arguments)
        (let [needle (nth arguments 0) xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "find"}))
          (loop [remaining xs]
            (if (empty? remaining)
              nil
              (if (equal-values-in-container needle (first remaining))
                (force-cell (first remaining))
                (recur (rest remaining)))))))
      :imap0
      (if (< (count arguments) 2)
        (->BuiltinValue :imap0 arguments)
        (let [f (nth arguments 0) xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "imap0"}))
          (mapv (fn [i x] (value-cell (apply-value2 f (js/BigInt i) x))) (range) xs)))
      :imap1
      (if (< (count arguments) 2)
        (->BuiltinValue :imap1 arguments)
        (let [f (nth arguments 0) xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "imap1"}))
          (mapv (fn [i x] (value-cell (apply-value2 f (js/BigInt i) x)))
                (iterate inc 1) xs)))
      :stringToCharacters
      (let [s (require-string-arg argument "stringToCharacters")]
        (mapv str (string-text s)))
      :groupBy
      (if (< (count arguments) 2)
        (->BuiltinValue :groupBy arguments)
        (let [f (nth arguments 0) xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "groupBy"}))
          (->AttrsetValue
           (into {}
                 (map (fn [[k vs]] [k (vec vs)]))
                 (group-by (fn [x] (require-string-arg (apply-value f x) "groupBy")) xs)))))
      :functionArgs
      (if (and (instance? ClosureValue argument) (map? (:parameter argument)))
        (->AttrsetValue
         (into {}
               (map (fn [field] [(:name field) (boolean (contains? field :default))]))
               (:fields (:parameter argument))))
        (->AttrsetValue {}))

      :compareVersions
      (if (< (count arguments) 2)
        (->BuiltinValue :compareVersions arguments)
        (let [a (string-text (require-string-arg (nth arguments 0) "compareVersions"))
              b (string-text (require-string-arg (nth arguments 1) "compareVersions"))
              parts (fn [s] (str/split s #"[.\-]"))
              cmp1 (fn [x y]
                     (if (and (re-matches #"\d+" x) (re-matches #"\d+" y))
                       (compare (js/parseInt x) (js/parseInt y))
                       (compare x y)))]
          (js/BigInt
           (loop [xs (parts a) ys (parts b)]
             (cond
               (and (empty? xs) (empty? ys)) 0
               (empty? xs) -1
               (empty? ys) 1
               :else (let [c (cmp1 (first xs) (first ys))]
                       (if (zero? c) (recur (rest xs) (rest ys)) c)))))))
      :splitVersion
      (vec (str/split
            (string-text (require-string-arg argument "splitVersion")) #"[.\-]"))
      :dirOf
      (let [s (string-text (require-string-arg argument "dirOf"))
            i (str/last-index-of s "/")]
        (cond (nil? i) "." (zero? i) "/" :else (subs s 0 i)))
      :baseNameOf
      (last (str/split (string-text (require-string-arg argument "baseNameOf")) #"/"))
      :toInt
      (let [s (str/trim (string-text (require-string-arg argument "toInt")))]
        (if (re-matches #"-?\d+" s)
          (js/BigInt s)
          (evaluation-failure! "type-error" {"operation" "toInt"})))
      :hasInfix
      (if (< (count arguments) 2)
        (->BuiltinValue :hasInfix arguments)
        (str/includes?
         (string-text (require-string-arg (nth arguments 1) "hasInfix"))
         (string-text (require-string-arg (nth arguments 0) "hasInfix"))))
      :concatMapStrings
      (if (< (count arguments) 2)
        (->BuiltinValue :concatMapStrings arguments)
        (let [f (nth arguments 0) xs (nth arguments 1)]
          (when-not (vector? xs)
            (evaluation-failure! "type-error" {"operation" "concatMapStrings"}))
          (decode-byte-string
           (concatenate-byte-arrays
            (mapv (fn [x] (string-bytes (nix-to-string (apply-value f x)))) xs)))))
      :concatStrings
      (if-not (vector? argument)
        (evaluation-failure! "type-error" {"operation" "concatStrings"})
        (decode-byte-string
         (concatenate-byte-arrays
          (mapv (fn [x] (string-bytes (nix-to-string (force-cell x)))) argument))))

      :placeholder
      ;; Deterministic context-free placeholder for an output name, replaced
      ;; at build time in real Nix. Pseudo hash, not byte-compatible with Nix.
      (let [output (string-text (require-string-arg argument "placeholder"))
            hex (hash-bytes "sha256" (.encode utf8-encoder (str "pnix-output:" output)))]
        (str "/" (subs hex 0 32)))
      :storePath
      (evaluation-failure! "type-error" {"operation" "storePath" "reason" "pure-evaluator-no-store"})
      :pnixMounts
      (evaluation-failure! "type-error" {"operation" "pnixMounts" "nix-builtin?" false})
      :addErrorContext
      (if (< (count arguments) 2)
        (->BuiltinValue :addErrorContext arguments)
        (nth arguments 1))

      :genericClosure
      (let [attrs argument]
        (when-not (instance? AttrsetValue attrs)
          (evaluation-failure! "type-error" {"operation" "genericClosure"}))
        (let [operator (force-cell (get (:fields attrs) "operator"))
              start-set (force-cell (get (:fields attrs) "startSet"))]
          (when-not (vector? start-set)
            (evaluation-failure! "type-error" {"operation" "genericClosure"}))
          (loop [worklist (vec start-set) seen #{} result []]
            (if (empty? worklist)
              result
              (let [item (force-cell (first worklist))
                    key (force-cell (get (:fields item) "key"))]
                (if (contains? seen key)
                  (recur (vec (rest worklist)) seen result)
                  (let [next-items (force-cell (apply-value operator (value-cell item)))]
                    (when-not (vector? next-items)
                      (evaluation-failure! "type-error" {"operation" "genericClosure"}))
                    (recur (into (vec (rest worklist)) next-items)
                           (conj seen key)
                           (conj result item)))))))))

      (evaluation-failure! "unknown-builtin"
                           {"operation" (name (:operation builtin))}))))

(defn invoke-builtin-cell [builtin argument-cell]
  (case (:operation builtin)
    :tryEval (invoke-builtin builtin argument-cell)
    :hashString (if (empty? (:arguments builtin))
                  (invoke-builtin builtin (force-cell argument-cell))
                  (invoke-builtin builtin argument-cell))
    (invoke-builtin builtin (force-cell argument-cell))))

(defn builtins-value []
  (let [self-cell (->Cell nil nil (atom {:tag :evaluating}))
        value (->AttrsetValue
               {"abort" (->BuiltinValue :abort [])
                "abs" (->BuiltinValue :abs [])
                "add" (->BuiltinValue :add [])
                "any" (->BuiltinValue :any [])
                "attrByPath" (->BuiltinValue :attrByPath [])
                "attrNames" (->BuiltinValue :attrNames [])
                "attrValues" (->BuiltinValue :attrValues [])
                "all" (->BuiltinValue :all [])
                "boolToString" (->BuiltinValue :boolToString [])
                "builtins" self-cell
                "break" (->BuiltinValue :identity [])
                "catAttrs" (->BuiltinValue :catAttrs [])
                "ceil" (->BuiltinValue :ceil [])
                "concatLists" (->BuiltinValue :concatLists [])
                "concatMap" (->BuiltinValue :concatMap [])
                "concatMapStringsSep" (->BuiltinValue :concatMapStringsSep [])
                "concatStringsSep" (->BuiltinValue :concatStringsSep [])
                "const" (->BuiltinValue :const [])
                "div" (->BuiltinValue :div [])
                "elem" (->BuiltinValue :elem [])
                "elemAt" (->BuiltinValue :elemAt [])
                "fetchGit" (->BuiltinValue :fetchGit [])
                "fetchTarball" (->BuiltinValue :fetchTarball [])
                "fetchurl" (->BuiltinValue :fetchurl [])
                "filter" (->BuiltinValue :filter [])
                "filterAttrs" (->BuiltinValue :filterAttrs [])
                "fix" (->BuiltinValue :fix [])
                "flatten" (->BuiltinValue :flatten [])
                "flip" (->BuiltinValue :flip [])
                "floor" (->BuiltinValue :floor [])
                "foldl" (->BuiltinValue :foldl [])
                "foldl'" (->BuiltinValue :foldl' [])
                "foldr" (->BuiltinValue :foldr [])
                "fromJSON" (->BuiltinValue :fromJSON [])
                "genList" (->BuiltinValue :genList [])
                "getAttr" (->BuiltinValue :getAttr [])
                "getAttrFromPath" (->BuiltinValue :getAttrFromPath [])
                "getName" (->BuiltinValue :getName [])
                "getVersion" (->BuiltinValue :getVersion [])
                "hasAttr" (->BuiltinValue :hasAttr [])
                "hasAttrByPath" (->BuiltinValue :hasAttrByPath [])
                "hasPrefix" (->BuiltinValue :hasPrefix [])
                "hasSuffix" (->BuiltinValue :hasSuffix [])
                "hashString" (->BuiltinValue :hashString [])
                "head" (->BuiltinValue :head [])
                "id" (->BuiltinValue :id [])
                "init" (->BuiltinValue :init [])
                "intersectAttrs" (->BuiltinValue :intersectAttrs [])
                "intersectLists" (->BuiltinValue :intersectLists [])
                "isAttrs" (->BuiltinValue :isAttrs [])
                "isBool" (->BuiltinValue :isBool [])
                "isFloat" (->BuiltinValue :isFloat [])
                "isFunction" (->BuiltinValue :isFunction [])
                "isInt" (->BuiltinValue :isInt [])
                "isList" (->BuiltinValue :isList [])
                "isNull" (->BuiltinValue :isNull [])
                "isPath" (->BuiltinValue :isPath [])
                "isString" (->BuiltinValue :isString [])
                "langVersion" (js/BigInt "6")
                "last" (->BuiltinValue :last [])
                "length" (->BuiltinValue :length [])
                "lessThan" (->BuiltinValue :lessThan [])
                "listToAttrs" (->BuiltinValue :listToAttrs [])
                "map" (->BuiltinValue :map [])
                "mapAttrs" (->BuiltinValue :mapAttrs [])
                "mapAttrsRecursive" (->BuiltinValue :mapAttrsRecursive [])
                "mapAttrsToList" (->BuiltinValue :mapAttrsToList [])
                "match" (->BuiltinValue :match [])
                "max" (->BuiltinValue :max [])
                "min" (->BuiltinValue :min [])
                "mul" (->BuiltinValue :mul [])
                "nixVersion" "2.34.7"
                "null" nil
                "optional" (->BuiltinValue :optional [])
                "optionalAttrs" (->BuiltinValue :optionalAttrs [])
                "optionals" (->BuiltinValue :optionals [])
                "optionalString" (->BuiltinValue :optionalString [])
                "parseDrvName" (->BuiltinValue :parseDrvName [])
                "partition" (->BuiltinValue :partition [])
                "pathExists" (->BuiltinValue :pathExists [])
                "pipe" (->BuiltinValue :pipe [])
                "product" (->BuiltinValue :product [])
                "range" (->BuiltinValue :range [])
                "readDir" (->BuiltinValue :readDir [])
                "readFile" (->BuiltinValue :readFile [])
                "recursiveUpdate" (->BuiltinValue :recursiveUpdate [])
                "removeAttrs" (->BuiltinValue :removeAttrs [])
                "removePrefix" (->BuiltinValue :removePrefix [])
                "removeSuffix" (->BuiltinValue :removeSuffix [])
                "stringLength" (->BuiltinValue :stringLength [])
                "substring" (->BuiltinValue :substring [])
                "sort" (->BuiltinValue :sort [])
                "split" (->BuiltinValue :split [])
                "splitString" (->BuiltinValue :splitString [])
                "storeDir" "/nix/store"
                "sub" (->BuiltinValue :sub [])
                "subtractLists" (->BuiltinValue :subtractLists [])
                "sum" (->BuiltinValue :sum [])
                "replaceStrings" (->BuiltinValue :replaceStrings [])
                "tail" (->BuiltinValue :tail [])
                "toFile" (->BuiltinValue :toFile [])
                "toJSON" (->BuiltinValue :toJSON [])
                "toLower" (->BuiltinValue :toLower [])
                "toPath" (->BuiltinValue :toPath [])
                "toString" (->BuiltinValue :toString [])
                "toUpper" (->BuiltinValue :toUpper [])
                "toXML" (->BuiltinValue :toXML [])
                "throw" (->BuiltinValue :throw [])
                "trace" (->BuiltinValue :trace [])
                "true" true
                "tryEval" (->BuiltinValue :tryEval [])
                "typeOf" (->BuiltinValue :typeOf [])
                "unique" (->BuiltinValue :unique [])
                "unsafeDiscardOutputDependency" (->BuiltinValue :identity [])
                "unsafeDiscardStringContext" (->BuiltinValue :identity [])
                "updateManyAttrs" (->BuiltinValue :updateManyAttrs [])
                "warn" (->BuiltinValue :warn [])
                "when" (->BuiltinValue :when [])
                "implies" (->BuiltinValue :implies [])
                "false" false
                "zipAttrs" (->BuiltinValue :zipAttrs [])
                "zipAttrsWith" (->BuiltinValue :zipAttrsWith [])
                "zipLists" (->BuiltinValue :zipLists [])
                "zipListsWith" (->BuiltinValue :zipListsWith [])
                "filterAttrsRecursive" (->BuiltinValue :filterAttrsRecursive [])
                "getAttrFromPathOr" (->BuiltinValue :getAttrFromPathOr [])

                ;; ---- Extended builtins (maturity pass 2026-08-11) ----
                "pow" (->BuiltinValue :pow [])
                "mod" (->BuiltinValue :mod [])
                "sqrt" (->BuiltinValue :sqrt [])
                "exp" (->BuiltinValue :exp [])
                "ln" (->BuiltinValue :ln [])
                "sin" (->BuiltinValue :sin [])
                "cos" (->BuiltinValue :cos [])
                "atan2" (->BuiltinValue :atan2 [])
                "bitAnd" (->BuiltinValue :bitAnd [])
                "bitOr" (->BuiltinValue :bitOr [])
                "bitXor" (->BuiltinValue :bitXor [])
                "and" (->BuiltinValue :and [])
                "or" (->BuiltinValue :or [])
                "not" (->BuiltinValue :not [])
                "eq" (->BuiltinValue :eq [])
                "lt" (->BuiltinValue :lt [])
                "le" (->BuiltinValue :le [])
                "gt" (->BuiltinValue :gt [])
                "ge" (->BuiltinValue :ge [])
                "neg" (->BuiltinValue :neg [])
                "get" (->BuiltinValue :get [])
                "set" (->BuiltinValue :set [])
                "keys" (->BuiltinValue :keys [])
                "values" (->BuiltinValue :values [])
                "merge" (->BuiltinValue :merge [])
                "genAttrs" (->BuiltinValue :genAttrs [])
                "nameValuePair" (->BuiltinValue :nameValuePair [])
                "foldlAttrs" (->BuiltinValue :foldlAttrs [])
                "unsafeGetAttrPos" (->BuiltinValue :unsafeGetAttrPos [])
                "seq" (->BuiltinValue :seq [])
                "deepSeq" (->BuiltinValue :deepSeq [])
                "drop" (->BuiltinValue :drop [])
                "take" (->BuiltinValue :take [])
                "cons" (->BuiltinValue :cons [])
                "append" (->BuiltinValue :append [])
                "zip" (->BuiltinValue :zip [])
                "reverseList" (->BuiltinValue :reverseList [])
                "replicate" (->BuiltinValue :replicate [])
                "findFirst" (->BuiltinValue :findFirst [])
                "find" (->BuiltinValue :find [])
                "imap0" (->BuiltinValue :imap0 [])
                "imap1" (->BuiltinValue :imap1 [])
                "stringToCharacters" (->BuiltinValue :stringToCharacters [])
                "groupBy" (->BuiltinValue :groupBy [])
                "functionArgs" (->BuiltinValue :functionArgs [])
                "compareVersions" (->BuiltinValue :compareVersions [])
                "splitVersion" (->BuiltinValue :splitVersion [])
                "dirOf" (->BuiltinValue :dirOf [])
                "baseNameOf" (->BuiltinValue :baseNameOf [])
                "toInt" (->BuiltinValue :toInt [])
                "hasInfix" (->BuiltinValue :hasInfix [])
                "concatMapStrings" (->BuiltinValue :concatMapStrings [])
                "concatStrings" (->BuiltinValue :concatStrings [])
                "placeholder" (->BuiltinValue :placeholder [])
                "storePath" (->BuiltinValue :storePath [])
                "pnixMounts" (->BuiltinValue :pnixMounts [])
                "addErrorContext" (->BuiltinValue :addErrorContext [])
                "genericClosure" (->BuiltinValue :genericClosure [])})]
    (reset! (:state self-cell) {:tag :evaluated :value value})
    value))

(defn make-lib [builtins]
  (let [fields (:fields builtins)
        attrsets (->AttrsetValue
                  {"isAttrs" (get fields "isAttrs")
                   "attrNames" (get fields "attrNames")
                   "attrValues" (get fields "attrValues")
                   "hasAttr" (get fields "hasAttr")
                   "getAttr" (get fields "getAttr")
                   "getAttrFromPath" (get fields "getAttrFromPath")
                   "getAttrFromPathOr" (get fields "getAttrFromPathOr")
                   "hasAttrByPath" (get fields "hasAttrByPath")
                   "attrByPath" (get fields "attrByPath")
                   "mapAttrs" (get fields "mapAttrs")
                   "mapAttrsToList" (get fields "mapAttrsToList")
                   "mapAttrsRecursive" (get fields "mapAttrsRecursive")
                   "filterAttrs" (get fields "filterAttrs")
                   "filterAttrsRecursive" (get fields "filterAttrsRecursive")
                   "recursiveUpdate" (get fields "recursiveUpdate")
                   "zipAttrs" (get fields "zipAttrs")
                   "zipAttrsWith" (get fields "zipAttrsWith")
                   "optionalAttrs" (get fields "optionalAttrs")
                   "listToAttrs" (get fields "listToAttrs")})
        lists (->AttrsetValue
               {"flatten" (get fields "flatten")
                "unique" (get fields "unique")
                "intersectLists" (get fields "intersectLists")
                "subtractLists" (get fields "subtractLists")
                "zipLists" (get fields "zipLists")
                "zipListsWith" (get fields "zipListsWith")
                "partition" (get fields "partition")
                "range" (get fields "range")
                "last" (get fields "last")
                "init" (get fields "init")
                "foldl" (get fields "foldl")
                "foldr" (get fields "foldr")
                "sum" (get fields "sum")
                "product" (get fields "product")})
        strings (->AttrsetValue
                 {"hasPrefix" (get fields "hasPrefix")
                  "hasSuffix" (get fields "hasSuffix")
                  "removePrefix" (get fields "removePrefix")
                  "removeSuffix" (get fields "removeSuffix")
                  "splitString" (get fields "splitString")
                  "toLower" (get fields "toLower")
                  "toUpper" (get fields "toUpper")
                  "concatMapStringsSep" (get fields "concatMapStringsSep")
                  "concatStringsSep" (get fields "concatStringsSep")
                  "optionalString" (get fields "optionalString")})
        extras {"attrsets" attrsets
                "lists" lists
                "strings" strings
                "implies" (get fields "implies")
                "optionalAttrs" (get fields "optionalAttrs")
                "when" (get fields "when")
                "const" (get fields "const")
                "fix" (get fields "fix")
                "sum" (get fields "sum")
                "product" (get fields "product")
                "updateManyAttrs" (get fields "updateManyAttrs")
                "getName" (get fields "getName")
                "getVersion" (get fields "getVersion")
                "getAttrFromPathOr" (get fields "getAttrFromPathOr")
                "filterAttrsRecursive" (get fields "filterAttrsRecursive")
                "mapAttrsRecursive" (get fields "mapAttrsRecursive")
                "intersectLists" (get fields "intersectLists")
                "subtractLists" (get fields "subtractLists")
                "zipLists" (get fields "zipLists")
                "id" (get fields "id")
                "flip" (get fields "flip")
                "pipe" (get fields "pipe")
                "min" (get fields "min")
                "max" (get fields "max")
                "optional" (get fields "optional")
                "optionals" (get fields "optionals")
                "filterAttrs" (get fields "filterAttrs")
                "flatten" (get fields "flatten")
                "last" (get fields "last")
                "init" (get fields "init")
                "unique" (get fields "unique")
                "recursiveUpdate" (get fields "recursiveUpdate")
                "mapAttrsToList" (get fields "mapAttrsToList")
                "zipAttrs" (get fields "zipAttrs")
                "hasPrefix" (get fields "hasPrefix")
                "hasSuffix" (get fields "hasSuffix")
                "removePrefix" (get fields "removePrefix")
                "removeSuffix" (get fields "removeSuffix")
                "splitString" (get fields "splitString")
                "toLower" (get fields "toLower")
                "toUpper" (get fields "toUpper")
                "boolToString" (get fields "boolToString")
                "concatMapStringsSep" (get fields "concatMapStringsSep")
                "foldl" (get fields "foldl")
                "foldr" (get fields "foldr")
                "range" (get fields "range")
                "partition" (get fields "partition")
                "zipListsWith" (get fields "zipListsWith")
                "isAttrs" (get fields "isAttrs")}]
    (->AttrsetValue (merge fields extras))))

(defn builtin-environment []
  (let [builtins (builtins-value)
        fields (:fields builtins)
        lib (make-lib builtins)]
    {"builtins" builtins
     "lib" lib
     "map" (get fields "map")
     "throw" (get fields "throw")
     "toString" (get fields "toString")}))

(defn load-module [requested-path environment]
  (let [context (get environment module-context-key)]
    (when-not (and (map? context)
                   (string? (:source-id context))
                   (fn? (:load-source context))
                   (some? (:cache context)))
      (evaluation-failure! "import-loader-unavailable"
                           {"path" requested-path}))
    (let [loaded (try
                   ((:load-source context)
                    (:source-id context)
                    requested-path)
                   (catch :default _
                     (evaluation-failure! "import-error"
                                          {"path" requested-path})))
          module-id (:source-id loaded)
          source (:source loaded)]
      (when-not (and (string? module-id) (string? source))
        (evaluation-failure! "invalid-import-source"
                             {"path" requested-path}))
      (let [cache (:cache context)
            snapshot (get @cache module-id)]
        (if (instance? Cell snapshot)
          (force-cell snapshot)
          (let [module-context (assoc context :source-id module-id)
                module-environment (assoc (builtin-environment)
                                          module-context-key
                                          module-context)
                module-cell (cell (parser/parse source)
                                  (atom module-environment))]
            (swap! cache assoc module-id module-cell)
            (force-cell module-cell)))))))

(defn referenced-cell [value]
  (loop [current value
         seen #{}]
    (if (and (instance? Cell current)
             (not (contains? seen current))
             (= :variable (:op (:expression current))))
      (let [environment (:environment current)
            environment (if (map? environment)
                          environment
                          (when environment @environment))
            target (get environment (:name (:expression current)))]
        (if (instance? Cell target)
          (recur target (conj seen current))
          current))
      current)))

(defn equal-values* [left right container?]
  (let [same-retained-cell?
        (and container?
             (instance? Cell left)
             (instance? Cell right)
             (identical? (referenced-cell left) (referenced-cell right)))
        left (force-cell left)
        right (force-cell right)]
    (if same-retained-cell?
      true
      (cond
        (and (string-value? left) (string-value? right))
        (equal-bytes? (string-bytes left) (string-bytes right))

        (or (string-value? left) (string-value? right))
        false

        (and (numeric-value? left) (numeric-value? right))
        (if (or (number? left) (number? right))
          (= (as-double left) (as-double right))
          (= left right))

        (or (instance? ClosureValue left)
            (instance? ClosureValue right)
            (instance? BuiltinValue left)
            (instance? BuiltinValue right))
        (and container? (identical? left right))

        (and (instance? AttrsetValue left)
             (instance? AttrsetValue right))
        (let [left-names (sorted-field-names (:fields left))
              right-names (sorted-field-names (:fields right))]
          (and (= left-names right-names)
               (every? true?
                       (map (fn [name]
                              (equal-values-in-container
                               (get (:fields left) name)
                               (get (:fields right) name)))
                            left-names))))

        (or (instance? AttrsetValue left)
            (instance? AttrsetValue right))
        false

        (and (vector? left) (vector? right))
        (and (= (count left) (count right))
             (every? true?
                     (map equal-values-in-container left right)))

        (or (vector? left) (vector? right))
        false

        :else (= left right)))))

(defn equal-values-in-container [left right]
  (equal-values* left right true))

(defn equal-values [left right]
  (equal-values* left right false))

(defn ordered-less [left right]
  (cond
    (equal-values-in-container left right) false

    :else
    (let [left (force-cell left)
          right (force-cell right)]
      (cond
        (and (string-value? left) (string-value? right))
        (< (compare-bytes (string-bytes left) (string-bytes right)) 0)

        (and (numeric-value? left) (numeric-value? right))
        (if (or (number? left) (number? right))
          (< (as-double left) (as-double right))
          (< left right))

        (and (vector? left) (vector? right))
        (loop [index 0]
          (cond
            (= index (count left)) (< index (count right))
            (= index (count right)) false
            (equal-values-in-container (nth left index) (nth right index))
            (recur (inc index))
            :else (ordered-less (nth left index) (nth right index))))

        :else
        (evaluation-failure! "type-error"
                             {"operation" "ordering"})))))

(defn evaluate-binary [operator left-expression right-expression environment]
  (case operator
    :and (let [left (require-boolean
                      (evaluate-expression left-expression environment))]
           (if left
             (require-boolean
               (evaluate-expression right-expression environment))
             false))

    :or (let [left (require-boolean
                     (evaluate-expression left-expression environment))]
          (if left
            true
            (require-boolean
              (evaluate-expression right-expression environment))))

    (let [left (evaluate-expression left-expression environment)
          right (evaluate-expression right-expression environment)]
      (case operator
        :add (numeric-binary :add left right)
        :subtract (numeric-binary :subtract left right)
        :multiply (numeric-binary :multiply left right)
        :divide (numeric-binary :divide left right)
        :concat (if (and (vector? left) (vector? right))
                  (into left right)
                  (evaluation-failure! "type-error"
                                       {"operation" "list-concat"}))
        :update (if (and (instance? AttrsetValue left)
                         (instance? AttrsetValue right))
                  (->AttrsetValue (merge (:fields left) (:fields right)))
                  (evaluation-failure! "type-error"
                                       {"operation" "attrset-update"}))
        :equal (equal-values left right)
        :not-equal (not (equal-values left right))
        :less (ordered-compare :less left right)
        :less-equal (ordered-compare :less-equal left right)
        :greater (ordered-compare :greater left right)
        :greater-equal (ordered-compare :greater-equal left right)
        (evaluation-failure! "unknown-operator" {"operator" (name operator)})))))

(defn build-let-environment [bindings environment]
  (let [environment-reference (atom nil)
        cells (into {}
                    (map (fn [{:keys [name value lexical-inherit]}]
                           [name (cell value
                                       (if lexical-inherit
                                         (atom environment)
                                         environment-reference))]))
                    bindings)
        result (merge environment cells)]
    (reset! environment-reference result)
    result))

(defn build-attrset [fields recursive? environment]
  (let [resolved-fields
        (mapv #(assoc % :resolved-name
                      (evaluate-attribute-name % environment))
              fields)
        names (mapv :resolved-name resolved-fields)
        _ (when-not (= (count names) (count (set names)))
            (evaluation-failure! "duplicate-attrset-binding" {}))
        environment-reference (atom nil)
        cells (into {}
                    (map (fn [{:keys [resolved-name value lexical-inherit]}]
                           [resolved-name
                            (cell value
                                  (if lexical-inherit
                                    (atom environment)
                                    environment-reference))]))
                    resolved-fields)
        field-environment (if recursive?
                            (merge environment cells)
                            environment)]
    (reset! environment-reference field-environment)
    (->AttrsetValue cells)))

(defn cached-let-environment [expression environment]
  (session-cached
   expression
   environment
   #(build-let-environment (:bindings expression) environment)))

(defn cached-attrset [expression environment]
  (session-cached
   expression
   environment
   #(build-attrset (:fields expression)
                   (:recursive expression)
                   environment)))

(defn cached-list [expression environment]
  (session-cached
   expression
   environment
   #(mapv (fn [value]
            (cell value (atom environment)))
          (:values expression))))

(defn has-attribute-path [value path environment]
  (loop [current (force-cell value)
         names path]
    (if (or (empty? names)
            (not (instance? AttrsetValue current)))
      false
      (let [name (maybe-attribute-name (first names) environment)
            fields (:fields current)]
        (if (or (nil? name) (not (contains? fields name)))
          false
          (if (= 1 (count names))
            true
            (recur (force-cell (get fields name)) (rest names))))))))

(defn with-environment [scope-expression environment]
  (let [scope (evaluate-expression scope-expression environment)]
    (if (instance? AttrsetValue scope)
      (merge (:fields scope) environment)
      (evaluation-failure! "type-error" {"operation" "with"}))))

(defn evaluate-tail [expression environment]
  (loop [current expression
         current-environment environment]
    (case (:op current)
      :let (recur (:body current)
                  (cached-let-environment current current-environment))

      :if (recur (if (require-condition
                       (evaluate-expression (:condition current)
                                            current-environment))
                   (:then current)
                   (:else current))
                 current-environment)

      :assert (if (require-condition
                   (evaluate-expression (:condition current)
                                        current-environment))
                (recur (:body current) current-environment)
                (evaluation-failure! "assertion-failed" {}))

      :with (recur (:body current)
                   (with-environment (:scope current)
                                     current-environment))

      :call (let [function-value
                  (evaluate-expression (:function current)
                                       current-environment)]
              (cond
                (instance? ClosureValue function-value)
                (let [argument-cell
                      (call-argument-cell current current-environment)]
                  (recur (:body function-value)
                         (closure-call-environment function-value
                                                   argument-cell)))

                (instance? BuiltinValue function-value)
                (invoke-builtin-cell
                 function-value
                 (call-argument-cell current current-environment))

                :else
                (evaluation-failure! "not-callable" {})))

      :call-value (let [function-value (:function current)
                        argument-cell (:argument-cell current)]
                    (cond
                      (instance? ClosureValue function-value)
                      (recur (:body function-value)
                             (closure-call-environment function-value
                                                       argument-cell))

                      (instance? BuiltinValue function-value)
                      (invoke-builtin-cell function-value argument-cell)

                      :else
                      (evaluation-failure! "not-callable" {})))

      (evaluate-expression current current-environment))))

(defn evaluate-expression [expression environment]
  (case (:op expression)
    :integer (checked-integer (:value expression))
    :float (:value expression)
    :string (:value expression)
    :interpolated-string (evaluate-string-segments (:segments expression)
                                                   environment)
    :indented-string (evaluate-indented-string (:segments expression)
                                               environment)
    :boolean (:value expression)
    :null nil
    :variable (lookup environment (:name expression))
    :import (load-module (:path expression) environment)

    :lambda (->ClosureValue (:parameter expression)
                            (:body expression)
                            environment)

    :apply-values
    (reduce apply-value
            (:function expression)
            (:argument-cells expression))

    :call-value (let [function-value (:function expression)
                      argument-cell (:argument-cell expression)]
                  (cond
                    (instance? ClosureValue function-value)
                    (evaluate-tail
                     (:body function-value)
                     (closure-call-environment function-value
                                               argument-cell))

                    (instance? BuiltinValue function-value)
                    (invoke-builtin-cell function-value argument-cell)

                    :else
                    (evaluation-failure! "not-callable" {})))

    :call (let [function-value (evaluate-expression (:function expression)
                                                     environment)]
            (cond
              (instance? ClosureValue function-value)
              (let [argument-cell
                    (call-argument-cell expression environment)]
                (evaluate-tail
                 (:body function-value)
                 (closure-call-environment function-value argument-cell)))

              (instance? BuiltinValue function-value)
              (invoke-builtin-cell
               function-value
               (call-argument-cell expression environment))

              :else
              (evaluation-failure! "not-callable" {})))

    :let (evaluate-expression
           (:body expression)
           (cached-let-environment expression environment))

    :if (if (require-condition
              (evaluate-expression (:condition expression) environment))
          (evaluate-expression (:then expression) environment)
          (evaluate-expression (:else expression) environment))

    :assert (if (require-condition
                 (evaluate-expression (:condition expression) environment))
              (evaluate-expression (:body expression) environment)
              (evaluation-failure! "assertion-failed" {}))

    :with (evaluate-expression (:body expression)
                               (with-environment (:scope expression)
                                                 environment))

    :attrset (cached-attrset expression environment)

    :list (cached-list expression environment)

    :select (let [target (evaluate-expression (:target expression) environment)
                  name (evaluate-attribute-name expression environment)]
              (cond
                (and (instance? AttrsetValue target)
                     (contains? (:fields target) name))
                (force-cell (get (:fields target) name))

                (contains? expression :default)
                (evaluate-expression (:default expression) environment)

                (instance? AttrsetValue target)
                (evaluation-failure! "attribute-missing" {"name" name})

                :else
                (evaluation-failure! "type-error"
                                     {"operation" "attribute-selection"})))

    :has-attr (has-attribute-path
               (evaluate-expression (:target expression) environment)
               (:path expression)
               environment)

    :unary (case (:operator expression)
             :not (not (require-boolean
                         (evaluate-expression (:value expression) environment)))
             :negate (let [operand (:value expression)]
                       ;; The PNIX surface spells I64_MIN as unary minus applied
                       ;; to magnitude I64_MAX+1.  Fold that one literal before
                       ;; the ordinary literal range check; every other positive
                       ;; overflow remains invalid.
                       (if (and (= :integer (:op operand))
                                (= (js/BigInt "9223372036854775808")
                                   (:value operand)))
                         (checked-integer
                          (js/BigInt "-9223372036854775808"))
                         (let [value
                               (require-number
                                (evaluate-expression operand environment))]
                           (if (integer-value? value)
                             (checked-integer (integer-negate value))
                             (if (zero? value) 0.0 (- value))))))
             (evaluation-failure! "unknown-operator"
                                  {"operator" (name (:operator expression))}))

    :binary (evaluate-binary (:operator expression)
                             (:left expression)
                             (:right expression)
                             environment)

    (evaluation-failure! "unsupported-expression"
                         {"operation" (name (:op expression))})))

(defn materialize [value]
  (let [value (force-cell value)]
    (cond
      (instance? AttrsetValue value)
      (into {}
            (map (fn [[name field]]
                   [name (materialize field)]))
            (:fields value))

      (or (instance? ClosureValue value)
          (instance? BuiltinValue value))
      (throw (ex-info "open function value is not canonically observable"
                      {"pnix_error" true
                       "phase" "observation"
                       "class" "invalid-guest-value"
                       "evidence" {"kind" "function"}}))

      (instance? ByteStringValue value)
      (throw (ex-info "invalid UTF-8 byte string is not canonically observable"
                      {"pnix_error" true
                       "phase" "observation"
                       "class" "invalid-guest-value"
                       "evidence" {"kind" "string"
                                   "detail_class" "invalid-utf8"}}))

      (vector? value)
      (mapv materialize value)

      :else value)))

(defn evaluate
  ([expression]
   (evaluate-expression expression (builtin-environment)))
  ([expression module-context]
   (let [context (assoc module-context
                        :cache (or (:cache module-context) (atom {})))]
     (evaluate-expression expression
                          (assoc (builtin-environment)
                                 module-context-key
                                 context)))))
