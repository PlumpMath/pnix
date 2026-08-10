(ns pnix-clj.json
  (:require [clojure.data.json :as data-json]
            [clojure.string :as str]))

(def lane-classification
  {:lane :core
   :scope :json-value-codec
   :role :parse-and-emit-pnix-json-values
   :product-runtime :allowed
   :semantic-authority :codec-only
   :mutation :forbidden
   :admission :forbidden
   :determinism :required
   :allowed-output :json-value-or-json-string})

(declare write-json)

(defn read-json
  "Parse a JSON string into pnix values: objects -> string-keyed maps, arrays ->
  vectors, numbers -> Long/Double, true/false -> Boolean, null -> nil. Unlike
  EDN reading, this treats `{\"a\":1}` as the integer 1, not the keyword :1."
  [s]
  (data-json/read-str (str s)))

(defn- escape-string
  [s]
  (str/escape (str s)
              {\" "\\\""
               \\ "\\\\"
               \newline "\\n"
               \return "\\r"
               \tab "\\t"}))

(defn- write-array
  [xs]
  (str "[" (str/join "," (map write-json xs)) "]"))

(defn- write-object
  [m]
  (let [entries (sort-by key m)]
    (str "{"
         (str/join ","
                   (map (fn [[k v]]
                          (str (write-json (name k)) ":" (write-json v)))
                        entries))
         "}")))

(defn write-json
  [value]
  (cond
    (nil? value)
    "null"

    (string? value)
    (str "\"" (escape-string value) "\"")

    (number? value)
    (str value)

    (true? value)
    "true"

    (false? value)
    "false"

    (map? value)
    (write-object value)

    (sequential? value)
    (write-array value)

    :else
    (write-json (str value))))
