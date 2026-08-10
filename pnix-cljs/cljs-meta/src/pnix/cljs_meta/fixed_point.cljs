(ns pnix.cljs-meta.fixed-point
  (:require [cljs.nodejs :as nodejs]
            [cljs.reader :as reader]
            [clojure.string :as string]))

(def fs (nodejs/require "fs"))
(def node-path (nodejs/require "path"))
(def crypto (nodejs/require "crypto"))

(def fixed-debug?
  (= "1" (aget js/process "env" "PNIX_CLJS_FIXED_DEBUG")))

(defn debug-load [& values]
  (when fixed-debug?
    (.error js/console (apply str values))))

(def harness-namespaces
  #{"pnix.cljs-meta.fixed-point"
    "pnix.cljs-meta.stage-runtime"})

(defn compiler-empty-state []
  (js* "cljs.js.empty_state()"))

(defn compiler-load-analysis-cache! [state namespace cache]
  (js* "cljs.js.load_analysis_cache_BANG_(~{}, ~{}, ~{})"
       state namespace cache))

(defn compiler-compile-str [state source name options callback]
  (js* "cljs.js.compile_str(~{}, ~{}, ~{}, ~{}, ~{})"
       state source name options callback))

(defn compiler-eval-str [state source name options callback]
  (js* "cljs.js.eval_str(~{}, ~{}, ~{}, ~{}, ~{})"
       state source name options callback))

(defn compiler-js-eval [request]
  (js* "cljs.js.js_eval(~{})" request))

(defonce embedded-core-analysis (atom nil))

(defn core-analysis-cache []
  (or @embedded-core-analysis
      (let [encoded (aget js/globalThis "PNIX_CLJS_CORE_ANALYSIS_CACHE_EDN")]
        (when-not encoded
          (throw (js/Error. "missing embedded cljs.core analysis cache")))
        (let [cache (reader/read-string encoded)]
          (reset! embedded-core-analysis cache)
          cache))))

(defn sha256 [value]
  (-> (.createHash crypto "sha256")
      (.update value "utf8")
      (.digest "hex")))

(defn normalize-name [value]
  (-> (str value)
      (string/replace #"\$macros$" "")))

(defn request-namespace [{:keys [name macros]}]
  (str (normalize-name name) (when macros "$macros")))

(defn skip-load? [runtime-namespaces request]
  (let [normalized (normalize-name (:name request))]
    (or (string/starts-with? normalized "goog.")
        (contains? runtime-namespaces (request-namespace request)))))

(defn source-roots [root]
  [(str root "/clojurescript-r1.12.145/src/main/cljs")
   (str root "/clojurescript-r1.12.145/src/main/clojure")
   (str root "/cljs-meta/target/cljs-meta-stage-runtime")])

(defn cache-files [directory]
  (if-not (.existsSync fs directory)
    []
    (mapcat
      (fn [entry]
        (let [filename (.join node-path directory (.-name entry))]
          (cond
            (.isDirectory entry) (cache-files filename)
            (and (.isFile entry)
                 (string/ends-with? filename ".cache.edn")) [filename]
            :else [])))
      (array-seq (.readdirSync fs directory #js {:withFileTypes true})))))

(defn preload-bootstrap-analysis! [state root]
  (let [cache-root (str root "/cljs-meta/target/cljs-meta-stage-runtime")
        core-macro-cache (str root
                              "/cljs-meta/target/cljs-meta-module/cljs/"
                              "core$macros.cljc.cache.edn")]
    (when-not (.existsSync fs cache-root)
      (throw (js/Error. (str "missing stage runtime analysis root " cache-root))))
    (when-not (.existsSync fs core-macro-cache)
      (throw (js/Error. (str "missing core macro analysis cache "
                             core-macro-cache))))
    (reduce
      (fn [names filename]
        (let [cache (reader/read-string (.readFileSync fs filename "utf8"))
              namespace (:name cache)
              namespace-name (str namespace)]
          (if (or (nil? namespace)
                  (contains? harness-namespaces namespace-name))
            names
            (do
              (when-not (= namespace-name "cljs.core")
                (compiler-load-analysis-cache! state namespace cache))
              (conj names namespace-name)))))
      #{"cljs.core"}
      (sort (conj (vec (cache-files cache-root)) core-macro-cache)))))

(defn candidate-files [root request-path macros]
  (let [logical-path (string/replace request-path #"\$macros$" "")
        suffixes (if macros [".clj" ".cljc"] [".cljs" ".cljc" ".js"])]
    (map #(str root "/" logical-path %) suffixes)))

(defn find-source [roots request-path macros]
  (first
    (filter #(.existsSync fs %)
            (mapcat #(candidate-files % request-path macros) roots))))

(defn make-load-fn [root loaded-sources source-units runtime-namespaces load-status]
  (let [roots (source-roots root)]
    (fn [{:keys [path macros] :as request} callback]
      (let [namespace (request-namespace request)
            status (get @load-status namespace)]
        (cond
          (skip-load? runtime-namespaces request)
          (callback {:lang :js :source ""})

          (= :loaded (:state status))
          (callback {:lang :js :source ""})

          (= :loading (:state status))
          (do
            (debug-load "fixed-loader duplicate " namespace "\n")
            (callback {:lang :js :source ""}))

          :else
          (if-let [filename (find-source roots path macros)]
            (let [source (.readFileSync fs filename "utf8")
                  relative (.relative node-path root filename)
                  language (if (string/ends-with? filename ".js") :js :clj)]
              (swap! load-status assoc namespace {:state :loading :waiters []})
              (debug-load "fixed-loader source " namespace " <- " relative "\n")
              (swap! loaded-sources assoc relative source)
              (swap! source-units assoc namespace
                     {:namespace namespace
                      :source source
                      :file relative
                      :macros macros})
              (callback {:lang language
                         :source source
                         :file relative}))
            (callback nil)))))))

(defn provided-namespace [source]
  (second (re-find #"goog\.provide\([\"']([^\"']+)[\"']\)" source)))

(defn namespace-init-source [qualified-name]
  (let [[root-name & segments] (string/split qualified-name #"\.")]
    (loop [base root-name
           remaining segments
           output ""]
      (if-let [segment (first remaining)]
        (let [path (str base "." segment)]
          (recur path
                 (next remaining)
                 (str output path " = " path " || {};\n")))
        output))))

(defn ensure-namespace-object! [qualified-name]
  (let [[root-name & segments] (string/split qualified-name #"\.")
        root-object (case root-name
                      "cljs" (js* "cljs")
                      "clojure" (js* "clojure")
                      (js* "goog.global"))]
    (loop [object root-object
           remaining segments]
      (when-let [segment (first remaining)]
        (when-not (aget object segment)
          (aset object segment #js {}))
        (recur (aget object segment) (next remaining))))))

(defn make-eval-fn [emitted load-status]
  (fn [{:keys [name source]}]
    (let [name-string (str name)
          provided-namespace (provided-namespace source)]
      (debug-load "fixed-loader eval " name-string "\n")
      (swap! emitted conj [name-string source])
      (when provided-namespace
        (ensure-namespace-object! provided-namespace))
      (try
        (let [value (js/eval source)
              loaded-name (or provided-namespace name-string)
              waiters (:waiters (get @load-status loaded-name))]
          (swap! load-status assoc loaded-name {:state :loaded :waiters []})
          (doseq [callback waiters]
            (callback {:lang :js :source ""}))
          value)
        (catch :default cause
          (.error js/console
                  (str "fixed-point dependency eval failed: " name-string
                       "\n" (subs source 0 (min 500 (count source)))))
          (throw cause))))))

(defn compiler-source [root]
  (-> (.readFileSync fs
                     (str root
                          "/clojurescript-r1.12.145/src/main/cljs/cljs/js.cljs")
                     "utf8")
      (string/replace #"\[cljs\.js :refer \[dump-core\]\]\s*" "")
      (string/replace "(dump-core)"
                      "(pnix.cljs-meta.fixed-point/core-analysis-cache)")))

(defn macro-namespace-name [namespace]
  (if (string/ends-with? namespace "$macros")
    namespace
    (str namespace "$macros")))

(defn unit-dependencies [analysis namespace]
  (let [info (get analysis (symbol namespace))
        runtime-dependencies (map str (vals (:requires info)))
        macro-dependencies (map #(macro-namespace-name (str %))
                                (vals (:require-macros info)))]
    (distinct (concat runtime-dependencies macro-dependencies))))

(defn topological-unit-names [source-units analysis]
  (let [temporary (atom #{})
        permanent (atom #{})
        ordered (atom [])]
    (letfn [(visit [namespace]
              (when (and (contains? source-units namespace)
                         (not (contains? @permanent namespace)))
                (when-not (contains? @temporary namespace)
                  (swap! temporary conj namespace)
                  (doseq [dependency (sort (unit-dependencies analysis namespace))]
                    (visit dependency))
                  (swap! temporary disj namespace)
                  (swap! permanent conj namespace)
                  (swap! ordered conj namespace))))]
      (doseq [namespace (sort (keys source-units))]
        (visit namespace))
      @ordered)))

(defn make-closure-load-fn [runtime-namespaces source-units compiled]
  (fn [request callback]
    (let [namespace (request-namespace request)]
      (cond
        (or (skip-load? runtime-namespaces request)
            (contains? @compiled namespace))
        (callback {:lang :js :source ""})

        (contains? source-units namespace)
        (callback {:error (str "compiler unit requested before dependency: "
                               namespace)})

        :else
        (callback nil)))))

(defn compile-unit-promise
  [state runtime-namespaces source-units compiled namespace]
  (js/Promise.
    (fn [resolve reject]
      (let [{:keys [source file macros]} (get source-units namespace)
            options (cond->
                      {:context :statement
                       :eval (fn [{:keys [source]}] (js/eval source))
                       :load (make-closure-load-fn runtime-namespaces
                                                   source-units
                                                   compiled)
                       :source-map false
                       :static-fns true
                       :target :nodejs
                       :verbose false}
                      macros (assoc :macros-ns true))]
        (swap! compiled conj namespace)
        (compiler-compile-str
          state source file options
          (fn [result]
            (if-let [error (:error result)]
              (reject error)
              (try
                (let [output (:value result)]
                  (when-let [provided (provided-namespace output)]
                    (ensure-namespace-object! provided))
                  (js/eval output)
                  (resolve [namespace output]))
                (catch :default cause
                  (reject cause))))))))))

(defn compile-units-promise
  [state runtime-namespaces source-units ordered-names]
  (let [compiled (atom #{})]
    (reduce
      (fn [promise namespace]
        (.then promise
               (fn [outputs]
                 (.then (compile-unit-promise state
                                              runtime-namespaces
                                              source-units
                                              compiled
                                              namespace)
                        (fn [output]
                          (conj outputs output))))))
      (js/Promise.resolve [])
      ordered-names)))

(defn payload [emitted root-source]
  (str
    (apply str
           (map-indexed
             (fn [index [name source]]
               (str "\n/* pnix-cljs compiler dependency " index " " name " */\n"
                    (when-let [provided (provided-namespace source)]
                      (namespace-init-source provided))
                    source
                    "\n"))
             emitted))
    "\n/* pnix-cljs compiler root cljs.js */\n"
    (namespace-init-source "cljs.js")
    root-source
    "\n"))

(defn source-closure [loaded-sources]
  (mapv (fn [[path source]]
          #js {:path path :sha256 (sha256 source)})
        (sort-by first loaded-sources)))

(defn result-projection [result]
  (if-let [error (:error result)]
    {"schema" "pnix.cljs-meta.result.v1"
     "outcome_kind" "failed"
     "error" {"phase" "host-eval"
              "class" "clojurescript-evaluation-error"
              "message" (or (.-message error) (str error))}}
    {"schema" "pnix.cljs-meta.result.v1"
     "outcome_kind" "done"
     "value" (:value result)}))

(defonce product-state (atom nil))

(defn product-compiler-state []
  (or @product-state
      (let [state (compiler-empty-state)]
        (reset! product-state state)
        state)))

(defn evaluate-promise [source]
  (js/Promise.
    (fn [resolve _reject]
      (compiler-eval-str
        (product-compiler-state)
        source
        "cljs-meta-fixed-input.cljs"
        {:eval compiler-js-eval
         :context :expr
         :source-map false}
        (fn [result]
          (resolve (clj->js (result-projection result))))))))

(defn compile-promise [source]
  (js/Promise.
    (fn [resolve _reject]
      (compiler-compile-str
        (product-compiler-state)
        source
        "cljs-meta-fixed-user.cljs"
        {:context :statement
         :source-map false
         :target :nodejs}
        (fn [result]
          (resolve (clj->js (result-projection result))))))))

(defn compile-compiler-promise [root]
  (js/Promise.
    (fn [resolve reject]
      (let [state (compiler-empty-state)
            loaded-sources (atom {})
            source-units (atom {})
            load-status (atom {})
            discovery-emitted (atom [])
            source (compiler-source root)
            root-file "clojurescript-r1.12.145/src/main/cljs/cljs/js.cljs"
            runtime-namespaces (preload-bootstrap-analysis! state root)]
        (compiler-compile-str
          state
          source
          root-file
          {:context :statement
           :eval (make-eval-fn discovery-emitted load-status)
           :load (make-load-fn root
                               loaded-sources
                               source-units
                               runtime-namespaces
                               load-status)
           :source-map false
           :static-fns true
           :target :nodejs
           :verbose false}
          (fn [result]
            (if-let [error (:error result)]
              (reject error)
              (let [units (assoc @source-units
                                 "cljs.js"
                                 {:namespace "cljs.js"
                                  :source source
                                  :file root-file
                                  :macros false})
                    _ (swap! loaded-sources assoc root-file source)
                    analysis (get @state :cljs.analyzer/namespaces)
                    ordered-names (topological-unit-names units analysis)
                    closure-state (compiler-empty-state)
                    closure-runtime-namespaces
                    (preload-bootstrap-analysis! closure-state root)]
                (.then
                  (compile-units-promise closure-state
                                         closure-runtime-namespaces
                                         units
                                         ordered-names)
                  (fn [outputs]
                    (let [root-output (second (first (filter #(= "cljs.js"
                                                                 (first %))
                                                            outputs)))
                          dependencies (remove #(= "cljs.js" (first %)) outputs)
                          compiler-payload (payload dependencies root-output)]
                      (resolve
                        #js {:payload compiler-payload
                             :payload_sha256 (sha256 compiler-payload)
                             :source_closure (clj->js
                                               (source-closure
                                                 @loaded-sources))})))
                  reject)))))))))
