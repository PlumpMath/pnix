(ns pnix-cljs.core
  (:require [clojure.string :as string]
            [pnix-cljs.evaluator :as evaluator]
            [pnix-cljs.outcome :as outcome]
            [pnix-cljs.parser :as parser]))

(defn error-data [cause]
  (let [data (ex-data cause)]
    (if (= true (get data "pnix_error"))
      (dissoc data "pnix_error")
      (throw cause))))

(defn eval-source
  ([source]
   (eval-source source nil))
  ([source module-context]
   (try
     (outcome/done
      (if module-context
        (evaluator/evaluate (parser/parse source) module-context)
        (evaluator/evaluate (parser/parse source))))
     (catch :default cause
       (outcome/failed (error-data cause))))))

(defn projection
  ([source]
   (projection source nil))
  ([source module-context]
   (let [result (eval-source source module-context)]
     (if (outcome/done? result)
       (try
         (outcome/project
           (outcome/done (evaluator/materialize (:value result))))
         (catch :default cause
           (outcome/project (outcome/failed (error-data cause)))))
       (outcome/project result)))))

(declare canonical-json)

(defn canonical-map-json [value]
  (let [names (sort (keys value))]
    (str "{"
         (string/join
           ","
           (map (fn [name]
                  (str (js/JSON.stringify name)
                       ":"
                       (canonical-json (get value name))))
                names))
         "}")))

(defn canonical-number-json [value]
  (if (js/Number.isFinite value)
    (let [rendered (str value)]
      (cond
        (js/Object.is value -0) "0.0"
        (or (string/includes? rendered ".")
            (string/includes? rendered "e")
            (string/includes? rendered "E")) rendered
        :else (str rendered ".0")))
    (throw (ex-info "non-finite canonical number"
                    {"phase" "observation"
                     "class" "invalid-guest-value"}))))

(defn canonical-json [value]
  (cond
    (evaluator/integer-value? value) (.toString value)
    (nil? value) "null"
    (true? value) "true"
    (false? value) "false"
    (string? value) (js/JSON.stringify value)
    (number? value) (canonical-number-json value)
    (map? value) (canonical-map-json value)
    (sequential? value)
    (str "[" (string/join "," (map canonical-json value)) "]")
    :else
    (throw (ex-info "unsupported canonical value"
                    {"phase" "observation"
                     "class" "invalid-guest-value"}))))

(defn projection-json
  ([source]
   (canonical-json (projection source)))
  ([source module-context]
   (canonical-json (projection source module-context))))

(defn value
  ([source]
   (value source nil))
  ([source module-context]
   (let [result (eval-source source module-context)]
     (if (outcome/done? result)
       (:value result)
       (throw (ex-info "PNIX evaluation failed" (:error result)))))))

(defn value-json
  ([source]
   (canonical-json (evaluator/materialize (value source))))
  ([source module-context]
   (canonical-json (evaluator/materialize (value source module-context)))))

(defn eval-source-js [source]
  (clj->js (projection source)))

(defn eval-source-json-js [source]
  (projection-json source))

(defn eval-value-json-js [source]
  (value-json source))

(defn eval-value-js [source]
  (clj->js (value source)))
