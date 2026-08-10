(ns pnix-clr.json
  (:require [clojure.string :as str]
            [pnix-clr.outcome :as outcome]))

(declare write-json)

(defn- json-key
  [key]
  (cond
    (keyword? key) (name key)
    (string? key) key
    :else (str key)))

(defn- utf8-byte-compare
  [left right]
  (let [encoding (System.Text.UTF8Encoding. false true)
        left-bytes (.GetBytes encoding (str left))
        right-bytes (.GetBytes encoding (str right))
        shared (min (count left-bytes) (count right-bytes))]
    (loop [index 0]
      (if (= index shared)
        (compare (count left-bytes) (count right-bytes))
        (let [order (compare (int (nth left-bytes index))
                             (int (nth right-bytes index)))]
          (if (zero? order)
            (recur (inc index))
            order))))))

(defn- escape-string
  [value]
  (reduce (fn [output ch]
            (str output
                 (or (get {\" "\\\""
                           \\ "\\\\"
                           \newline "\\n"
                           \return "\\r"
                           \tab "\\t"
                           \backspace "\\b"
                           \formfeed "\\f"}
                          ch)
                     (when (< (int ch) 32)
                       (format "\\u%04x" (int ch)))
                     (str ch))))
          ""
          (str value)))

(defn- write-map
  [value]
  (let [entries (sort-by (comp json-key key) utf8-byte-compare value)]
    (str "{"
         (str/join ","
                   (map (fn [[key item]]
                          (str (write-json (json-key key)) ":" (write-json item)))
                        entries))
         "}")))

(defn write-json
  [value]
  (cond
    (nil? value) "null"
    (true? value) "true"
    (false? value) "false"
    (string? value) (str "\"" (escape-string value) "\"")
    ;; Prefer float serialization before integer? — CLR may box whole
    ;; doubles in a way that integer? is true.
    (and (number? value)
         (or (float? value)
             (instance? System.Double value)
             (instance? System.Single value)))
    (let [d (double value)]
      (if (or (Double/IsNaN d) (Double/IsInfinity d))
        (outcome/fail! :observation :invalid-guest-value
                       {:reason "json-noncanonical-number"})
        ;; Always include a decimal so 3.0 stays a JSON float (Nix parity).
        (let [s (.ToString d "R" System.Globalization.CultureInfo/InvariantCulture)]
          (if (re-find #"[.eE]" s) s (str s ".0")))))
    (integer? value) (str value)
    (number? value)
    (let [d (double value)]
      (if (or (Double/IsNaN d) (Double/IsInfinity d))
        (outcome/fail! :observation :invalid-guest-value
                       {:reason "json-noncanonical-number"})
        (.ToString d System.Globalization.CultureInfo/InvariantCulture)))
    (map? value) (write-map value)
    (sequential? value) (str "[" (str/join "," (map write-json value)) "]")
    :else
    (outcome/fail! :observation :invalid-guest-value
                   {:reason "json-unsupported-value"})))

(defn- json-fail!
  [reason evidence]
  (outcome/fail! :eval :type-error
                 (merge {:operation "fromJSON" :reason reason} evidence)))

(defn- skip-ws
  [s i]
  (let [n (count s)]
    (loop [j i]
      (if (and (< j n) (contains? #{\space \tab \newline \return} (nth s j)))
        (recur (inc j))
        j))))

(defn- parse-json-string
  [s i]
  (let [n (count s)]
    (when (or (>= i n) (not= \" (nth s i)))
      (json-fail! "expected-string" {:offset i}))
    (loop [j (inc i)
           out ""]
      (when (>= j n)
        (json-fail! "unterminated-string" {:offset i}))
      (let [ch (nth s j)]
        (cond
          (= ch \")
          [(inc j) out]

          (= ch \\)
          (let [k (inc j)]
            (when (>= k n)
              (json-fail! "unterminated-escape" {:offset j}))
            (let [e (nth s k)
                  piece (case e
                          \" "\""
                          \\ "\\"
                          \/ "/"
                          \b "\b"
                          \f "\f"
                          \n "\n"
                          \r "\r"
                          \t "\t"
                          \u (let [hex (subs s (inc k) (+ k 5))
                                   code (System.Int32/Parse
                                         hex
                                         System.Globalization.NumberStyles/HexNumber)]
                               (str (char code)))
                          (json-fail! "bad-escape" {:offset j :escape (str e)}))
                  advance (if (= e \u) 6 2)]
              (recur (+ j advance) (str out piece))))

          :else
          (recur (inc j) (str out ch)))))))

(declare parse-json-value)

(defn- parse-json-array
  [s i]
  (let [n (count s)
        i (inc i)]
    (loop [j (skip-ws s i)
           items []]
      (cond
        (>= j n)
        (json-fail! "unterminated-array" {:offset i})

        (= \] (nth s j))
        [(inc j) items]

        :else
        (let [[j' v] (parse-json-value s j)
              j2 (skip-ws s j')]
          (cond
            (>= j2 n)
            (json-fail! "unterminated-array" {:offset i})

            (= \, (nth s j2))
            (recur (skip-ws s (inc j2)) (conj items v))

            (= \] (nth s j2))
            [(inc j2) (conj items v)]

            :else
            (json-fail! "array-separator" {:offset j2})))))))

(defn- parse-json-object
  [s i]
  (let [n (count s)
        i (inc i)]
    (loop [j (skip-ws s i)
           obj {}]
      (cond
        (>= j n)
        (json-fail! "unterminated-object" {:offset i})

        (= \} (nth s j))
        [(inc j) obj]

        :else
        (let [[j1 key] (parse-json-string s j)
              j2 (skip-ws s j1)]
          (when (or (>= j2 n) (not= \: (nth s j2)))
            (json-fail! "expected-colon" {:offset j2}))
          (let [[j3 v] (parse-json-value s (skip-ws s (inc j2)))
                j4 (skip-ws s j3)
                obj' (assoc obj key v)]
            (cond
              (>= j4 n)
              (json-fail! "unterminated-object" {:offset i})

              (= \, (nth s j4))
              (recur (skip-ws s (inc j4)) obj')

              (= \} (nth s j4))
              [(inc j4) obj']

              :else
              (json-fail! "object-separator" {:offset j4}))))))))

(defn- parse-json-number
  [s i]
  (let [n (count s)
        start i
        i (if (and (< i n) (= \- (nth s i))) (inc i) i)
        i (if (and (< i n) (= \0 (nth s i)))
            (inc i)
            (loop [j i]
              (if (and (< j n) (contains? #{\0 \1 \2 \3 \4 \5 \6 \7 \8 \9}
                                          (nth s j)))
                (recur (inc j))
                j)))
        has-frac (and (< i n) (= \. (nth s i)))
        i (if has-frac
            (loop [j (inc i)]
              (if (and (< j n) (contains? #{\0 \1 \2 \3 \4 \5 \6 \7 \8 \9}
                                          (nth s j)))
                (recur (inc j))
                j))
            i)
        has-exp (and (< i n) (contains? #{\e \E} (nth s i)))
        i (if has-exp
            (let [j (inc i)
                  j (if (and (< j n) (contains? #{\+ \-} (nth s j))) (inc j) j)]
              (loop [k j]
                (if (and (< k n) (contains? #{\0 \1 \2 \3 \4 \5 \6 \7 \8 \9}
                                            (nth s k)))
                  (recur (inc k))
                  k)))
            i)
        text (subs s start i)]
    (if (or has-frac has-exp)
      [i (System.Double/Parse
          text System.Globalization.CultureInfo/InvariantCulture)]
      [i (System.Int64/Parse text)])))

(defn- parse-json-value
  [s i]
  (let [i (skip-ws s i)
        n (count s)]
    (when (>= i n)
      (json-fail! "unexpected-eof" {:offset i}))
    (let [ch (nth s i)]
      (cond
        (= ch \") (parse-json-string s i)
        (= ch \[) (parse-json-array s i)
        (= ch \{) (parse-json-object s i)
        (= ch \t)
        (if (= "true" (subs s i (min n (+ i 4))))
          [(+ i 4) true]
          (json-fail! "expected-true" {:offset i}))
        (= ch \f)
        (if (= "false" (subs s i (min n (+ i 5))))
          [(+ i 5) false]
          (json-fail! "expected-false" {:offset i}))
        (= ch \n)
        (if (= "null" (subs s i (min n (+ i 4))))
          [(+ i 4) nil]
          (json-fail! "expected-null" {:offset i}))
        (or (= ch \-) (contains? #{\0 \1 \2 \3 \4 \5 \6 \7 \8 \9} ch))
        (parse-json-number s i)
        :else
        (json-fail! "unexpected-token" {:offset i :token (str ch)})))))

(defn read-json
  "Parse a JSON string into a guest value (fromJSON). Integer-looking numbers
  prefer Int64; objects are string-keyed maps (last key wins)."
  [s]
  (let [s (str s)
        [i v] (parse-json-value s 0)
        j (skip-ws s i)]
    (when (< j (count s))
      (json-fail! "trailing-input" {:offset j}))
    v))
