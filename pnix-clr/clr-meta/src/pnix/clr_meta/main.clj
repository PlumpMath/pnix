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
          " | --multi-form FILE | --multi-form -"
          " | --multi-e FORM | --multi-eval FORM"
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

(defn- open-tool-reader
  [source]
  (clojure.lang.LineNumberingTextReader.
   (System.IO.StringReader. source)))

(defn- with-tool-reader-bindings
  [thunk]
  (binding [*read-eval* false
            *data-readers* disabled-data-readers
            *default-data-reader-fn*
            (fn [tag form] (tagged-literal tag form))]
    (thunk)))

(defn read-tool-form
  "Read exactly one non-evaluating form in the admitted evaluator domain.
  Trailing forms fail closed (default tool-eval profile)."
  [source]
  (let [eof (Object.)]
    (try
      (with-open [reader (open-tool-reader source)]
        (with-tool-reader-bindings
          (fn []
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
              (validate-portable-form! form)))))
      (catch clojure.lang.ExceptionInfo cause
        (throw cause))
      (catch System.Exception cause
        (read-failure! :reader-form-rejected
                       "tool input was rejected by the non-evaluating reader"
                       {:cause-type (str (type cause))})))))

(defn read-tool-forms
  "Read one or more non-evaluating forms (tool-eval-multi profile).
  Each form is validated with the same portable domain as single-form mode.
  Empty input fails closed."
  [source]
  (let [eof (Object.)]
    (try
      (with-open [reader (open-tool-reader source)]
        (with-tool-reader-bindings
          (fn []
            (loop [acc []]
              (let [form (read {:eof eof :read-cond :preserve} reader)]
                (if (identical? form eof)
                  (if (seq acc)
                    acc
                    (read-failure! :empty-tool-source
                                   "tool-eval-multi input must contain at least one form"
                                   {}))
                  (recur (conj acc (validate-portable-form! form)))))))))
      (catch clojure.lang.ExceptionInfo cause
        (throw cause))
      (catch System.Exception cause
        (read-failure! :reader-form-rejected
                       "tool input was rejected by the non-evaluating reader"
                       {:cause-type (str (type cause))})))))

(defn- failed-result
  [cause]
  (if (instance? clojure.lang.ExceptionInfo cause)
    (let [data (ex-data cause)]
      {:schema :pnix.clr-meta.tool-eval.v1
       :outcome-kind :failed
       :error {:phase (or (:phase data) :tool-eval)
               :class (or (:class data) :clr-meta-evaluation-error)
               :message (.Message ^clojure.lang.ExceptionInfo cause)}})
    {:schema :pnix.clr-meta.tool-eval.v1
     :outcome-kind :failed
     :error {:phase :tool-eval
             :class :clr-meta-evaluation-error
             :message (.Message ^System.Exception cause)}}))

(defn evaluate-source
  "Default tool-eval: exactly one form (trailing forms fail)."
  [source]
  (try
    {:schema :pnix.clr-meta.tool-eval.v1
     :outcome-kind :done
     :execution :evaluator-generation-2
     :profile :tool-eval
     :form-count 1
     :value ((force tool-evaluator) (read-tool-form source) tool-environment)}
    (catch clojure.lang.ExceptionInfo cause
      (failed-result cause))
    (catch System.Exception cause
      (failed-result cause))))

(defn evaluate-source-multi
  "tool-eval-multi: evaluate top-level forms left-to-right; return last value.
  Forms share the fixed tool-environment (no host def/ns). Opt-in only."
  [source]
  (try
    (let [forms (read-tool-forms source)
          eval-fn (force tool-evaluator)
          value (reduce (fn [_ form]
                          (eval-fn form tool-environment))
                        nil
                        forms)]
      {:schema :pnix.clr-meta.tool-eval.v1
       :outcome-kind :done
       :execution :evaluator-generation-2
       :profile :tool-eval-multi
       :form-count (count forms)
       :value value})
    (catch clojure.lang.ExceptionInfo cause
      (failed-result cause))
    (catch System.Exception cause
      (failed-result cause))))

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

      ;; tool-eval-multi: inline multi-form string (opt-in; -e stays single-form).
      (and (= 2 (count args))
           (contains? #{"--multi-e" "--multi-eval"} (first args)))
      (let [result (evaluate-source-multi (second args))]
        (prn result)
        (when (= :failed (:outcome-kind result))
          (System.Environment/Exit 1)))

      ;; tool-eval-multi: file, or "-" = stdin (Console.OpenStandardInput on CLR).
      (and (= 2 (count args)) (= "--multi-form" (first args)))
      (let [path (second args)
            source (cond
                     (= path "-")
                     (with-open [r (System.IO.StreamReader.
                                    (System.Console/OpenStandardInput))]
                       (.ReadToEnd r))

                     (.Exists (System.IO.FileInfo. path))
                     (slurp path)

                     :else
                     (do (binding [*out* *err*]
                           (println (str "clr-meta: --multi-form requires an existing file or '-' for stdin (got: " path ")")))
                         (System.Environment/Exit 2)
                         ""))
            result (evaluate-source-multi source)]
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