(ns pnix.clj-meta.host-reflection
  "Host snapshot helpers for Clojure/JVM values and operations.

  This namespace is intentionally small and non-opinionated: it only exposes
  deterministic, best-effort snapshots for common host-domain reflections that
  pnix-clj can consume through interop.

  Contract:
  - Every snapshot returns a map with :status.
  - :ok means :snapshot contains host-inspection data.
  - :held means :reason carries why this snapshot could not be produced."
  (:require [clojure.walk :as walk])
  (:import [java.net URLClassLoader]
           [java.security MessageDigest]
           [java.nio.charset StandardCharsets]))

(def ^:private max-macroexpand-steps
  128)

(defn- sha256
  [value]
  (let [md (doto (MessageDigest/getInstance "SHA-256")
             (.reset))]
    (.update md (.getBytes (str value) StandardCharsets/UTF_8))
    (->> (.digest md)
         (map #(format "%02x" (bit-and % 0xff)))
         (apply str))))

(defn- data-hash
  [value]
  (sha256 (pr-str value)))

(defn- resolve-var*
  [value]
  (cond
    (var? value)
    value

    (symbol? value)
    (or (resolve value)
        (ns-resolve 'clojure.core (symbol (name value)))
        (find-var value))

    (string? value)
    (resolve-var* (symbol value))

    :else
    nil))

(defn- resolve-namespace*
  [value]
  (cond
    (instance? clojure.lang.Namespace value)
    value

    (or (symbol? value) (string? value))
    (or (find-ns (symbol value))
        (find-ns (symbol (name value))))

    :else
    nil))

(defn- namespace-name
  [ns-value]
  (some-> ns-value ns-name str))

(defn snapshot-var
  [value]
  (let [v (resolve-var* value)]
    (if-not v
      {:status :failed
       :reason :var-not-found
       :value value}
      (let [m (meta v)
            snapshot (assoc {"tag" "Var"
                            "host" "clojure"
                            "ns" (or (namespace-name (:ns m))
                                     (some-> (:name m) namespace)
                                     "")
                            "name" (str (:name m))
                            "dynamic" (boolean (:dynamic m))}
                           :snapshot-hash
                           (data-hash (into (sorted-map) (select-keys m [:name :ns :dynamic :tag]))))]
        {:status :ok
         :kind :var
         :snapshot snapshot}))))

(defn snapshot-namespace
  [value]
  (let [n (resolve-namespace* value)]
    (if-not n
      {:status :failed
       :reason :namespace-not-found
       :value value}
      {:status :ok
       :kind :namespace
       :snapshot
       {"tag" "Namespace"
        "host" "clojure"
        "name" (namespace-name n)
        :snapshot-hash (data-hash (namespace-name n))}})))

(defn snapshot-throwable
  [throwable]
  (if-not (instance? Throwable throwable)
    {:status :failed
     :reason :not-a-throwable
     :value throwable}
    {:status :ok
     :kind :throwable
     :snapshot
     {"tag" "Exception"
      "host" "clojure"
      "class" (.getName (class throwable))
      "message" (.getMessage throwable)
      :data (pr-str (ex-data throwable))
      :cause-class (some-> throwable .getCause class .getName)
      :trace (try (vec (.getStackTrace throwable))
                  (catch Throwable _ []))}}))

(defn- resolve-class*
  [value]
  (cond
    (class? value)
    value

    (string? value)
    (or (try (Class/forName value) (catch Throwable _ nil))
        (when-let [prefixed (try (Class/forName (str "java.lang." value))
                                (catch Throwable _ nil))]
          prefixed))

    (symbol? value)
    (resolve-class* (str value))

    :else
    nil))

(defn snapshot-class
  [value]
  (let [class-value (resolve-class* value)]
    (if-not class-value
      {:status :failed
       :reason :class-not-found
       :value value}
      {:status :ok
       :kind :class
       :snapshot
       {"tag" "JavaClass"
        "host" "clojure"
        "name" (.getName class-value)
        "simple_name" (.getSimpleName class-value)
        "package" (some-> class-value .getPackage .getName)
        "primitive" (.isPrimitive class-value)
        "array" (.isArray class-value)
        "interface" (.isInterface class-value)
        :snapshot-hash (data-hash (.getName class-value))}})))

(defn snapshot-java-object
  [value]
  (if (nil? value)
    {:status :failed
     :reason :not-a-java-object
     :value value}
    {:status :ok
     :kind :java-object
     :snapshot
     {"tag" "JavaObject"
      "host" "clojure"
      "class" (.getName (class value))
      "string" (try (str value)
                    (catch Throwable _
                      nil))
      "projection_note" "jvm-object-envelope"
      :snapshot-hash (data-hash (str value))}}))

(defn snapshot-metadata
  [value]
  (let [m (meta value)]
    {:status :ok
     :kind :metadata
     :snapshot {:status "ok"
                :metadata (into {} (or m {}))
                :schema :pnix.clj-meta.host-reflection.metadata.v0
                :snapshot-hash (data-hash (or m {}))
                :target-kind (cond
                               (var? value) :var
                               (class? value) :class
                               (instance? clojure.lang.Namespace value) :namespace
                               :else :other)}}))

(defn- normalize-auto-gensyms
  [form]
  (walk/postwalk
   (fn [value]
     (if (symbol? value)
       (if-let [[_ prefix] (re-matches #"(.+)__\\d+__auto__$" (name value))
                ]
         (symbol (namespace value) (str prefix "__AUTO__"))
         value)
       value))
   form))

(defn- macroexpand-step
  [path index before after]
  (let [before* (normalize-auto-gensyms before)
        after* (normalize-auto-gensyms after)]
    {:path path
     :phase "macroexpand-1"
     :index index
     :before before*
     :after after*
     :before-hash (data-hash before*)
     :after-hash (data-hash after*)}))

(defn- expand-one-form
  [path form]
  (loop [current form
         index 0
         steps []]
    (when (>= index max-macroexpand-steps)
      (throw (ex-info "macroexpand trace exceeded step limit"
                      {:path path
                       :limit max-macroexpand-steps
                       :form (normalize-auto-gensyms current)})))
    (let [expanded (macroexpand-1 current)]
      (if (= expanded current)
        {:form current
         :steps steps}
        (recur expanded
               (inc index)
               (conj steps
                     (macroexpand-step path index current expanded)))))))

(declare macroexpand-all-trace*)

(defn- rebuild-seq
  [form items]
  (cond
    (list? form) (apply list items)
    (seq? form) (seq items)
    :else items))

(defn- macroexpand-all-trace*
  [path form]
  (let [{expanded :form
         root-steps :steps} (if (seq? form)
                               (expand-one-form path form)
                               {:form form
                                :steps []})]
    (cond
      (seq? expanded)
      (let [children (map-indexed
                     (fn [idx item]
                       (macroexpand-all-trace* (conj path idx) item))
                     expanded)]
        {:form (rebuild-seq expanded (map :form children))
         :steps (into root-steps (mapcat :steps children))})

      (vector? expanded)
      (let [children (map-indexed
                     (fn [idx item]
                       (macroexpand-all-trace* (conj path idx) item))
                     expanded)]
        {:form (mapv :form children)
         :steps (into root-steps (mapcat :steps children))})

      (map? expanded)
      (let [entries (map-indexed
                     (fn [idx [k v]]
                       {:key (macroexpand-all-trace* (conj path idx :key) k)
                        :value (macroexpand-all-trace* (conj path idx :value) v)})
                     expanded)]
        {:form (into (empty expanded)
                     (map (fn [{:keys [key value]}]
                            [(:form key) (:form value)]))
                     entries)
         :steps (into root-steps
                      (mapcat (fn [{:keys [key value]}]
                                (concat (:steps key) (:steps value)))
                              entries))})

      (set? expanded)
      (let [children (map-indexed
                      (fn [idx item]
                        (macroexpand-all-trace* (conj path idx) item))
                      expanded)]
        {:form (into (empty expanded) (map :form) children)
         :steps (into root-steps (mapcat :steps children))})

      :else
      {:form expanded
       :steps root-steps})))

(defn macroexpand-snapshot
  "Return a stable snapshot of macroexpand-1 traces for host-form `form`.
  Kept for pnix-clj interop so host snapshot/evidence stays in clj-meta."
  [form]
  (try
    (let [{:keys [form steps]} (if (seq? form)
                                 (macroexpand-all-trace* [] form)
                                 {:form form
                                  :steps []})]
      {:status :ok
       :kind :macroexpand
       :snapshot {:final-form (normalize-auto-gensyms form)
                  :expanded-form (normalize-auto-gensyms form)
                  :steps steps
                  :step-count (count steps)
                  :snapshot-hash (data-hash {:final-form form :steps steps})}})
    (catch Throwable t
      {:status :failed
       :reason :macroexpand-failed
       :error {:class (.getName (class t))
               :message (.getMessage t)
               :data (ex-data t)}})))

(defn- classloader-summary
  [^ClassLoader loader]
  (let [parent (.getParent loader)]
    {"class" (.getName (class loader))
     "toString" (str loader)
     :parent (when parent (.getName (class parent)))
     :parent-name (when parent (.toString parent))
     :hash (data-hash (str loader))}))

(defn snapshot-classloader
  ([] (snapshot-classloader (.getContextClassLoader (Thread/currentThread))))
  ([value]
   (let [loader (if (nil? value)
                  (.getContextClassLoader (Thread/currentThread))
                  (if (instance? ClassLoader value)
                    value
                    nil))]
     (if-not loader
       {:status :failed
        :reason :classloader-not-found
        :value value}
       (let [url-loader? (instance? URLClassLoader loader)]
         {:status :ok
          :kind :classloader
          :snapshot {"tag" "Classloader"
                     "host" "clojure"
                     :summary (classloader-summary loader)
                     :urls (if url-loader?
                             (mapv str (.getURLs ^URLClassLoader loader))
                             [])
                     :snapshot-hash (data-hash (str loader))}})))))

(defn snapshot
  "Dispatch a host snapshot by kind.

  Supported kinds:
  :var, :namespace, :metadata, :class, :classloader, :java-object,
  :throwable, :macroexpand.
  String kinds are accepted and normalized to keywords."
  [kind value]
  (let [k (if (string? kind) (keyword kind) kind)]
    (case k
      :var (snapshot-var value)
      :namespace (snapshot-namespace value)
      :metadata (snapshot-metadata value)
      :class (snapshot-class value)
      :classloader (snapshot-classloader value)
      :java-object (snapshot-java-object value)
      :throwable (snapshot-throwable value)
      :macroexpand (macroexpand-snapshot value)
      {:status :failed
       :reason :unknown-snapshot-kind
       :kind kind})))
