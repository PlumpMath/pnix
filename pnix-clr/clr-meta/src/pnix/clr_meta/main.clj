(ns pnix.clr-meta.main
  (:require [pnix.clr-meta.bootstrap :as bootstrap]
            [pnix.clr-meta.compiler-stage1-bundle :as compiler-stage1]
            [pnix.clr-meta.runtime-artifact :as runtime-artifact]))

(defn usage! []
  ;; Selfhost bundle builders are wrapper-dispatched by bin/clr-meta; Stage2 is
  ;; dispatched before bootstrap checks. Keep every public form visible here.
  (binding [*out* *err*]
    (println
     (str "usage: clr-meta [--gate] | -e FORM | FILE"
          " | --build-runtime PLAN OUTPUT SOURCE_ROOT"
          " | --build-compiler-stage1 PROFILE PLAN SOURCE OUTPUT"
          " | --build-compiler-selfhost-stage1 OUTPUT"
          " | --build-compiler-selfhost-stage2 STAGE1_BUNDLE OUTPUT"))))

(def ^:private tool-environment
  (merge bootstrap/base-env
         {'+ + '- - '* * '/ / '< < '<= <= '> > '>= >=
          'count count 'list list 'str str 'vector vector}))

(def ^:private tool-evaluator
  ;; The admitted tool profile is intentionally the already-proven physical
  ;; evaluator generation 2. Compiler Stage15/N remains a separate open track.
  (delay (nth (bootstrap/build-stage-chain 2) 2)))

(defn- read-failure!
  [class message data]
  (throw (ex-info message
                  (merge {:phase :tool-read
                          :class class}
                         data))))

(defn- validate-portable-form!
  [form]
  (cond
    (tagged-literal? form)
    (read-failure! :tagged-reader-disabled
                   "tagged reader values are outside the admitted tool surface"
                   {:tag (str (get form :tag))})

    (reader-conditional? form)
    (read-failure! :reader-conditional-disabled
                   "reader conditionals are outside the admitted tool surface"
                   {})

    (or (nil? form)
        (boolean? form)
        (number? form)
        (string? form)
        (keyword? form)
        (symbol? form))
    form

    (or (vector? form) (seq? form))
    (do
      (doseq [item form]
        (validate-portable-form! item))
      form)

    :else
    (read-failure! :non-portable-reader-value
                   "reader value is outside the admitted evaluator value domain"
                   {:value-type (str (type form))})))

(def ^:private disabled-data-readers
  ;; LispReader consults *data-readers* before its built-in inst/uuid table.
  ;; Shadow every built-in with inert TaggedLiteral construction so no host
  ;; reader function runs before the admitted-domain check below.
  (into {}
        (map (fn [tag]
               [tag (fn [form] (tagged-literal tag form))])
             (keys default-data-readers))))

(defn read-tool-form
  "Read exactly one non-evaluating form in the admitted evaluator domain."
  [source]
  (let [eof (Object.)]
    (try
      (with-open [reader (clojure.lang.LineNumberingTextReader.
                          (System.IO.StringReader. source))]
        (binding [*read-eval* false
                  *data-readers* disabled-data-readers
                  *default-data-reader-fn*
                  (fn [tag form] (tagged-literal tag form))]
          (let [form (read {:eof eof :read-cond :preserve} reader)]
            (when (identical? form eof)
              (read-failure! :empty-tool-source
                             "tool input must contain exactly one form"
                             {}))
            (let [trailing (read {:eof eof :read-cond :preserve} reader)]
              (when-not (identical? trailing eof)
                (read-failure! :trailing-tool-form
                               "tool input must contain exactly one form"
                               {})))
            (validate-portable-form! form))))
      (catch clojure.lang.ExceptionInfo cause
        (throw cause))
      (catch System.Exception cause
        (read-failure! :reader-form-rejected
                       "tool input was rejected by the non-evaluating reader"
                       {:cause-type (str (type cause))})))))

(defn evaluate-source [source]
  (try
    {:schema :pnix.clr-meta.tool-eval.v1
     :outcome-kind :done
     :execution :evaluator-generation-2
     :value ((force tool-evaluator) (read-tool-form source) tool-environment)}
    (catch clojure.lang.ExceptionInfo cause
      (let [data (ex-data cause)]
        {:schema :pnix.clr-meta.tool-eval.v1
         :outcome-kind :failed
         :error {:phase (or (:phase data) :tool-eval)
                 :class (or (:class data) :clr-meta-evaluation-error)
                 :message (.Message cause)}}))
    (catch System.Exception cause
      {:schema :pnix.clr-meta.tool-eval.v1
       :outcome-kind :failed
       :error {:phase :tool-eval
               :class :clr-meta-evaluation-error
               :message (.Message cause)}})))

(defn -main [& args]
  (let [args (vec args)]
    (cond
      (or (empty? args) (= ["--gate"] args))
      (bootstrap/-main)

      (and (= 4 (count args)) (= "--build-runtime" (first args)))
      (let [manifest (runtime-artifact/build! (second args)
                                              (nth args 2)
                                              (nth args 3))]
        (println (runtime-artifact/manifest-json manifest)))

      (and (= 5 (count args)) (= "--build-compiler-stage1" (first args)))
      (let [manifest (compiler-stage1/build! (second args)
                                             (nth args 2)
                                             (nth args 3)
                                             (nth args 4))]
        (println (runtime-artifact/manifest-json manifest)))

      (and (= 2 (count args)) (contains? #{"-e" "--eval"} (first args)))
      (let [result (evaluate-source (second args))]
        (prn result)
        (when (= :failed (:outcome-kind result))
          (System.Environment/Exit 1)))

      (= 1 (count args))
      (let [result (evaluate-source (slurp (first args)))]
        (prn result)
        (when (= :failed (:outcome-kind result))
          (System.Environment/Exit 1)))

      :else
      (do
        (usage!)
        (System.Environment/Exit 2)))))
