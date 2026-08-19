(ns pnix-clr.evaluator
  "Small CLR seed evaluator. It has no host-language evaluator fallback: every
  accepted AST node is interpreted here and every other node is Failed.
  Builtins live here (not separate namespaces) to keep runtime-artifact.edn
  at exactly eight namespaces."
  (:require [clojure.string :as str]
            [pnix-clr.host :as host]
            [pnix-clr.json :as json]
            [pnix-clr.outcome :as outcome]
            [pnix-clr.parser :as parser]))

(declare eval-ast* eval-file* force-value realize-value
         apply-callable apply-callable2 exec-builtin root-environment)

;; ---------------------------------------------------------------------------
;; Values
;; ---------------------------------------------------------------------------

(defn- thunk
  ([compute]
   (thunk compute :eval :cycle-detected))
  ([compute cycle-phase cycle-class]
   {:pnix/type :thunk
    :state (atom :pending)
    :value (atom nil)
    :compute compute
    :cycle-phase cycle-phase
    :cycle-class cycle-class}))

(defn- thunk?
  [value]
  (and (map? value) (= :thunk (:pnix/type value))))

(defn force-value
  [value]
  (cond
    (thunk? value)
    (case @(:state value)
      :done @(:value value)
      :running (outcome/fail! (:cycle-phase value) (:cycle-class value) {})
      :failed (throw @(:value value))
      :pending
      (do
        (reset! (:state value) :running)
        (try
          (let [result ((:compute value))]
            (reset! (:value value) result)
            (reset! (:state value) :done)
            result)
          (catch System.Exception error
            (reset! (:value value) error)
            (reset! (:state value) :failed)
            (throw error)))))

    :else value))

(defn- lookup-value
  [environment name]
  (loop [frames environment]
    (if-let [frame (first frames)]
      (let [entries @frame]
        (if (contains? entries name)
          (force-value (get entries name))
          (recur (next frames))))
      (outcome/fail! :eval :unknown-variable {:name name}))))

(defn- boolean-value
  [value operator]
  (let [value (force-value value)]
    (if (boolean? value)
      value
      (outcome/fail! :eval :type-error {:operator operator :expected "bool"}))))

(defn- integer-value
  [value operator]
  (let [value (force-value value)]
    (if (integer? value)
      value
      (outcome/fail! :eval :type-error
                     {:operator operator :expected "int"}))))

(defn- string-value
  [value operation]
  (let [value (force-value value)]
    (cond
      (string? value) value
      ;; Raw-byte intermediates are not general strings (see :raw-bytes).
      (and (map? value) (= :raw-bytes (:pnix/type value)))
      (outcome/fail! :eval :type-error
                     {:operation operation :expected "string"
                      :reason "raw-bytes"})
      :else
      (outcome/fail! :eval :type-error
                     {:operation operation :expected "string"}))))

(defn- list-value
  [value operation]
  (let [value (force-value value)]
    (if (vector? value)
      value
      (outcome/fail! :eval :type-error
                     {:operation operation :expected "list"}))))

(defn- checked-add
  [left right]
  (when (or (and (pos? right)
                 (> left (- System.Int64/MaxValue right)))
            (and (neg? right)
                 (< left (- System.Int64/MinValue right))))
    (outcome/fail! :eval :integer-overflow {:operator "+"}))
  (+ left right))

(defn- checked-negate
  [value]
  (when (= value System.Int64/MinValue)
    (outcome/fail! :eval :integer-overflow {:operator "-" :arity 1}))
  (- value))

(defn- checked-subtract
  [left right]
  (when (or (and (pos? right)
                 (< left (+ System.Int64/MinValue right)))
            (and (neg? right)
                 (> left (+ System.Int64/MaxValue right))))
    (outcome/fail! :eval :integer-overflow {:operator "-"}))
  (- left right))

(defn- checked-multiply
  [left right]
  (when
      (cond
        (zero? left) false
        (pos? left) (if (pos? right)
                      (> left (quot System.Int64/MaxValue right))
                      (and (neg? right)
                           (< right (quot System.Int64/MinValue left))))
        (pos? right) (< left (quot System.Int64/MinValue right))
        (neg? right) (< left (quot System.Int64/MaxValue right))
        :else false)
    (outcome/fail! :eval :integer-overflow {:operator "*"}))
  (* left right))

(defn- checked-divide
  [left right]
  (when (zero? right)
    (outcome/fail! :eval :division-by-zero {:operator "/"}))
  (when (and (= left System.Int64/MinValue) (= right -1))
    (outcome/fail! :eval :integer-overflow {:operator "/"}))
  (quot left right))

(defn- recursive-frame
  "Build a recursive binding frame. Plain inherit desugars to
  `:enclosing-var`, which must resolve in `outer-environment` (Nix:
  `rec { inherit x; }` / `let inherit x;` bind from outside the form)."
  [bindings outer-environment context]
  (let [frame (atom {})
        environment (cons frame outer-environment)
        entries (into {}
                      (map (fn [[name ast]]
                             [name
                              (if (= :enclosing-var (:op ast))
                                (thunk #(lookup-value outer-environment
                                                      (:name ast)))
                                (thunk #(eval-ast* ast environment context)))]))
                      bindings)]
    (reset! frame entries)
    [frame environment]))

(defn- attrset-value
  [entries]
  {:pnix/type :attrset :entries entries})

(defn- attrset?
  [value]
  (and (map? value) (= :attrset (:pnix/type value))))

(defn- require-attrset
  [value operation]
  (let [value (force-value value)]
    (if (attrset? value)
      value
      (outcome/fail! :eval :type-error
                     {:operation operation :expected "attrset"}))))

(defn- closure-value
  ([parameter body environment context]
   {:pnix/type :closure
    :parameter parameter
    :param-pattern nil
    :body body
    :environment environment
    :context context})
  ([parameter param-pattern body environment context]
   {:pnix/type :closure
    :parameter parameter
    :param-pattern param-pattern
    :body body
    :environment environment
    :context context}))

(defn- closure?
  [value]
  (and (map? value) (= :closure (:pnix/type value))))

(defn- path-value
  [path]
  {:pnix/type :path :value path})

(defn- path-value?
  [value]
  (and (map? value) (= :path (:pnix/type value))))

(defn- raw-bytes-value
  "Nix-permitted invalid UTF-8 intermediate (substring off a char boundary)."
  [bs]
  {:pnix/type :raw-bytes
   :bytes (if (instance? System.Array bs)
            bs
            (byte-array (map unchecked-byte bs)))})

(defn- raw-bytes?
  [value]
  (and (map? value) (= :raw-bytes (:pnix/type value))))

(def ^:private utf8-strict
  (System.Text.UTF8Encoding. false true))

(def ^:private utf8-lenient
  (System.Text.Encoding/UTF8))

(defn- as-utf8-bytes
  "UTF-8 byte view of a string or raw-bytes value (Nix byte string model)."
  [value operation]
  (let [value (force-value value)]
    (cond
      (string? value)
      (.GetBytes utf8-lenient ^String value)

      (raw-bytes? value)
      (:bytes value)

      :else
      (outcome/fail! :eval :type-error
                     {:operation operation :expected "string"}))))

(defn- decode-utf8-or-raw
  [bs]
  (try
    (.GetString utf8-strict bs)
    (catch System.Exception _
      (raw-bytes-value bs))))

(defn- builtin-value
  ([name arity]
   (builtin-value name arity []))
  ([name arity args]
   {:pnix/type :builtin
    :name name
    :arity arity
    :args (vec args)}))

(defn- builtin?
  [value]
  (and (map? value) (= :builtin (:pnix/type value))))

(defn- callable?
  [value]
  (or (closure? value) (builtin? value)))

(defn- float-value?
  "Finite/Inf doubles, or a NaN cell (unique object so shared-value identity
  works on CLR where raw Double.NaN boxes may collapse)."
  [value]
  (or (and (number? value) (not (integer? value)))
      (and (map? value) (= :float-nan (:pnix/type value)))))

(defn- float-double
  "Unwrap a guest float (including NaN cells) to System.Double."
  [value]
  (if (and (map? value) (= :float-nan (:pnix/type value)))
    (double (:value value))
    (double value)))

(defn- nix-double
  "Box a host double as a guest float. Each NaN production gets a fresh cell
  so two independent `inf-inf` bindings stay unequal under nested identity."
  [d]
  (let [d (double d)]
    (if (Double/IsNaN d)
      {:pnix/type :float-nan :value d :cell (Object.)}
      d)))

(defn- ieee-div
  "Float division with IEEE Inf/NaN for zero divisors. CLR may throw on
  bare `/` when the denominator is 0.0; construct the result explicitly."
  [a b]
  (let [a (double a) b (double b)]
    (if (zero? b)
      (cond
        (Double/IsNaN a) Double/NaN
        (zero? a) Double/NaN
        (pos? a) (if (neg? (Math/CopySign 1.0 b))
                   Double/NegativeInfinity
                   Double/PositiveInfinity)
        :else (if (neg? (Math/CopySign 1.0 b))
                Double/PositiveInfinity
                Double/NegativeInfinity))
      (/ a b))))

(def ^:private nix-int-upper-exclusive-double
  ;; 2^63 is exactly representable as f64; positive exclusive bound for i64.
  9223372036854775808.0)

(def ^:private nix-int-lower-inclusive-double
  (- nix-int-upper-exclusive-double))

(defn- integer-exactly-representable-as-double?
  "Nix 2.34.7: int→float for ceil/floor must round-trip exactly."
  [value]
  (let [n (long value)
        d (double n)]
    (and (Double/IsFinite d)
         (< d nix-int-upper-exclusive-double)
         (>= d nix-int-lower-inclusive-double)
         ;; Lossy mantissa (e.g. 2^53+1 → 2^53) fails the int cast-back.
         (= n (long d)))))

(defn- round-to-int
  "Nix ceil/floor: ints must survive the int→float seam; float results must
  land in signed i64 before conversion."
  [op-name value upward?]
  (cond
    (integer? value)
    (if (integer-exactly-representable-as-double? value)
      (long value)
      (outcome/fail! :eval :type-error
                     {:operation op-name
                      :reason "integer-float-precision-loss"
                      :value value}))

    (float-value? value)
    (let [d (float-double value)]
      (if (or (Double/IsNaN d) (Double/IsInfinity d)
              (>= d nix-int-upper-exclusive-double)
              (< d nix-int-lower-inclusive-double))
        (outcome/fail! :eval :type-error
                       {:operation op-name
                        :reason "float-to-int-out-of-range"
                        :value value})
        (let [truncated (long d)
              exact? (= (double truncated) d)
              n (cond
                  (and upward? (pos? d) (not exact?)) (inc truncated)
                  (and (not upward?) (neg? d) (not exact?)) (dec truncated)
                  :else truncated)]
          (long n))))

    :else
    (outcome/fail! :eval :type-error
                   {:operation op-name :expected "number"})))

(defn- nix-float-str
  "Nix float toString (oracle D4): banker's round to 6 places; exact +0 is
  \"0.000000\"; exact -0 and nonzero that round to zero keep a leading '-'."
  [v]
  (let [d (float-double v)]
    (cond
      (Double/IsNaN d) "nan"
      (= d Double/PositiveInfinity) "inf"
      (= d Double/NegativeInfinity) "-inf"
      (and (zero? d) (not (neg? (Math/CopySign 1.0 d))))
      "0.000000"
      :else
      (let [rounded-num (Math/Round d 6 System.MidpointRounding/ToEven)
            body (System.String/Format
                  System.Globalization.CultureInfo/InvariantCulture
                  "{0:F6}"
                  (Math/Abs rounded-num))
            negative? (or (neg? d)
                          (and (zero? d)
                               (neg? (Math/CopySign 1.0 d))))]
        (if negative?
          (str "-" body)
          body)))))

(defn- deep-equal?
  "Nix equality. Top-level functions and NaNs are never equal; inside a
  list/attrset slot, the same object (pointer) is equal before scalar rules
  (shared-value shortcut)."
  ([left right]
   (deep-equal? left right false))
  ([left right recursive-slot?]
   (let [left (force-value left)
         right (force-value right)]
     (cond
       (and recursive-slot? (identical? left right)) true

       (and (nil? left) (nil? right)) true
       (and (boolean? left) (boolean? right)) (= left right)
       (and (string? left) (string? right)) (= left right)
       (and (integer? left) (integer? right)) (= left right)
       (and (or (integer? left) (float-value? left))
            (or (integer? right) (float-value? right)))
       ;; IEEE: NaN never equals (shared identity handled above).
       (let [dl (float-double left)
             dr (float-double right)]
         (if (or (Double/IsNaN dl) (Double/IsNaN dr))
           false
           (== dl dr)))

       (and (path-value? left) (path-value? right))
       (= (:value left) (:value right))

       (and (vector? left) (vector? right))
       (and (= (count left) (count right))
            (every? true? (map #(deep-equal? %1 %2 true) left right)))

       (and (attrset? left) (attrset? right))
       (let [le (:entries left)
             re (:entries right)
             lk (set (keys le))
             rk (set (keys re))]
         (and (= lk rk)
              (every? (fn [k]
                        (deep-equal? (get le k) (get re k) true))
                      lk)))

       ;; Closures/builtins: only shared-slot identity (handled above).
       (or (closure? left) (closure? right)
           (builtin? left) (builtin? right))
       false

       :else false))))

(defn- i64
  [n]
  (long n))

;; ---------------------------------------------------------------------------
;; Coercion / equality helpers
;; ---------------------------------------------------------------------------

(defn- path-or-string
  [value operation]
  (let [value (force-value value)]
    (cond
      (string? value) value
      (path-value? value) (:value value)
      :else
      (outcome/fail! :eval :type-error
                     {:operation operation :expected "path-or-string"}))))

(defn- nix-to-string
  [value]
  (let [value (force-value value)]
    (cond
      (string? value) value
      (raw-bytes? value)
      ;; Best-effort: ISO-8859-1 round-trip keeps single-byte slices as 1:1
      ;; chars for display; hashString uses the raw bytes, not this path.
      (.GetString (System.Text.Encoding/GetEncoding "ISO-8859-1")
                  (:bytes value))
      (nil? value) ""
      (true? value) "1"
      (false? value) ""
      (integer? value) (str value)
      (float-value? value)
      (nix-float-str value)
      (path-value? value) (:value value)
      (vector? value)
      (str/join " " (map nix-to-string value))
      (attrset? value)
      (cond
        (contains? (:entries value) "__toString")
        (nix-to-string
         (apply-callable (force-value (get (:entries value) "__toString"))
                         value))
        (contains? (:entries value) "outPath")
        (nix-to-string (force-value (get (:entries value) "outPath")))
        :else
        (outcome/fail! :eval :type-error
                       {:operation "toString" :expected "coercible"}))
      (or (closure? value) (builtin? value))
      (outcome/fail! :eval :type-error
                     {:operation "toString" :expected "coercible"})
      :else
      (outcome/fail! :eval :type-error
                     {:operation "toString" :expected "coercible"}))))

(defn- values-equal?
  "Same algebra as deep-equal? (including shared-slot identity)."
  [left right]
  (deep-equal? left right))

(defn- string-ordinal-compare
  [a b]
  (System.String/Compare (str a) (str b) System.StringComparison/Ordinal))

(defn- sorted-attr-keys
  [attrs]
  (vec (sort string-ordinal-compare (keys (:entries attrs)))))

(defn- force-list-items
  [xs]
  (mapv force-value xs))

;; ---------------------------------------------------------------------------
;; Apply callables
;; ---------------------------------------------------------------------------

(def ^:private lazy-arg-builtins
  "Builtins whose next argument is kept as a thunk (Nix lazy positions)."
  #{:tryEval :map :mapAttrs :filter :filterAttrs :concatMap :genList
    :foldl :foldl' :foldr :partition :sort :any :all :zipListsWith
    :concatMapStringsSep :pipe :fix :flip :const :id :optional :optionals
    :optionalAttrs :optionalString :when :implies :trace :warn :hashString})

(defn- apply-pattern-closure
  "Apply a pattern-lambda closure. Nix/D19 semantics (oracle-aligned):
  argument must be an attrset; required formals checked in pattern order
  before extra-key check; without `...` extras fail; defaults are lazy
  thunks in a knot-tied recursive frame; `@as` binds the actual argument
  only (defaults are not injected into it)."
  [f arg]
  (let [pattern (:param-pattern f)
        arg-val (force-value arg)
        params (:params pattern)
        as-name (:as pattern)]
    (when-not (attrset? arg-val)
      (outcome/fail! :eval :lambda-pattern-arg-not-attrset
                     {:value-type (cond
                                    (nil? arg-val) "null"
                                    (boolean? arg-val) "bool"
                                    (integer? arg-val) "int"
                                    (string? arg-val) "string"
                                    (vector? arg-val) "list"
                                    :else "other")}))
    (when-let [missing (some (fn [p]
                               (when (and (not (contains? p :default))
                                          (not (contains? (:entries arg-val)
                                                          (:name p))))
                                 (:name p)))
                             params)]
      (outcome/fail! :eval :missing-lambda-pattern-arg {:param missing}))
    (let [formal-names (into #{} (map :name) params)
          extra (when-not (:ellipsis? pattern)
                  (first (sort (remove formal-names (keys (:entries arg-val))))))]
      (when extra
        (outcome/fail! :eval :unexpected-lambda-pattern-arg {:arg extra}))
      (let [frame (atom {})
            environment (cons frame (:environment f))
            bindings
            (reduce
             (fn [acc {:keys [name default]}]
               (assoc acc name
                      (if (contains? (:entries arg-val) name)
                        (get (:entries arg-val) name)
                        (thunk #(eval-ast* default environment
                                           (:context f))))))
             (cond-> {}
               as-name (assoc as-name arg-val))
             params)]
        (reset! frame bindings)
        (eval-ast* (:body f) environment (:context f))))))

(defn- apply-callable
  [f arg]
  (let [f (force-value f)]
    (cond
      (closure? f)
      (if (:param-pattern f)
        (apply-pattern-closure f arg)
        (let [frame (atom {(:parameter f) arg})]
          (eval-ast* (:body f)
                     (cons frame (:environment f))
                     (:context f))))

      (builtin? f)
      (let [arg-value (if (contains? lazy-arg-builtins (:name f))
                        arg
                        (force-value arg))
            next (update f :args conj arg-value)]
        (if (= (count (:args next)) (:arity next))
          (exec-builtin next)
          next))

      :else
      (outcome/fail! :eval :not-callable {}))))

(defn- apply-callable2
  [f a b]
  (apply-callable (apply-callable f a) b))

;; ---------------------------------------------------------------------------
;; Attr path walks
;; ---------------------------------------------------------------------------

(defn- walk-attr-path
  [path attrs]
  (loop [remaining path
         current attrs]
    (if (empty? remaining)
      current
      (let [current (force-value current)
            key (force-value (first remaining))]
        (when-not (string? key)
          (outcome/fail! :eval :type-error
                         {:operation "attr-path" :expected "string-key"}))
        (when-not (attrset? current)
          (outcome/fail! :eval :attribute-missing
                         {:attribute key :path path}))
        (when-not (contains? (:entries current) key)
          (outcome/fail! :eval :attribute-missing
                         {:attribute key :path path}))
        (recur (rest remaining) (get (:entries current) key))))))

(defn- has-attr-by-path?
  [path attrs]
  (try
    (walk-attr-path path attrs)
    true
    (catch System.Exception e
      (if (::outcome/failure (ex-data e))
        false
        (throw e)))))

(defn- attr-by-path
  [path default attrs]
  (try
    (force-value (walk-attr-path path attrs))
    (catch System.Exception e
      (if (::outcome/failure (ex-data e))
        default
        (throw e)))))

(defn- recursive-update
  [left right]
  (let [left (require-attrset left "recursiveUpdate")
        right (require-attrset right "recursiveUpdate")]
    (attrset-value
     (reduce (fn [acc [k v]]
               (if (and (contains? acc k)
                        (attrset? (force-value (get acc k)))
                        (attrset? (force-value v)))
                 (assoc acc k (recursive-update (get acc k) v))
                 (assoc acc k v)))
             (:entries left)
             (:entries right)))))

;; ---------------------------------------------------------------------------
;; toXML / JSON / regex / fetch
;; ---------------------------------------------------------------------------

(defn- xml-escape
  [s]
  (-> (str s)
      (str/replace "&" "&amp;")
      (str/replace "<" "&lt;")
      (str/replace ">" "&gt;")
      (str/replace "\"" "&quot;")))

(defn- to-xml-value
  [value indent]
  (let [pad (apply str (repeat indent "  "))
        value (force-value value)]
    (cond
      (nil? value) (str pad "<null />\n")
      (true? value) (str pad "<bool>true</bool>\n")
      (false? value) (str pad "<bool>false</bool>\n")
      (integer? value) (str pad "<int>" value "</int>\n")
      (string? value) (str pad "<string>" (xml-escape value) "</string>\n")
      (path-value? value)
      (str pad "<path>" (xml-escape (:value value)) "</path>\n")
      (vector? value)
      (str pad "<list>\n"
           (apply str (map #(to-xml-value % (inc indent)) value))
           pad "</list>\n")
      (attrset? value)
      (str pad "<attrs>\n"
           (apply str
                  (map (fn [k]
                         (str pad "  <attr name=\"" (xml-escape k) "\">\n"
                              (to-xml-value (get (:entries value) k)
                                            (+ indent 2))
                              pad "  </attr>\n"))
                       (sorted-attr-keys value)))
           pad "</attrs>\n")
      (or (closure? value) (builtin? value))
      (str pad "<function />\n")
      :else (str pad "<unknown />\n"))))

(def ^:private posix-bracket-class
  "Contents to splice inside an outer `[...]` for `[:name:]` tokens.
  `[[:alnum:]]` becomes `[A-Za-z0-9]`, not `[[A-Za-z0-9]]`."
  {"alnum" "A-Za-z0-9"
   "alpha" "A-Za-z"
   "blank" " \\t"
   "cntrl" "\\x00-\\x1F\\x7F"
   "digit" "0-9"
   "graph" "\\x21-\\x7E"
   "lower" "a-z"
   "print" "\\x20-\\x7E"
   "punct" "\\x21-\\x2F\\x3A-\\x40\\x5B-\\x60\\x7B-\\x7E"
   "space" " \\t\\n\\r\\f\\v"
   "upper" "A-Z"
   "xdigit" "0-9A-Fa-f"})

(defn- translate-nix-regex-pattern
  "Rewrite `[[:class:]]` inside a bracket expression to ASCII ranges.
  Bare `[:space:]` (no outer brackets) stays literal. Unknown class → fail."
  [pattern]
  (let [source (str pattern)
        n (count source)]
    (loop [i 0
           in-class? false
           class-first? false
           out ""]
      (if (>= i n)
        out
        (let [ch (nth source i)]
          (cond
            (and (= ch \\) (< (inc i) n))
            (recur (+ i 2) in-class? false
                   (str out (subs source i (+ i 2))))

            (not in-class?)
            (recur (inc i) (= ch \[) (= ch \[) (str out ch))

            (and class-first? (= ch \^))
            (recur (inc i) true true (str out "^"))

            (and class-first? (= ch \]))
            (recur (inc i) true false (str out "]"))

            (= ch \[)
            (let [rest (subs source i)
                  m (re-find #"^\[:([a-z]+):\]" rest)]
              (if m
                (let [token (first m)
                      class-name (second m)
                      replacement (get posix-bracket-class class-name)]
                  (if replacement
                    (recur (+ i (count token)) true false
                           (str out replacement))
                    (outcome/fail! :eval :type-error
                                   {:operation "match"
                                    :reason "unknown-posix-class"
                                    :class class-name})))
                (recur (inc i) true false (str out "["))))

            (= ch \])
            (recur (inc i) false false (str out "]"))

            :else
            (recur (inc i) true false (str out ch))))))))

(defn- re-group-at
  "CLR GroupCollection indexer (get_Item), not JVM-style .Item."
  [groups i]
  (.get_Item
   ^System.Text.RegularExpressions.GroupCollection groups
   (int i)))

(defn- re-match-at
  [matches i]
  (.get_Item
   ^System.Text.RegularExpressions.MatchCollection matches
   (int i)))

(defn- compile-nix-regex
  [pattern]
  (System.Text.RegularExpressions.Regex.
   (translate-nix-regex-pattern pattern)))

(defn- regex-match
  [pattern s]
  (try
    (let [re (compile-nix-regex pattern)
          m (.Match re (str s))]
      (if (.Success m)
        (let [groups (.Groups m)
              n (.Count groups)]
          ;; group 0 is full match; Nix match returns capture groups 1..
          (if (<= n 1)
            []
            (mapv (fn [i]
                    (let [g (re-group-at groups i)]
                      (when (.Success g) (.Value g))))
                  (range 1 n))))
        nil))
    (catch System.ArgumentException e
      (outcome/fail! :eval :type-error
                     {:operation "match"
                      :reason "invalid-regex"
                      :message (.Message e)}))))

(defn- regex-split
  [pattern s]
  (try
    (let [re (compile-nix-regex pattern)
          s (str s)
          matches (.Matches re s)]
      (if (zero? (.Count matches))
        [s]
        (loop [i 0
               idx 0
               out []]
          (if (>= i (.Count matches))
            (conj out (subs s idx))
            (let [m (re-match-at matches i)
                  start (.Index m)
                  len (.Length m)
                  before (subs s idx start)
                  groups (.Groups m)
                  caps (if (<= (.Count groups) 1)
                         []
                         (mapv (fn [gi]
                                 (let [g (re-group-at groups gi)]
                                   (when (.Success g) (.Value g))))
                               (range 1 (.Count groups))))]
              (recur (inc i)
                     (+ start len)
                     (conj (conj out before) caps)))))))
    (catch System.ArgumentException e
      (outcome/fail! :eval :type-error
                     {:operation "split"
                      :reason "invalid-regex"
                      :message (.Message e)}))))

(defn- fetch-url-arg
  [arg]
  (let [arg (force-value arg)]
    (cond
      (string? arg) {:url arg :name "source"}
      (attrset? arg)
      (let [url (force-value (get (:entries arg) "url"))
            name (or (some-> (get (:entries arg) "name") force-value)
                     "source")]
        {:url (str url) :name (str name)})
      :else nil)))

(defn- download-url!
  [url name]
  (try
    (let [client (System.Net.Http.HttpClient.)
          bytes (.Result (.GetByteArrayAsync client (str url)))
          encoding (System.Text.UTF8Encoding. false)
          text (try (.GetString encoding bytes)
                    (catch System.Exception _
                      (System.Convert/ToBase64String bytes)))
          path (host/write-store-file name text)]
      (.Dispose client)
      (path-value path))
    (catch System.Exception e
      (try
        (let [tmp (host/combine (System.IO.Path/GetTempPath)
                                (str "pnix-fetch-" (System.Guid/NewGuid)))
              psi (System.Diagnostics.ProcessStartInfo.)]
          (set! (.FileName psi) "curl")
          (set! (.Arguments psi) (str "-fsSL -o " tmp " " url))
          (set! (.UseShellExecute psi) false)
          (set! (.RedirectStandardError psi) true)
          (let [proc (System.Diagnostics.Process/Start psi)]
            (.WaitForExit proc)
            (if (and (zero? (.ExitCode proc))
                     (System.IO.File/Exists tmp))
              (let [text (System.IO.File/ReadAllText tmp)
                    path (host/write-store-file name text)]
                (System.IO.File/Delete tmp)
                (path-value path))
              (outcome/fail! :eval :type-error
                             {:operation "fetchurl"
                              :reason "download-failed"
                              :url (str url)
                              :message (.Message e)}))))
        (catch System.Exception e2
          (outcome/fail! :eval :type-error
                         {:operation "fetchurl"
                          :reason "download-failed"
                          :url (str url)
                          :message (.Message e2)}))))))

(defn- fetch-git!
  [arg]
  (let [arg (force-value arg)
        attrs (cond
                (string? arg) {"url" arg}
                (attrset? arg)
                (into {}
                      (map (fn [[k v]] [k (force-value v)]))
                      (:entries arg))
                :else nil)]
    (when (nil? attrs)
      (outcome/fail! :eval :type-error
                     {:operation "fetchGit" :expected "url-or-attrs"}))
    (let [url (str (get attrs "url"))
          rev (str (or (get attrs "rev") (get attrs "ref") "HEAD"))
          key (str url "\0" rev)
          store-path (host/write-store-file "git-src" key)
          dir (str store-path ".clone")]
      (if (System.IO.Directory/Exists dir)
        (attrset-value
         {"outPath" dir
          "rev" rev
          "url" url
          "shortRev" (subs rev 0 (min 7 (count rev)))})
        (try
          (let [psi (System.Diagnostics.ProcessStartInfo.)]
            (set! (.FileName psi) "git")
            (set! (.Arguments psi)
                  (if (and rev (not= rev "HEAD") (not (str/blank? rev)))
                    (str "clone --depth 1 --branch " rev " " url " " dir)
                    (str "clone --depth 1 " url " " dir)))
            (set! (.UseShellExecute psi) false)
            (set! (.RedirectStandardError psi) true)
            (let [proc (System.Diagnostics.Process/Start psi)]
              (.WaitForExit proc)
              (if (and (zero? (.ExitCode proc))
                       (System.IO.Directory/Exists dir))
                (attrset-value
                 {"outPath" dir
                  "rev" rev
                  "url" url
                  "shortRev" (subs rev 0 (min 7 (count rev)))})
                (attrset-value
                 {"outPath" store-path
                  "rev" rev
                  "url" url
                  "shortRev" (subs rev 0 (min 7 (count rev)))}))))
          (catch System.Exception _
            (attrset-value
             {"outPath" store-path
              "rev" rev
              "url" url
              "shortRev" (subs rev 0 (min 7 (count rev)))})))))))

(defn- flatten-lists
  [value]
  (let [value (force-value value)]
    (if (vector? value)
      (vec (mapcat (fn [item]
                     (let [item (force-value item)]
                       (if (vector? item)
                         (flatten-lists item)
                         [item])))
                   value))
      (outcome/fail! :eval :type-error
                     {:operation "flatten" :expected "list"}))))

(defn- partition-list
  [pred xs]
  (loop [remaining xs
         right []
         wrong []]
    (if (empty? remaining)
      (attrset-value {"right" right "wrong" wrong})
      (let [item (first remaining)
            keep? (boolean-value (apply-callable pred item) "partition")]
        (if keep?
          (recur (rest remaining) (conj right item) wrong)
          (recur (rest remaining) right (conj wrong item)))))))

;; ---------------------------------------------------------------------------
;; Builtin execution
;; ---------------------------------------------------------------------------

(defn- exec-builtin
  [{:keys [name args]}]
  (case name
    ;; ---- Core ----
    :typeOf
    (let [v (force-value (first args))]
      (cond
        (nil? v) "null"
        (path-value? v) "path"
        (string? v) "string"
        (boolean? v) "bool"
        (integer? v) "int"
        (float-value? v) "float"
        (vector? v) "list"
        (attrset? v) "set"
        (or (closure? v) (builtin? v)) "lambda"
        :else "unknown"))

    :tryEval
    ;; Nix: only `throw` and `assert` failures are catchable. Division by
    ;; zero, overflow, type errors, abort, etc. propagate (uncatchable).
    (try
      (let [v (force-value (first args))]
        (attrset-value {"success" true "value" v}))
      (catch System.Exception e
        (let [data (ex-data e)]
          (if (and (::outcome/failure data)
                   (contains? #{:throw :assertion-failed}
                              (::outcome/class data)))
            (attrset-value {"success" false "value" false})
            (throw e)))))

    :throw
    ;; Message must be string-coercible only via toString rules; non-strings
    ;; are a type error before a catchable throw exists (Nix/corpus).
    (let [msg (force-value (first args))]
      (when-not (string? msg)
        (outcome/fail! :eval :type-error
                       {:operation "throw" :expected "string"}))
      (outcome/fail! :eval :throw {:message msg}))

    :abort
    (outcome/fail! :eval :abort
                   {:message (nix-to-string (first args))})

    :trace
    (do
      (binding [*out* *err*]
        (println (str "trace: " (nix-to-string (first args)))))
      (force-value (second args)))

    :warn
    (do
      (binding [*out* *err*]
        (println (str "evaluation warning: " (nix-to-string (first args)))))
      (force-value (second args)))

    :toString
    (nix-to-string (first args))

    :toJSON
    (json/write-json (realize-value (first args)))

    :toXML
    (str "<?xml version='1.0' encoding='utf-8'?>\n"
         "<expr>\n"
         (to-xml-value (first args) 1)
         "</expr>\n")

    :toFile
    (path-value
     (host/write-store-file (string-value (first args) "toFile")
                            (nix-to-string (second args))))

    ;; ---- Attrs ----
    :attrNames
    (sorted-attr-keys (require-attrset (first args) "attrNames"))

    :attrValues
    (let [attrs (require-attrset (first args) "attrValues")]
      (mapv #(get (:entries attrs) %) (sorted-attr-keys attrs)))

    :hasAttr
    (let [attr (string-value (first args) "hasAttr")
          attrs (require-attrset (second args) "hasAttr")]
      (contains? (:entries attrs) attr))

    :getAttr
    (let [attr (string-value (first args) "getAttr")
          attrs (require-attrset (second args) "getAttr")]
      (when-not (contains? (:entries attrs) attr)
        (outcome/fail! :eval :attribute-missing {:attribute attr}))
      (force-value (get (:entries attrs) attr)))

    :getAttrFromPath
    (force-value
     (walk-attr-path (list-value (first args) "getAttrFromPath")
                     (require-attrset (second args) "getAttrFromPath")))

    :hasAttrByPath
    (has-attr-by-path?
     (list-value (first args) "hasAttrByPath")
     (require-attrset (second args) "hasAttrByPath"))

    :attrByPath
    (attr-by-path
     (list-value (first args) "attrByPath")
     (second args)
     (require-attrset (nth args 2) "attrByPath"))

    :mapAttrs
    (let [f (force-value (first args))
          attrs (require-attrset (second args) "mapAttrs")]
      (attrset-value
       (into {}
             (map (fn [[k v]]
                    [k (thunk #(apply-callable2 f k v))]))
             (:entries attrs))))

    :filterAttrs
    (let [pred (force-value (first args))
          attrs (require-attrset (second args) "filterAttrs")]
      (attrset-value
       (into {}
             (keep (fn [[k v]]
                     (when (boolean-value (apply-callable2 pred k v)
                                          "filterAttrs")
                       [k v])))
             (:entries attrs))))

    :listToAttrs
    (let [xs (list-value (first args) "listToAttrs")]
      (attrset-value
       (reduce (fn [acc row]
                 (let [row (require-attrset row "listToAttrs")
                       n (force-value (get (:entries row) "name"))
                       v (get (:entries row) "value")]
                   (when-not (string? n)
                     (outcome/fail! :eval :type-error
                                    {:operation "listToAttrs"
                                     :expected "name-string"}))
                   (if (contains? acc n)
                     acc
                     (assoc acc n v))))
               {}
               xs)))

    :removeAttrs
    (let [attrs (require-attrset (first args) "removeAttrs")
          names (force-list-items (list-value (second args) "removeAttrs"))]
      (when-not (every? string? names)
        (outcome/fail! :eval :type-error
                       {:operation "removeAttrs" :expected "string-list"}))
      (attrset-value (apply dissoc (:entries attrs) names)))

    :recursiveUpdate
    (recursive-update (first args) (second args))

    :zipAttrsWith
    (let [f (force-value (first args))
          rows (mapv #(require-attrset % "zipAttrsWith")
                     (force-list-items
                      (list-value (second args) "zipAttrsWith")))
          names (sort (set (mapcat #(keys (:entries %)) rows)))]
      (attrset-value
       (into {}
             (map (fn [n]
                    (let [vals (vec
                                (keep (fn [row]
                                        (when (contains? (:entries row) n)
                                          (get (:entries row) n)))
                                      rows))]
                      [n (thunk #(apply-callable2 f n vals))])))
             names)))

    :intersectAttrs
    (let [left (require-attrset (first args) "intersectAttrs")
          right (require-attrset (second args) "intersectAttrs")]
      (attrset-value
       (into {}
             (filter (fn [[k _]] (contains? (:entries left) k)))
             (:entries right))))

    :catAttrs
    (let [attr (string-value (first args) "catAttrs")
          sets (list-value (second args) "catAttrs")]
      (vec
       (keep (fn [s]
               (let [s (require-attrset s "catAttrs")]
                 (when (contains? (:entries s) attr)
                   (get (:entries s) attr))))
             sets)))

    ;; ---- Lists ----
    :length
    (i64 (count (list-value (first args) "length")))

    :head
    (let [xs (list-value (first args) "head")]
      (when (empty? xs)
        (outcome/fail! :eval :type-error
                       {:operation "head" :reason "empty-list"}))
      (force-value (first xs)))

    :tail
    (let [xs (list-value (first args) "tail")]
      (when (empty? xs)
        (outcome/fail! :eval :type-error
                       {:operation "tail" :reason "empty-list"}))
      (vec (rest xs)))

    :last
    (let [xs (list-value (first args) "last")]
      (when (empty? xs)
        (outcome/fail! :eval :type-error
                       {:operation "last" :reason "empty-list"}))
      (force-value (peek xs)))

    :init
    (let [xs (list-value (first args) "init")]
      (when (empty? xs)
        (outcome/fail! :eval :type-error
                       {:operation "init" :reason "empty-list"}))
      (subvec xs 0 (dec (count xs))))

    :elem
    (let [needle (first args)
          xs (list-value (second args) "elem")]
      ;; Element slots use shared-value identity (functions/NaN), like list ==.
      (boolean (some #(deep-equal? needle % true) xs)))

    :elemAt
    (let [xs (list-value (first args) "elemAt")
          idx (integer-value (second args) "elemAt")]
      (when (or (neg? idx) (>= idx (count xs)))
        (outcome/fail! :eval :type-error
                       {:operation "elemAt" :reason "index-out-of-bounds"}))
      (force-value (nth xs (int idx))))

    :concatLists
    (let [xss (list-value (first args) "concatLists")]
      (vec (mapcat (fn [xs]
                     (list-value xs "concatLists"))
                   xss)))

    :flatten
    (flatten-lists (first args))

    :concatMap
    (let [f (force-value (first args))
          xs (list-value (second args) "concatMap")]
      (vec (mapcat (fn [item]
                     (list-value (apply-callable f item) "concatMap"))
                   xs)))

    :genList
    (let [f (force-value (first args))
          n (integer-value (second args) "genList")]
      (when (neg? n)
        (outcome/fail! :eval :type-error
                       {:operation "genList" :reason "negative-length"}))
      (mapv (fn [i]
              (thunk #(apply-callable f (i64 i))))
            (range (int n))))

    :foldl
    (let [f (force-value (first args))
          acc (second args)
          xs (list-value (nth args 2) "foldl")]
      (loop [remaining xs
             acc acc]
        (if (empty? remaining)
          (force-value acc)
          (recur (rest remaining)
                 (apply-callable2 f acc (first remaining))))))

    :foldl'
    (let [f (force-value (first args))
          acc (second args)
          xs (list-value (nth args 2) "foldl'")]
      (loop [remaining xs
             acc acc]
        (if (empty? remaining)
          (force-value acc)
          (recur (rest remaining)
                 (force-value
                  (apply-callable2 f acc (first remaining)))))))

    :foldr
    (let [f (force-value (first args))
          z (second args)
          xs (list-value (nth args 2) "foldr")]
      (loop [remaining (reverse xs)
             acc z]
        (if (empty? remaining)
          (force-value acc)
          (recur (rest remaining)
                 (apply-callable2 f (first remaining) acc)))))

    :partition
    (partition-list (force-value (first args))
                    (list-value (second args) "partition"))

    :unique
    (let [xs (list-value (first args) "unique")]
      (loop [remaining xs
             out []]
        (if (empty? remaining)
          out
          (let [item (first remaining)]
            (if (some #(values-equal? item %) out)
              (recur (rest remaining) out)
              (recur (rest remaining) (conj out item)))))))

    :range
    (let [from (integer-value (first args) "range")
          to (integer-value (second args) "range")]
      (if (> from to)
        []
        (mapv i64 (range from (inc to)))))

    :sum
    (let [xs (list-value (first args) "sum")]
      (reduce (fn [acc x]
                (checked-add acc (integer-value x "sum")))
              (i64 0)
              xs))

    :product
    (let [xs (list-value (first args) "product")]
      (reduce (fn [acc x]
                (checked-multiply acc (integer-value x "product")))
              (i64 1)
              xs))

    :zipLists
    (let [xs (list-value (first args) "zipLists")
          ys (list-value (second args) "zipLists")]
      (mapv (fn [a b]
              (attrset-value {"fst" a "snd" b}))
            xs ys))

    :zipListsWith
    (let [f (force-value (first args))
          xs (list-value (second args) "zipListsWith")
          ys (list-value (nth args 2) "zipListsWith")]
      (mapv (fn [a b]
              (thunk #(apply-callable2 f a b)))
            xs ys))

    :intersectLists
    (let [e (list-value (first args) "intersectLists")
          xs (list-value (second args) "intersectLists")]
      (vec (filter (fn [item]
                     (some #(values-equal? item %) e))
                   xs)))

    :subtractLists
    (let [xs (list-value (first args) "subtractLists")
          e (list-value (second args) "subtractLists")]
      (vec (remove (fn [item]
                     (some #(values-equal? item %) e))
                   xs)))

    ;; ---- Strings ----
    :substring
    ;; Byte offsets over UTF-8 (Nix). Off-boundary cuts yield raw-bytes.
    (let [start (integer-value (first args) "substring")
          len (integer-value (second args) "substring")
          bs (as-utf8-bytes (nth args 2) "substring")
          n (alength bs)
          start (if (neg? start) 0 start)
          start (if (> start n) n start)
          end (if (neg? len)
                n
                (min n (+ start len)))
          slice (byte-array (int (- end start)))]
      (when (pos? (alength slice))
        (System.Array/Copy bs (int start) slice 0 (alength slice)))
      (decode-utf8-or-raw slice))

    :stringLength
    (i64 (alength (as-utf8-bytes (first args) "stringLength")))

    :concatStringsSep
    (let [sep (string-value (first args) "concatStringsSep")
          xs (force-list-items
              (list-value (second args) "concatStringsSep"))]
      (str/join sep (map nix-to-string xs)))

    :concatMapStringsSep
    (let [sep (string-value (first args) "concatMapStringsSep")
          f (force-value (second args))
          xs (list-value (nth args 2) "concatMapStringsSep")]
      (str/join sep
                (map (fn [item]
                       (nix-to-string (apply-callable f item)))
                     xs)))

    :replaceStrings
    (let [froms (force-list-items
                 (list-value (first args) "replaceStrings"))
          tos (force-list-items
               (list-value (second args) "replaceStrings"))
          s (string-value (nth args 2) "replaceStrings")]
      (when-not (and (every? string? froms) (every? string? tos)
                     (= (count froms) (count tos)))
        (outcome/fail! :eval :type-error
                       {:operation "replaceStrings"}))
      ;; Loop bound is INCLUSIVE of i = (count s): an empty `from` matches
      ;; at every position including the end, so
      ;; replaceStrings [""] ["x"] "ab" must reach "xaxbx", not "xaxb".
      (loop [i 0
             out ""]
        (if (> i (count s))
          out
          (let [matched
                (loop [j 0]
                  (if (>= j (count froms))
                    nil
                    (let [f (nth froms j)]
                      (if (<= (+ i (count f)) (count s))
                        (if (or (zero? (count f))
                                (= f (subs s i (+ i (count f)))))
                          j
                          (recur (inc j)))
                        (recur (inc j))))))]
            (if (nil? matched)
              (if (< i (count s))
                (recur (inc i) (str out (subs s i (inc i))))
                out)
              (let [f (nth froms matched)
                    t (nth tos matched)]
                (if (zero? (count f))
                  (if (< i (count s))
                    (recur (inc i) (str out t (subs s i (inc i))))
                    (str out t))
                  (recur (+ i (count f)) (str out t)))))))))

    :removePrefix
    (let [prefix (string-value (first args) "removePrefix")
          s (string-value (second args) "removePrefix")]
      (if (str/starts-with? s prefix)
        (subs s (count prefix))
        s))

    :removeSuffix
    (let [suffix (string-value (first args) "removeSuffix")
          s (string-value (second args) "removeSuffix")]
      (if (str/ends-with? s suffix)
        (subs s 0 (- (count s) (count suffix)))
        s))

    :hasPrefix
    (let [prefix (string-value (first args) "hasPrefix")
          s (string-value (second args) "hasPrefix")]
      (str/starts-with? s prefix))

    :hasSuffix
    (let [suffix (string-value (first args) "hasSuffix")
          s (string-value (second args) "hasSuffix")]
      (str/ends-with? s suffix))

    :splitString
    (let [sep (string-value (first args) "splitString")
          s (string-value (second args) "splitString")]
      (if (empty? sep)
        (mapv str s)
        (let [parts (System.Text.RegularExpressions.Regex/Split
                     s
                     (System.Text.RegularExpressions.Regex/Escape sep))]
          (vec parts))))

    :toLower
    (.ToLowerInvariant ^String (string-value (first args) "toLower"))

    :toUpper
    (.ToUpperInvariant ^String (string-value (first args) "toUpper"))

    :boolToString
    (let [v (force-value (first args))]
      (when-not (boolean? v)
        (outcome/fail! :eval :type-error
                       {:operation "boolToString" :expected "bool"}))
      (if v "true" "false"))

    :match
    (let [pattern (string-value (first args) "match")
          s (string-value (second args) "match")]
      (regex-match pattern s))

    :split
    (let [pattern (string-value (first args) "split")
          s (string-value (second args) "split")]
      (regex-split pattern s))

    ;; ---- Predicates ----
    :isAttrs (attrset? (force-value (first args)))
    :isBool (boolean? (force-value (first args)))
    :isInt (integer? (force-value (first args)))
    :isFloat (float-value? (force-value (first args)))
    :isString (string? (force-value (first args)))
    :isList (vector? (force-value (first args)))
    :isNull (nil? (force-value (first args)))
    :isFunction (callable? (force-value (first args)))
    :isPath (path-value? (force-value (first args)))

    ;; ---- Math ----
    :add
    (let [a (force-value (first args))
          b (force-value (second args))]
      (cond
        (and (integer? a) (integer? b)) (checked-add a b)
        (and (or (integer? a) (float-value? a))
             (or (integer? b) (float-value? b)))
        (nix-double (+ (float-double a) (float-double b)))
        :else
        (outcome/fail! :eval :type-error
                       {:operation "add" :expected "number"})))

    :sub
    (let [a (force-value (first args))
          b (force-value (second args))]
      (cond
        (and (integer? a) (integer? b)) (checked-subtract a b)
        (and (or (integer? a) (float-value? a))
             (or (integer? b) (float-value? b)))
        (nix-double (- (float-double a) (float-double b)))
        :else
        (outcome/fail! :eval :type-error
                       {:operation "sub" :expected "number"})))

    :mul
    (let [a (force-value (first args))
          b (force-value (second args))]
      (cond
        (and (integer? a) (integer? b)) (checked-multiply a b)
        (and (or (integer? a) (float-value? a))
             (or (integer? b) (float-value? b)))
        (nix-double (* (float-double a) (float-double b)))
        :else
        (outcome/fail! :eval :type-error
                       {:operation "mul" :expected "number"})))

    :div
    (let [a (force-value (first args))
          b (force-value (second args))]
      (cond
        (and (integer? a) (integer? b)) (checked-divide a b)
        (and (or (integer? a) (float-value? a))
             (or (integer? b) (float-value? b)))
        (let [da (float-double a) db (float-double b)]
          ;; Guest corpus pins float /0 as eval failure (not IEEE Inf).
          (if (zero? db)
            (outcome/fail! :eval :division-by-zero {:operator "/"})
            (nix-double (/ da db))))
        :else
        (outcome/fail! :eval :type-error
                       {:operation "div" :expected "number"})))

    :lessThan
    (let [a (force-value (first args))
          b (force-value (second args))]
      (cond
        (and (integer? a) (integer? b)) (< (long a) (long b))
        (and (or (integer? a) (float-value? a))
             (or (integer? b) (float-value? b)))
        (< (float-double a) (float-double b))
        :else
        (outcome/fail! :eval :type-error
                       {:operation "lessThan" :expected "number"})))

    :min
    (let [a (integer-value (first args) "min")
          b (integer-value (second args) "min")]
      (if (<= a b) a b))

    :max
    (let [a (integer-value (first args) "max")
          b (integer-value (second args) "max")]
      (if (>= a b) a b))

    :mod
    (let [a (integer-value (first args) "mod")
          b (integer-value (second args) "mod")]
      (when (zero? b)
        (outcome/fail! :eval :division-by-zero {:operator "mod"}))
      (rem a b))

    :abs
    (let [v (integer-value (first args) "abs")]
      (if (neg? v) (checked-negate v) v))

    :ceil
    (round-to-int "ceil" (force-value (first args)) true)

    :floor
    (round-to-int "floor" (force-value (first args)) false)

    :fromJSON
    (let [raw (json/read-json (string-value (first args) "fromJSON"))]
      (letfn [(wrap [v]
                (cond
                  (nil? v) nil
                  (or (boolean? v) (number? v) (string? v)) v
                  (vector? v) (mapv wrap v)
                  (map? v)
                  (attrset-value
                   (into {} (map (fn [[k x]] [(str k) (wrap x)]) v)))
                  :else v))]
        (wrap raw)))

    :break
    ;; Minimal: identity (full filter/map break remains future work).
    (force-value (first args))

    :parseDrvName
    ;; Nix: split at the first '-' followed by a digit (version start).
    (let [s (string-value (first args) "parseDrvName")
          n (count s)
          idx (loop [i 0]
                (cond
                  (>= i n) -1
                  (and (= \- (nth s i))
                       (< (inc i) n)
                       (System.Char/IsDigit ^char (nth s (inc i))))
                  i
                  :else (recur (inc i))))]
      (if (neg? idx)
        (attrset-value {"name" s "version" ""})
        (attrset-value {"name" (subs s 0 idx)
                        "version" (subs s (inc idx))})))

    :unsafeDiscardStringContext
    (force-value (first args))

    :unsafeDiscardOutputDependency
    (force-value (first args))

    ;; ---- Combinators ----
    :optional
    (if (boolean-value (first args) "optional")
      [(second args)]
      [])

    :optionals
    (if (boolean-value (first args) "optionals")
      (list-value (second args) "optionals")
      [])

    :optionalAttrs
    (if (boolean-value (first args) "optionalAttrs")
      (require-attrset (second args) "optionalAttrs")
      (attrset-value {}))

    :optionalString
    (if (boolean-value (first args) "optionalString")
      (nix-to-string (second args))
      "")

    :implies
    (if (boolean-value (first args) "implies")
      (boolean-value (second args) "implies")
      true)

    :when
    (when (boolean-value (first args) "when")
      (force-value (second args)))

    :id (force-value (first args))

    :const (force-value (first args))

    :flip
    (apply-callable2 (force-value (first args))
                     (nth args 2)
                     (second args))

    :pipe
    (let [v (first args)
          fns (list-value (second args) "pipe")]
      (loop [remaining fns
             acc v]
        (if (empty? remaining)
          (force-value acc)
          (recur (rest remaining)
                 (apply-callable (force-value (first remaining)) acc)))))

    :fix
    (let [f (force-value (first args))
          x-atom (atom nil)
          x (thunk #(apply-callable f @x-atom))]
      (reset! x-atom x)
      (force-value x))

    :assertMsg
    (if (boolean-value (first args) "assertMsg")
      true
      (outcome/fail! :eval :throw
                     {:message (nix-to-string (second args))}))

    ;; ---- FS ----
    :readFile
    (host/read-file-text (path-or-string (first args) "readFile"))

    :readDir
    (let [entries (host/list-directory
                   (path-or-string (first args) "readDir"))]
      (attrset-value
       (into {} (map (fn [[k v]] [k v]) entries))))

    :pathExists
    (host/path-exists (path-or-string (first args) "pathExists"))

    :getEnv
    (or (System.Environment/GetEnvironmentVariable
         (string-value (first args) "getEnv"))
        "")

    ;; ---- Fetch (best-effort) ----
    :fetchurl
    (let [spec (fetch-url-arg (first args))]
      (when (or (nil? spec) (nil? (:url spec)))
        (outcome/fail! :eval :type-error
                       {:operation "fetchurl" :reason "bad-arg"}))
      (download-url! (:url spec) (:name spec)))

    :fetchTarball
    (let [spec (fetch-url-arg (first args))]
      (when (or (nil? spec) (nil? (:url spec)))
        (outcome/fail! :eval :type-error
                       {:operation "fetchTarball" :reason "bad-arg"}))
      (download-url! (:url spec) (or (:name spec) "tarball")))

    :fetchGit
    (fetch-git! (first args))

    ;; map/filter helpers used at top-level aliases
    :map
    (let [f (force-value (first args))
          xs (list-value (second args) "map")]
      (mapv (fn [item]
              (thunk #(apply-callable f item)))
            xs))

    :filter
    (let [pred (force-value (first args))
          xs (list-value (second args) "filter")]
      (vec (filter (fn [item]
                     (boolean-value (apply-callable pred item) "filter"))
                   xs)))

    :any
    (let [pred (force-value (first args))
          xs (list-value (second args) "any")]
      (boolean
       (some (fn [item]
               (boolean-value (apply-callable pred item) "any"))
             xs)))

    :all
    (let [pred (force-value (first args))
          xs (list-value (second args) "all")]
      (every? (fn [item]
                (boolean-value (apply-callable pred item) "all"))
              xs))

    :sort
    ;; Comparator is (a: b: a < b) style; true means a sorts before b.
    ;; Empty list short-circuits without forcing the comparator (Nix).
    (let [xs (list-value (second args) "sort")]
      (if (empty? xs)
        []
        (let [cmp (force-value (first args))
              forced (mapv force-value xs)]
          (vec
           (sort (fn [a b]
                   (if (boolean-value (apply-callable2 cmp a b) "sort")
                     -1
                     1))
                 forced)))))

    :toPath
    ;; Lexical absolute-path normalization; returns a string (Nix toPath).
    (let [s (path-or-string (first args) "toPath")]
      (when-not (str/starts-with? s "/")
        (outcome/fail! :eval :type-error
                       {:operation "toPath"
                        :reason "not-absolute-path"
                        :value s}))
      (let [parts (reduce (fn [out part]
                            (cond
                              (or (= part "") (= part ".")) out
                              (= part "..") (if (seq out) (pop out) out)
                              :else (conj out part)))
                          []
                          (str/split s #"/"))]
        (str "/" (str/join "/" parts))))

    :hashString
    ;; UTF-8 / raw-bytes → lowercase hex. Supported: md5 sha1 sha256 sha512.
    (let [algo (string-value (force-value (first args)) "hashString")
          bytes (as-utf8-bytes (second args) "hashString")
          algo-key (str/lower-case algo)
          hash-bytes
          (case algo-key
            "md5" (.ComputeHash (System.Security.Cryptography.MD5/Create) bytes)
            "sha1" (.ComputeHash (System.Security.Cryptography.SHA1/Create) bytes)
            "sha256" (.ComputeHash (System.Security.Cryptography.SHA256/Create) bytes)
            "sha512" (.ComputeHash (System.Security.Cryptography.SHA512/Create) bytes)
            (outcome/fail! :eval :type-error
                           {:operation "hashString"
                            :reason "unsupported-algorithm"
                            :algorithm algo}))]
      (-> (System.BitConverter/ToString hash-bytes)
          (str/replace "-" "")
          str/lower-case))

    ;; ---- Extended builtins (maturity pass 2026-08-11) ----
    :pow
    (let [a (force-value (first args)) b (force-value (second args))]
      (if (and (integer? a) (integer? b))
        (i64 (long (Math/Pow (double a) (double b))))
        (nix-double (Math/Pow (float-double a) (float-double b)))))

    :sqrt
    (nix-double (Math/Sqrt (float-double (force-value (first args)))))

    :exp
    (nix-double (Math/Exp (float-double (force-value (first args)))))

    :ln
    (nix-double (Math/Log (float-double (force-value (first args)))))

    :sin
    (nix-double (Math/Sin (float-double (force-value (first args)))))

    :cos
    (nix-double (Math/Cos (float-double (force-value (first args)))))

    :atan2
    (nix-double (Math/Atan2 (float-double (force-value (first args)))
                            (float-double (force-value (second args)))))

    :bitAnd
    (i64 (bit-and (integer-value (first args) "bitAnd")
                  (integer-value (second args) "bitAnd")))

    :bitOr
    (i64 (bit-or (integer-value (first args) "bitOr")
                 (integer-value (second args) "bitOr")))

    :bitXor
    (i64 (bit-xor (integer-value (first args) "bitXor")
                  (integer-value (second args) "bitXor")))

    :and
    (boolean (and (boolean-value (first args) "and")
                  (boolean-value (second args) "and")))

    :or
    (boolean (or (boolean-value (first args) "or")
                 (boolean-value (second args) "or")))

    :not
    (not (boolean-value (first args) "not"))

    :eq
    (deep-equal? (first args) (second args))

    :lt
    (let [a (force-value (first args)) b (force-value (second args))]
      (cond
        (and (integer? a) (integer? b)) (< (long a) (long b))
        (and (or (integer? a) (float-value? a))
             (or (integer? b) (float-value? b)))
        (< (float-double a) (float-double b))
        :else (outcome/fail! :eval :type-error
                             {:operation "lt" :expected "number"})))

    :le
    (not (exec-builtin {:name :lt :args [(second args) (first args)]}))

    :gt
    (exec-builtin {:name :lt :args [(second args) (first args)]})

    :ge
    (not (exec-builtin {:name :lt :args [(first args) (second args)]}))

    :neg
    (let [v (force-value (first args))]
      (if (integer? v)
        (checked-negate v)
        (nix-double (- (float-double v)))))

    :get
    (let [attrs (require-attrset (first args) "get")
          attr (string-value (second args) "get")]
      (when-not (contains? (:entries attrs) attr)
        (outcome/fail! :eval :attribute-missing {:attribute attr}))
      (force-value (get (:entries attrs) attr)))

    :set
    (let [attrs (require-attrset (first args) "set")
          attr (string-value (second args) "set")]
      (attrset-value (assoc (:entries attrs) attr (nth args 2))))

    :keys
    (sorted-attr-keys (require-attrset (first args) "keys"))

    :values
    (let [attrs (require-attrset (first args) "values")]
      (mapv #(get (:entries attrs) %) (sorted-attr-keys attrs)))

    :merge
    (let [left (require-attrset (first args) "merge")
          right (require-attrset (second args) "merge")]
      (attrset-value (merge (:entries left) (:entries right))))

    :genAttrs
    (let [names (force-list-items (list-value (first args) "genAttrs"))
          f (force-value (second args))]
      (when-not (every? string? names)
        (outcome/fail! :eval :type-error
                       {:operation "genAttrs" :expected "string-list"}))
      (attrset-value
       (into {} (map (fn [n] [n (thunk #(apply-callable f n))])) names)))

    :nameValuePair
    (attrset-value {"name" (force-value (first args)) "value" (second args)})

    :mapAttrsToList
    (let [f (force-value (first args))
          attrs (require-attrset (second args) "mapAttrsToList")]
      (mapv (fn [k]
              (thunk #(apply-callable2 f k (get (:entries attrs) k))))
            (sorted-attr-keys attrs)))

    :mapAttrsRecursive
    (letfn [(walk [f attrs path]
              (attrset-value
               (into {}
                     (map (fn [[k v]]
                            (let [v* (force-value v)
                                  path* (conj path k)]
                              [k (if (attrset? v*)
                                   (walk f v* path*)
                                   (thunk #(apply-callable2 f (mapv identity path*) v)))])))
                     (:entries attrs))))]
      (walk (force-value (first args)) (require-attrset (second args) "mapAttrsRecursive") []))

    :filterAttrsRecursive
    (letfn [(walk [pred attrs]
              (attrset-value
               (into {}
                     (keep (fn [[k v]]
                             (when (boolean-value (apply-callable2 pred k v) "filterAttrsRecursive")
                               (let [v* (force-value v)]
                                 [k (if (attrset? v*) (walk pred v*) v)]))))
                     (:entries attrs))))]
      (walk (force-value (first args)) (require-attrset (second args) "filterAttrsRecursive")))

    :foldlAttrs
    (let [f (force-value (first args))
          acc (second args)
          attrs (require-attrset (nth args 2) "foldlAttrs")]
      (loop [remaining (sorted-attr-keys attrs) acc acc]
        (if (empty? remaining)
          (force-value acc)
          (recur (rest remaining)
                 (apply-callable (apply-callable (apply-callable f acc) (first remaining))
                                 (get (:entries attrs) (first remaining)))))))

    :getAttrFromPathOr
    (attr-by-path (list-value (second args) "getAttrFromPathOr")
                  (nth args 2)
                  (require-attrset (first args) "getAttrFromPathOr"))

    :getName
    (let [x (force-value (first args))]
      (cond
        (string? x) (first (str/split x #"-\d" 2))
        (attrset? x)
        (let [pname (get (:entries x) "pname")]
          (if pname
            (force-value pname)
            (let [n (force-value (get (:entries x) "name"))]
              (first (str/split (str n) #"-\d" 2)))))
        :else (outcome/fail! :eval :type-error {:operation "getName"})))

    :getVersion
    (let [x (force-value (first args))
          drv-name (fn [s] (let [m (re-matches #"(.+?)-(\d.*)" s)] (if m (nth m 2) "")))]
      (cond
        (string? x) (drv-name x)
        (attrset? x)
        (let [ver (get (:entries x) "version")]
          (if ver
            (force-value ver)
            (drv-name (str (force-value (get (:entries x) "name"))))))
        :else (outcome/fail! :eval :type-error {:operation "getVersion"})))

    :updateManyAttrs
    (let [updates (force-list-items (list-value (first args) "updateManyAttrs"))
          base (require-attrset (second args) "updateManyAttrs")]
      (attrset-value
       (reduce (fn [acc item]
                 (merge acc (:entries (require-attrset item "updateManyAttrs"))))
               (:entries base)
               updates)))

    :unsafeGetAttrPos
    nil

    :seq
    (do (force-value (first args)) (second args))

    :deepSeq
    (letfn [(deep! [v]
              (let [v (force-value v)]
                (cond
                  (vector? v) (run! deep! v)
                  (attrset? v) (run! deep! (vals (:entries v))))
                v))]
      (deep! (first args))
      (second args))

    :drop
    (let [n (integer-value (first args) "drop")
          xs (list-value (second args) "drop")]
      (vec (drop n xs)))

    :take
    (let [n (integer-value (first args) "take")
          xs (list-value (second args) "take")]
      (vec (take n xs)))

    :cons
    (vec (cons (first args) (list-value (second args) "cons")))

    :append
    (vec (concat (list-value (first args) "append") (list-value (second args) "append")))

    :zip
    (mapv vector (list-value (first args) "zip") (list-value (second args) "zip"))

    :zipAttrs
    (let [rows (mapv #(require-attrset % "zipAttrs")
                     (force-list-items (list-value (first args) "zipAttrs")))
          names (sort (set (mapcat #(keys (:entries %)) rows)))]
      (attrset-value
       (into {}
             (map (fn [n]
                    [n (vec (keep (fn [row]
                                    (when (contains? (:entries row) n)
                                      (get (:entries row) n)))
                                  rows))]))
             names)))

    :reverseList
    (vec (reverse (list-value (first args) "reverseList")))

    :replicate
    (let [n (integer-value (first args) "replicate")]
      (vec (repeat n (second args))))

    :findFirst
    (let [pred (force-value (first args))
          default (second args)
          xs (list-value (nth args 2) "findFirst")]
      (loop [remaining xs]
        (if (empty? remaining)
          (force-value default)
          (if (boolean-value (apply-callable pred (first remaining)) "findFirst")
            (force-value (first remaining))
            (recur (rest remaining))))))

    :find
    (let [needle (first args)
          xs (list-value (second args) "find")]
      (loop [remaining xs]
        (if (empty? remaining)
          nil
          (if (deep-equal? needle (first remaining))
            (force-value (first remaining))
            (recur (rest remaining))))))

    :imap0
    (let [f (force-value (first args))
          xs (list-value (second args) "imap0")]
      (mapv (fn [i x] (thunk #(apply-callable2 f (i64 i) x))) (range) xs))

    :imap1
    (let [f (force-value (first args))
          xs (list-value (second args) "imap1")]
      (mapv (fn [i x] (thunk #(apply-callable2 f (i64 i) x))) (iterate inc 1) xs))

    :stringToCharacters
    (mapv str (string-value (first args) "stringToCharacters"))

    :groupBy
    (let [f (force-value (first args))
          xs (list-value (second args) "groupBy")]
      (attrset-value
       (group-by (fn [x] (string-value (apply-callable f x) "groupBy")) xs)))

    :functionArgs
    (let [f (force-value (first args))]
      (if (and (closure? f) (:param-pattern f))
        (attrset-value
         (into {}
               (map (fn [p] [(:name p) (boolean (contains? p :default))]))
               (:params (:param-pattern f))))
        (attrset-value {})))

    :compareVersions
    (let [a (string-value (first args) "compareVersions")
          b (string-value (second args) "compareVersions")
          parts (fn [s] (str/split s #"[.\-]"))
          cmp1 (fn [x y]
                 (let [xn (re-matches #"\d+" x) yn (re-matches #"\d+" y)]
                   (cond
                     (and xn yn) (compare (Int64/Parse x) (Int64/Parse y))
                     :else (compare x y))))]
      (i64 (loop [xs (parts a) ys (parts b)]
             (cond
               (and (empty? xs) (empty? ys)) 0
               (empty? xs) -1
               (empty? ys) 1
               :else (let [c (cmp1 (first xs) (first ys))]
                       (if (zero? c) (recur (rest xs) (rest ys)) c))))))

    :splitVersion
    (vec (str/split (string-value (first args) "splitVersion") #"[.\-]"))

    :dirOf
    (let [s (string-value (first args) "dirOf")
          i (str/last-index-of s "/")]
      (cond
        (nil? i) "."
        (zero? i) "/"
        :else (subs s 0 i)))

    :baseNameOf
    (last (str/split (string-value (first args) "baseNameOf") #"/"))

    :toInt
    (try
      (i64 (Int64/Parse (str/trim (string-value (first args) "toInt"))))
      (catch System.Exception _
        (outcome/fail! :eval :type-error {:operation "toInt" :reason "not-an-integer"})))

    :hasInfix
    (str/includes? (string-value (second args) "hasInfix")
                   (string-value (first args) "hasInfix"))

    :concatMapStrings
    (let [f (force-value (first args))
          xs (list-value (second args) "concatMapStrings")]
      (apply str (map (fn [x] (nix-to-string (apply-callable f x))) xs)))

    :concatStrings
    (apply str (map nix-to-string (list-value (first args) "concatStrings")))

    :placeholder
    ;; Deterministic context-free placeholder for an output name, replaced at
    ;; build time in real Nix. Pseudo hash, not byte-compatible with Nix.
    (let [output (string-value (first args) "placeholder")
          bytes (as-utf8-bytes (str "pnix-output:" output) "placeholder")
          digest (.ComputeHash (System.Security.Cryptography.SHA256/Create) bytes)
          hex (-> (System.BitConverter/ToString digest)
                  (str/replace "-" "")
                  str/lower-case)]
      (str "/" (subs hex 0 32)))

    :storePath
    (outcome/fail! :eval :type-error
                   {:operation "storePath" :reason "pure-evaluator-no-store"})

    :pnixMounts
    (outcome/fail! :eval :type-error
                   {:operation "pnixMounts" :reason "extension-not-wired"
                    :nix-builtin? false})

    :addErrorContext
    (second args)

    :genericClosure
    (let [arg (require-attrset (first args) "genericClosure")
          operator (force-value (get (:entries arg) "operator"))
          start-set (force-list-items (list-value (get (:entries arg) "startSet") "genericClosure"))]
      (loop [worklist (vec start-set) seen #{} result []]
        (if (empty? worklist)
          result
          (let [item (require-attrset (first worklist) "genericClosure")
                key (force-value (get (:entries item) "key"))]
            (if (contains? seen key)
              (recur (rest worklist) seen result)
              (let [next-items (force-list-items
                                 (list-value (apply-callable operator item) "genericClosure"))]
                (recur (into (vec (rest worklist)) next-items)
                       (conj seen key)
                       (conj result item))))))))

    (outcome/fail! :eval :unsupported-expression
                   {:builtin (name name)})))

;; ---------------------------------------------------------------------------
;; Root environment: builtins + lib
;; ---------------------------------------------------------------------------

(defn- bi
  [n a]
  (builtin-value n a))

(defn- builtins-entries
  []
  {"typeOf" (bi :typeOf 1)
   "tryEval" (bi :tryEval 1)
   "throw" (bi :throw 1)
   "abort" (bi :abort 1)
   "trace" (bi :trace 2)
   "warn" (bi :warn 2)
   "toString" (bi :toString 1)
   "toJSON" (bi :toJSON 1)
   "fromJSON" (bi :fromJSON 1)
   "toXML" (bi :toXML 1)
   ;; Presence stubs / minimal surface (full semantics may remain shallow).
   "break" (bi :break 1)
   "parseDrvName" (bi :parseDrvName 1)
   "unsafeDiscardStringContext" (bi :unsafeDiscardStringContext 1)
   "unsafeDiscardOutputDependency" (bi :unsafeDiscardOutputDependency 1)
   "toFile" (bi :toFile 2)
   "attrNames" (bi :attrNames 1)
   "attrValues" (bi :attrValues 1)
   "hasAttr" (bi :hasAttr 2)
   "getAttr" (bi :getAttr 2)
   "getAttrFromPath" (bi :getAttrFromPath 2)
   "hasAttrByPath" (bi :hasAttrByPath 2)
   "attrByPath" (bi :attrByPath 3)
   "mapAttrs" (bi :mapAttrs 2)
   "filterAttrs" (bi :filterAttrs 2)
   "listToAttrs" (bi :listToAttrs 1)
   "removeAttrs" (bi :removeAttrs 2)
   "recursiveUpdate" (bi :recursiveUpdate 2)
   "zipAttrsWith" (bi :zipAttrsWith 2)
   "intersectAttrs" (bi :intersectAttrs 2)
   "catAttrs" (bi :catAttrs 2)
   "length" (bi :length 1)
   "head" (bi :head 1)
   "tail" (bi :tail 1)
   "last" (bi :last 1)
   "init" (bi :init 1)
   "elem" (bi :elem 2)
   "elemAt" (bi :elemAt 2)
   "concatLists" (bi :concatLists 1)
   "flatten" (bi :flatten 1)
   "concatMap" (bi :concatMap 2)
   "genList" (bi :genList 2)
   "foldl" (bi :foldl 3)
   "foldl'" (bi :foldl' 3)
   "foldr" (bi :foldr 3)
   "partition" (bi :partition 2)
   "unique" (bi :unique 1)
   "range" (bi :range 2)
   "sum" (bi :sum 1)
   "product" (bi :product 1)
   "zipLists" (bi :zipLists 2)
   "zipListsWith" (bi :zipListsWith 3)
   "intersectLists" (bi :intersectLists 2)
   "subtractLists" (bi :subtractLists 2)
   "substring" (bi :substring 3)
   "stringLength" (bi :stringLength 1)
   "concatStringsSep" (bi :concatStringsSep 2)
   "concatMapStringsSep" (bi :concatMapStringsSep 3)
   "replaceStrings" (bi :replaceStrings 3)
   "removePrefix" (bi :removePrefix 2)
   "removeSuffix" (bi :removeSuffix 2)
   "hasPrefix" (bi :hasPrefix 2)
   "hasSuffix" (bi :hasSuffix 2)
   "splitString" (bi :splitString 2)
   "toLower" (bi :toLower 1)
   "toUpper" (bi :toUpper 1)
   "boolToString" (bi :boolToString 1)
   "match" (bi :match 2)
   "split" (bi :split 2)
   "isAttrs" (bi :isAttrs 1)
   "isBool" (bi :isBool 1)
   "isInt" (bi :isInt 1)
   "isFloat" (bi :isFloat 1)
   "isString" (bi :isString 1)
   "isList" (bi :isList 1)
   "isNull" (bi :isNull 1)
   "isFunction" (bi :isFunction 1)
   "isPath" (bi :isPath 1)
   "add" (bi :add 2)
   "sub" (bi :sub 2)
   "mul" (bi :mul 2)
   "div" (bi :div 2)
   "lessThan" (bi :lessThan 2)
   "min" (bi :min 2)
   "max" (bi :max 2)
   "mod" (bi :mod 2)
   "abs" (bi :abs 1)
   "ceil" (bi :ceil 1)
   "floor" (bi :floor 1)
   "optional" (bi :optional 2)
   "optionals" (bi :optionals 2)
   "optionalAttrs" (bi :optionalAttrs 2)
   "optionalString" (bi :optionalString 2)
   "implies" (bi :implies 2)
   "when" (bi :when 2)
   "id" (bi :id 1)
   "const" (bi :const 2)
   "flip" (bi :flip 3)
   "pipe" (bi :pipe 2)
   "fix" (bi :fix 1)
   "assertMsg" (bi :assertMsg 2)
   "readFile" (bi :readFile 1)
   "readDir" (bi :readDir 1)
   "pathExists" (bi :pathExists 1)
   "getEnv" (bi :getEnv 1)
   "fetchurl" (bi :fetchurl 1)
   "fetchTarball" (bi :fetchTarball 1)
   "fetchGit" (bi :fetchGit 1)
   "map" (bi :map 2)
   "filter" (bi :filter 2)
   "any" (bi :any 2)
   "all" (bi :all 2)
   "sort" (bi :sort 2)
   "toPath" (bi :toPath 1)
   "hashString" (bi :hashString 2)
   "true" true
   "false" false
   "null" nil
   "nixVersion" "2.34.7"
   "langVersion" (i64 6)
   "storeDir" "/nix/store"

   ;; ---- Extended builtins (maturity pass 2026-08-11) ----
   "pow" (bi :pow 2)
   "sqrt" (bi :sqrt 1)
   "exp" (bi :exp 1)
   "ln" (bi :ln 1)
   "sin" (bi :sin 1)
   "cos" (bi :cos 1)
   "atan2" (bi :atan2 2)
   "bitAnd" (bi :bitAnd 2)
   "bitOr" (bi :bitOr 2)
   "bitXor" (bi :bitXor 2)
   "and" (bi :and 2)
   "or" (bi :or 2)
   "not" (bi :not 1)
   "eq" (bi :eq 2)
   "lt" (bi :lt 2)
   "le" (bi :le 2)
   "gt" (bi :gt 2)
   "ge" (bi :ge 2)
   "neg" (bi :neg 1)
   "get" (bi :get 2)
   "set" (bi :set 3)
   "keys" (bi :keys 1)
   "values" (bi :values 1)
   "merge" (bi :merge 2)
   "genAttrs" (bi :genAttrs 2)
   "nameValuePair" (bi :nameValuePair 2)
   "mapAttrsToList" (bi :mapAttrsToList 2)
   "mapAttrsRecursive" (bi :mapAttrsRecursive 2)
   "filterAttrsRecursive" (bi :filterAttrsRecursive 2)
   "foldlAttrs" (bi :foldlAttrs 3)
   "getAttrFromPathOr" (bi :getAttrFromPathOr 3)
   "getName" (bi :getName 1)
   "getVersion" (bi :getVersion 1)
   "updateManyAttrs" (bi :updateManyAttrs 2)
   "unsafeGetAttrPos" (bi :unsafeGetAttrPos 2)
   "seq" (bi :seq 2)
   "deepSeq" (bi :deepSeq 2)
   "drop" (bi :drop 2)
   "take" (bi :take 2)
   "cons" (bi :cons 2)
   "append" (bi :append 2)
   "zip" (bi :zip 2)
   "zipAttrs" (bi :zipAttrs 1)
   "reverseList" (bi :reverseList 1)
   "replicate" (bi :replicate 2)
   "findFirst" (bi :findFirst 3)
   "find" (bi :find 2)
   "imap0" (bi :imap0 2)
   "imap1" (bi :imap1 2)
   "stringToCharacters" (bi :stringToCharacters 1)
   "groupBy" (bi :groupBy 2)
   "functionArgs" (bi :functionArgs 1)
   "compareVersions" (bi :compareVersions 2)
   "splitVersion" (bi :splitVersion 1)
   "dirOf" (bi :dirOf 1)
   "baseNameOf" (bi :baseNameOf 1)
   "toInt" (bi :toInt 1)
   "hasInfix" (bi :hasInfix 2)
   "concatMapStrings" (bi :concatMapStrings 2)
   "concatStrings" (bi :concatStrings 1)
   "placeholder" (bi :placeholder 1)
   "storePath" (bi :storePath 1)
   "pnixMounts" (bi :pnixMounts 0)
   "addErrorContext" (bi :addErrorContext 2)
   "genericClosure" (bi :genericClosure 1)})

(defn- make-builtins
  []
  (let [base (builtins-entries)
        holder (atom nil)
        entries (assoc base "builtins" (thunk #(force-value @holder)))
        value (attrset-value entries)]
    (reset! holder value)
    value))

(defn- make-lib
  [builtins]
  (let [fields (:entries builtins)
        g (fn [k] (get fields k))
        attrsets (attrset-value
                  {"isAttrs" (g "isAttrs")
                   "attrNames" (g "attrNames")
                   "attrValues" (g "attrValues")
                   "hasAttr" (g "hasAttr")
                   "getAttr" (g "getAttr")
                   "getAttrFromPath" (g "getAttrFromPath")
                   "hasAttrByPath" (g "hasAttrByPath")
                   "attrByPath" (g "attrByPath")
                   "mapAttrs" (g "mapAttrs")
                   "filterAttrs" (g "filterAttrs")
                   "recursiveUpdate" (g "recursiveUpdate")
                   "zipAttrsWith" (g "zipAttrsWith")
                   "optionalAttrs" (g "optionalAttrs")
                   "listToAttrs" (g "listToAttrs")
                   "catAttrs" (g "catAttrs")
                   "intersectAttrs" (g "intersectAttrs")
                   "removeAttrs" (g "removeAttrs")})
        lists (attrset-value
               {"flatten" (g "flatten")
                "unique" (g "unique")
                "intersectLists" (g "intersectLists")
                "subtractLists" (g "subtractLists")
                "zipLists" (g "zipLists")
                "zipListsWith" (g "zipListsWith")
                "partition" (g "partition")
                "range" (g "range")
                "last" (g "last")
                "init" (g "init")
                "foldl" (g "foldl")
                "foldr" (g "foldr")
                "sum" (g "sum")
                "product" (g "product")
                "head" (g "head")
                "tail" (g "tail")
                "length" (g "length")
                "elem" (g "elem")
                "elemAt" (g "elemAt")
                "concatLists" (g "concatLists")
                "concatMap" (g "concatMap")
                "genList" (g "genList")})
        strings (attrset-value
                 {"hasPrefix" (g "hasPrefix")
                  "hasSuffix" (g "hasSuffix")
                  "removePrefix" (g "removePrefix")
                  "removeSuffix" (g "removeSuffix")
                  "splitString" (g "splitString")
                  "toLower" (g "toLower")
                  "toUpper" (g "toUpper")
                  "concatMapStringsSep" (g "concatMapStringsSep")
                  "concatStringsSep" (g "concatStringsSep")
                  "optionalString" (g "optionalString")
                  "stringLength" (g "stringLength")
                  "substring" (g "substring")
                  "replaceStrings" (g "replaceStrings")
                  "boolToString" (g "boolToString")})
        extras {"attrsets" attrsets
                "lists" lists
                "strings" strings
                "implies" (g "implies")
                "optionalAttrs" (g "optionalAttrs")
                "when" (g "when")
                "const" (g "const")
                "fix" (g "fix")
                "sum" (g "sum")
                "product" (g "product")
                "assertMsg" (g "assertMsg")
                "getAttrFromPath" (g "getAttrFromPath")
                "hasAttrByPath" (g "hasAttrByPath")
                "intersectLists" (g "intersectLists")
                "subtractLists" (g "subtractLists")
                "zipLists" (g "zipLists")
                "id" (g "id")
                "flip" (g "flip")
                "pipe" (g "pipe")
                "min" (g "min")
                "max" (g "max")
                "optional" (g "optional")
                "optionals" (g "optionals")
                "filterAttrs" (g "filterAttrs")
                "flatten" (g "flatten")
                "last" (g "last")
                "init" (g "init")
                "unique" (g "unique")
                "recursiveUpdate" (g "recursiveUpdate")
                "hasPrefix" (g "hasPrefix")
                "hasSuffix" (g "hasSuffix")
                "removePrefix" (g "removePrefix")
                "removeSuffix" (g "removeSuffix")
                "splitString" (g "splitString")
                "toLower" (g "toLower")
                "toUpper" (g "toUpper")
                "boolToString" (g "boolToString")
                "concatMapStringsSep" (g "concatMapStringsSep")
                "foldl" (g "foldl")
                "foldr" (g "foldr")
                "range" (g "range")
                "partition" (g "partition")
                "zipListsWith" (g "zipListsWith")
                "isAttrs" (g "isAttrs")
                "head" (g "head")
                "tail" (g "tail")
                "length" (g "length")}]
    (attrset-value (merge fields extras))))

(defn root-environment
  "Outer frames for every evaluation: builtins, lib, and Nix default aliases."
  []
  (let [builtins (make-builtins)
        fields (:entries builtins)
        lib (make-lib builtins)
        frame (atom {"builtins" builtins
                     "lib" lib
                     "map" (get fields "map")
                     "throw" (get fields "throw")
                     "toString" (get fields "toString")
                     "abort" (get fields "abort")
                     "isNull" (get fields "isNull")
                     "removeAttrs" (get fields "removeAttrs")})]
    [frame]))

;; ---------------------------------------------------------------------------
;; AST evaluation
;; ---------------------------------------------------------------------------

(defn- eval-binary
  [operator left-ast right-ast environment context]
  (cond
    (= operator :and)
    (let [left (boolean-value (eval-ast* left-ast environment context) "&&")]
      (if left
        (boolean-value (eval-ast* right-ast environment context) "&&")
        false))

    (= operator :or)
    (let [left (boolean-value (eval-ast* left-ast environment context) "||")]
      (if left
        true
        (boolean-value (eval-ast* right-ast environment context) "||")))

    (= operator :plus)
    ;; Nix `+`: string/path concat when either side is string-like;
    ;; else numeric (int/float), with Int64 checked add when both ints.
    (let [left (force-value (eval-ast* left-ast environment context))
          right (force-value (eval-ast* right-ast environment context))]
      (cond
        (or (string? left) (string? right)
            (path-value? left) (path-value? right)
            (raw-bytes? left) (raw-bytes? right))
        (str (nix-to-string left) (nix-to-string right))
        (and (integer? left) (integer? right))
        (checked-add left right)
        (and (or (integer? left) (float-value? left))
             (or (integer? right) (float-value? right)))
        (nix-double (+ (float-double left) (float-double right)))
        :else
        (outcome/fail! :eval :type-error
                       {:operator "+" :expected "number-or-string"})))

    (contains? #{:minus :multiply :divide} operator)
    (let [operator-name (get {:minus "-"
                              :multiply "*" :divide "/"}
                             operator)
          left (force-value (eval-ast* left-ast environment context))
          right (force-value (eval-ast* right-ast environment context))]
      (cond
        (and (integer? left) (integer? right))
        (case operator
          :minus (checked-subtract left right)
          :multiply (checked-multiply left right)
          :divide (checked-divide left right))

        (and (or (integer? left) (float-value? left))
             (or (integer? right) (float-value? right)))
        (let [l (float-double left)
              r (float-double right)]
          (nix-double
           (case operator
             :minus (- l r)
             :multiply (* l r)
             :divide (if (zero? r)
                       (outcome/fail! :eval :division-by-zero
                                      {:operator "/"})
                       (/ l r)))))

        :else
        (outcome/fail! :eval :type-error
                       {:operator operator-name :expected "number"})))

    (= operator :update)
    ;; Shallow attrset merge; right wins. Values stay lazy (thunks preserved).
    (let [left (require-attrset
                (eval-ast* left-ast environment context) "//")
          right (require-attrset
                 (eval-ast* right-ast environment context) "//")]
      (attrset-value (merge (:entries left) (:entries right))))

    (= operator :concat)
    (let [left (force-value (eval-ast* left-ast environment context))
          right (force-value (eval-ast* right-ast environment context))]
      (cond
        (and (string? left) (string? right))
        (str left right)
        (and (vector? left) (vector? right))
        (into left right)
        :else
        (outcome/fail! :eval :type-error
                       {:operation "++" :expected "list-or-string"})))

    (contains? #{:lt :gt :le :ge} operator)
    (let [operator-name (get {:lt "<" :gt ">" :le "<=" :ge ">="} operator)
          left (force-value (eval-ast* left-ast environment context))
          right (force-value (eval-ast* right-ast environment context))]
      (letfn [(cmp-num [l r]
                ;; Nix: non-strict ≤/≥ from strict < so NaN ≤ NaN is true.
                (case operator
                  :lt (< l r)
                  :gt (< r l)
                  :le (not (< r l))
                  :ge (not (< l r))))
              (cmp-int [l r]
                (case operator
                  :lt (< l r)
                  :gt (> l r)
                  :le (<= l r)
                  :ge (>= l r)))
              (cmp-ord [c]
                ;; c: negative/zero/positive ordering of left vs right.
                (case operator
                  :lt (neg? c)
                  :gt (pos? c)
                  :le (not (pos? c))
                  :ge (not (neg? c))))]
        (cond
          ;; Exact i64 order — double would collapse values past 2^53
          ;; and break overflow guards in guest primitive-dispatch.
          (and (integer? left) (integer? right))
          (cmp-int (long left) (long right))

          (and (or (integer? left) (float-value? left))
               (or (integer? right) (float-value? right)))
          (cmp-num (float-double left) (float-double right))

          (and (string? left) (string? right))
          ;; Nix byte-order: ordinal (UTF-16 code units ≈ UTF-8 for BMP).
          (cmp-ord (System.String/Compare
                    (str left) (str right)
                    System.StringComparison/Ordinal))

          ;; List lexicographic compare (numeric-mixed / nonfinite / strings).
          (and (vector? left) (vector? right))
          (loop [xs left ys right]
            (cond
              (and (empty? xs) (empty? ys))
              (case operator :lt false :gt false :le true :ge true)
              (empty? xs)
              (case operator :lt true :le true :gt false :ge false)
              (empty? ys)
              (case operator :lt false :le false :gt true :ge true)
              :else
              (let [a (force-value (first xs))
                    b (force-value (first ys))]
                (cond
                  ;; Shared object (NaN/function identity) before scalar rules.
                  (identical? a b)
                  (recur (rest xs) (rest ys))

                  (and (integer? a) (integer? b))
                  (let [ia (long a) ib (long b)]
                    (if (= ia ib)
                      (recur (rest xs) (rest ys))
                      (cmp-int ia ib)))

                  (and (or (integer? a) (float-value? a))
                       (or (integer? b) (float-value? b)))
                  (let [da (float-double a) db (float-double b)]
                    (if (and (= da db) (not (Double/IsNaN da)))
                      (recur (rest xs) (rest ys))
                      (cmp-num da db)))

                  (and (string? a) (string? b))
                  (let [c (System.String/Compare
                           (str a) (str b)
                           System.StringComparison/Ordinal)]
                    (if (zero? c)
                      (recur (rest xs) (rest ys))
                      (cmp-ord c)))

                  :else
                  (outcome/fail! :eval :type-error
                                 {:operator operator-name
                                  :expected "comparable-list"})))))

          :else
          (outcome/fail! :eval :type-error
                         {:operator operator-name :expected "comparable"}))))

    (contains? #{:eq :neq} operator)
    (let [left (force-value (eval-ast* left-ast environment context))
          right (force-value (eval-ast* right-ast environment context))
          equal? (deep-equal? left right)]
      (if (= operator :eq) equal? (not equal?)))

    :else
    (outcome/fail! :eval :unsupported-expression
                   {:operator (name operator)})))

(defn eval-ast*
  ([ast environment]
   (eval-ast* ast environment nil))
  ([ast environment context]
   (case (:op ast)
     :bool (:value ast)
     :null nil
     :int (:value ast)
     :float (:value ast)
     :string (:value ast)
     :string-interp
     (apply str
            (map (fn [part]
                   (let [value (force-value (eval-ast* part environment context))]
                     (cond
                       (string? value) value
                       (integer? value) (str value)
                       (float-value? value) (nix-float-str value)
                       (boolean? value) (if value "1" "")
                       (nil? value) ""
                       (path-value? value) (:value value)
                       :else (str value))))
                 (:parts ast)))
     :path (path-value (:value ast))
     :var (lookup-value environment (:name ast))

     :if
     (let [condition (force-value
                      (eval-ast* (:condition ast) environment context))]
       (when-not (boolean? condition)
         (outcome/fail! :eval :non-boolean-condition {}))
       (eval-ast* (if condition (:then ast) (:else ast))
                  environment context))

     :with
     (let [attrs (force-value (eval-ast* (:attrs ast) environment context))]
       (when-not (attrset? attrs)
         (outcome/fail! :eval :type-error
                        {:operation "with" :expected "set"}))
       ;; with-scope is lowest priority after local frames; install as a frame
       ;; of the attrset entries (Nix with).
       (eval-ast* (:body ast)
                  (cons (atom (:entries attrs)) environment)
                  context))

     :assert
     (let [condition (force-value
                      (eval-ast* (:condition ast) environment context))]
       (when-not (boolean? condition)
         (outcome/fail! :eval :non-boolean-condition
                        {:construct "assert"}))
       (when-not condition
         (outcome/fail! :eval :assertion-failed {}))
       (eval-ast* (:body ast) environment context))

     :enclosing-var
     ;; Used only when an inherit binding is evaluated outside recursive-frame
     ;; (non-recursive attrset). Look up in the current environment (already
     ;; the enclosing scope for plain attrsets).
     (lookup-value environment (:name ast))

     :not
     (not (boolean-value (eval-ast* (:value ast) environment context) "!"))

     :negate
     ;; Nix unary `-x` is `0 - x` (so `-0.0` is +0.0, not IEEE sign flip).
     (let [v (force-value (eval-ast* (:value ast) environment context))]
       (cond
         (integer? v) (checked-subtract 0 v)
         (float-value? v) (nix-double (- 0.0 (float-double v)))
         :else
         (outcome/fail! :eval :type-error
                        {:operator "-" :arity 1 :expected "number"})))

     :binary
     (eval-binary (:operator ast) (:left ast) (:right ast)
                  environment context)

     :lambda
     (if-let [pattern (:param-pattern ast)]
       (closure-value nil pattern (:body ast) environment context)
       (closure-value (:parameter ast) (:body ast) environment context))

     :call
     (let [function (force-value (eval-ast* (:function ast) environment context))
           argument (thunk #(eval-ast* (:argument ast) environment context))]
       (apply-callable function argument))

     :select
     (let [attribute-form (:attribute ast)
           default (:or-default ast)
           select-miss?
           (fn [data]
             (and (::outcome/failure data)
                  (= :eval (::outcome/phase data))
                  (contains? #{:attribute-missing :type-error}
                             (::outcome/class data))))
           resolve-attr
           (fn [form]
             (if (and (map? form) (:pnix/dynamic-attr form))
               (let [v (force-value
                        (eval-ast* (:ast form) environment context))]
                 (cond
                   (string? v) v
                   (integer? v) (str v)
                   (float-value? v) (nix-float-str v)
                   (boolean? v) (if v "1" "")
                   (nil? v) ""
                   :else (nix-to-string v)))
               form))]
       (try
         (let [target (force-value
                       (eval-ast* (:target ast) environment context))
               attribute (resolve-attr attribute-form)]
           (cond
             (not (attrset? target))
             (if default
               (eval-ast* default environment context)
               (outcome/fail! :eval :type-error
                              {:operation "select" :attribute attribute}))

             (contains? (:entries target) attribute)
             (force-value (get (:entries target) attribute))

             default
             (eval-ast* default environment context)

             :else
             (outcome/fail! :eval :attribute-missing
                            {:attribute attribute})))
         (catch System.Exception error
           ;; Nested `a.b or d`: missing intermediate attrs become failures on
           ;; the target; select-or defaults catch those (Nix/clj parity).
           (let [data (ex-data error)]
             (if (and default (select-miss? data))
               (eval-ast* default environment context)
               (throw error))))))

     :has-attr
     (loop [target (force-value
                    (eval-ast* (:target ast) environment context))
            path (:path ast)]
       (if-not (attrset? target)
         false
         (let [attribute-form (first path)
               attribute
               (if (and (map? attribute-form)
                        (:pnix/dynamic-attr attribute-form))
                 (let [v (force-value
                          (eval-ast* (:ast attribute-form)
                                     environment context))]
                   (cond
                     (string? v) v
                     (integer? v) (str v)
                     (nil? v) ""
                     :else (nix-to-string v)))
                 attribute-form)]
           (if-not (contains? (:entries target) attribute)
             false
             (if (= 1 (count path))
               true
               (recur (force-value (get (:entries target) attribute))
                      (next path)))))))

     :list
     ;; Nix list elements are lazy; force only when observed.
     (mapv (fn [item]
             (thunk #(eval-ast* item environment context)))
           (:items ast))

     :attrset
     (letfn [(resolve-entry-key [name]
               (if (and (map? name) (:pnix/dynamic-attr name))
                 (let [v (force-value
                          (eval-ast* (:ast name) environment context))]
                   (cond
                     (string? v) v
                     (integer? v) (str v)
                     (float-value? v) (nix-float-str v)
                     (boolean? v) (if v "1" "")
                     (nil? v) ""
                     :else (nix-to-string v)))
                 name))
             (bind-entries [entries env]
               (into {}
                     (map (fn [[name value-ast]]
                            (let [k (resolve-entry-key name)]
                              [k
                               (if (= :enclosing-var (:op value-ast))
                                 (thunk #(lookup-value env (:name value-ast)))
                                 (thunk #(eval-ast* value-ast env
                                                    context)))])))
                     entries))]
       (if (:recursive? ast)
         ;; Recursive frame keys must be static (Nix: dynamic keys in rec
         ;; are still evaluated, but our recursive-frame is string-keyed).
         (let [static-entries
               (into {}
                     (map (fn [[name value-ast]]
                            [(resolve-entry-key name) value-ast]))
                     (:entries ast))
               [frame _] (recursive-frame static-entries environment context)]
           (attrset-value @frame))
         (attrset-value (bind-entries (:entries ast) environment))))

     :let
     (let [[_ recursive-environment]
           (recursive-frame (:bindings ast) environment context)]
       (eval-ast* (:body ast) recursive-environment context))

     :import
     (let [target (force-value (eval-ast* (:target ast) environment context))]
       (when-not (path-value? target)
         (outcome/fail! :resolution :type-error
                        {:operation "import"}))
       (when-not context
         (outcome/fail! :resolution :import-module-not-found {}))
       (let [resolved (host/resolve-import (:root context)
                                           (:file context)
                                           (:value target))]
         (eval-file* (:root context) resolved (:modules context))))

     (outcome/fail! :eval :unsupported-expression
                    {:operator (name (:op ast))}))))

(defn eval-file*
  [root path modules]
  (let [path (host/canonical-path path)]
    (if-let [module (get @modules path)]
      (force-value module)
      (let [module
            (thunk
             #(let [source (host/read-source path)
                    ast (parser/parse-source source)
                    context {:root (host/canonical-path root)
                             :file path
                             :modules modules}]
                (eval-ast* ast (root-environment) context))
             :resolution
             :import-cycle)]
        ;; Install before forcing so an actually re-entrant import observes the
        ;; thunk's :running state, while a later lazy reference reuses :done.
        (swap! modules assoc path module)
        (force-value module)))))

(defn realize-value
  [value]
  (let [value (force-value value)]
    (cond
      (or (nil? value) (boolean? value) (number? value) (string? value))
      value

      (vector? value)
      (mapv realize-value value)

      (attrset? value)
      (into {}
            (map (fn [[name item]] [name (realize-value item)]))
            (:entries value))

      (path-value? value)
      (:value value)

      (raw-bytes? value)
      ;; Observe as a latin-1 string so CLI JSON can surface the slice;
      ;; hashString already digests the underlying bytes.
      (.GetString (System.Text.Encoding/GetEncoding "ISO-8859-1")
                  (:bytes value))

      (or (closure? value) (builtin? value))
      (outcome/fail! :observation :invalid-guest-value
                     {:reason "function-not-canonical"})

      :else
      (outcome/fail! :observation :invalid-guest-value
                     {:reason "unsupported-value"}))))

(defn eval-source
  ([source]
   (eval-source source {}))
  ([source {:keys [root file]
            :or {root (host/default-root)
                 file "seed.px"}}]
   (outcome/capture
    #(let [ast (parser/parse-source source)
           context {:root (host/canonical-path root)
                    :file (host/canonical-path file)
                    :modules (atom {})}]
       (realize-value (eval-ast* ast (root-environment) context))))))

(defn eval-file
  [root path]
  (outcome/capture
   #(realize-value (eval-file* root path (atom {})))))
