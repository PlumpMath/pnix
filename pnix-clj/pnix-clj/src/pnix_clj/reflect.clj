(ns pnix-clj.reflect
  "§10 + §13.1 — Clojure namespace/Var/metadata reflection SNAPSHOTS + classpath/
  JVM version snapshot (spine BUILD 3rd; see docs/SPINE_ROADMAP.md).

  These capture the HOST-VARYING inputs the §8 snapshot must PIN (Bazel
  hermeticity: system binaries / classpath / host compilers differ across
  hosts). Every snapshot is DETERMINISTIC within a process: sorted, and reduced
  to pure EDN (a Var becomes its namespaced symbol + a stable metadata subset --
  never the identity-bearing Var object, never :line/:column/:file which vary).
  So a snapshot hashes stably and can be diffed and pinned; it is the material
  the §15 witness binds a result to."
  (:require [clojure.string :as str]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :host-runtime-reflection-identity
   :role :deterministic-host-lane-snapshot
   :product-runtime :allowed
   :semantic-authority :host-identity-evidence-only
   :mutation :forbidden
   :admission :forbidden
   :determinism :sorted-pure-edn-snapshot
   :allowed-output :reflection-snapshot-or-host-lane-id})

;; ---- §10 namespace / Var / metadata snapshots ---------------------------

(defn all-ns-snapshot
  "Sorted vector of loaded namespace names (strings). Deterministic per process."
  []
  (vec (sort (map (comp name ns-name) (all-ns)))))

(defn ns-publics-snapshot
  "{public-var-name -> stable-meta-subset} for a namespace, sorted."
  [ns-sym]
  (when-let [n (find-ns (symbol ns-sym))]
    (into (sorted-map)
          (map (fn [[sym v]]
                 [(name sym) (select-keys (meta v)
                                          [:macro :dynamic :private :arglists])]))
          (ns-publics n))))

(defn var-snapshot
  "A pure-EDN snapshot of a Var: its namespaced name + a stable metadata subset
  (no identity, no source positions)."
  [var-sym]
  (when-let [v (resolve (symbol var-sym))]
    (let [m (meta v)]
      {:ns (name (ns-name (:ns m)))
       :name (name (:name m))
       :macro (boolean (:macro m))
       :dynamic (boolean (:dynamic m))
       :arglists (mapv #(mapv str %) (:arglists m))})))

(defn namespace-diff
  "Added / removed namespaces between two all-ns snapshots."
  [before after]
  (let [b (set before) a (set after)]
    {:added (vec (sort (remove b a)))
     :removed (vec (sort (remove a b)))}))

;; ---- §13.1 classpath / JVM version snapshot -----------------------------

(defn classpath-snapshot
  "Sorted classpath entries + a content hash. Absolute paths ARE host-varying
  (that is the point -- the hash PINS this host lane so §8 can detect a
  mismatch); within a host it is stable."
  []
  (let [entries (->> (str/split (or (System/getProperty "java.class.path") "")
                                (re-pattern (java.util.regex.Pattern/quote
                                             (System/getProperty "path.separator"))))
                     (remove str/blank?)
                     sort vec)]
    {:entry-count (count entries)
     :entries entries
     :classpath-hash (hash/data-hash entries)}))

(defn jvm-version-id
  "JVM + Clojure version identity (pure EDN)."
  []
  {:java (System/getProperty "java.version")
   :java-vm (System/getProperty "java.vm.name")
   :clojure (clojure-version)})

(defn host-lane-id
  "A single content hash pinning the host lane: JVM version + Clojure version +
  classpath hash. This is what a snapshot's :symbol-version should bind."
  []
  (hash/data-hash [(jvm-version-id) (:classpath-hash (classpath-snapshot))]))

(defn reflection-snapshot
  "A full deterministic host-reflection snapshot (pure EDN, hashable)."
  []
  {:kind :pnix-reflection-snapshot
   :schema :pnix-clj.reflection-snapshot.v0
   :ns-count (count (all-ns-snapshot))
   :jvm (jvm-version-id)
   :classpath (dissoc (classpath-snapshot) :entries)
   :host-lane-id (host-lane-id)})

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [snap1 (reflection-snapshot)
        snap2 (reflection-snapshot)
        ns1 (all-ns-snapshot)
        vs (var-snapshot 'clojure.core/map)
        cp (classpath-snapshot)
        rows [{:id :snapshot-deterministic
               :ok? (= snap1 snap2)}                 ; two snapshots identical
              {:id :all-ns-sorted-strings
               :ok? (and (seq ns1) (= ns1 (vec (sort ns1)))
                         (every? string? ns1))}
              {:id :var-snapshot-pure-edn
               :ok? (and (= "clojure.core" (:ns vs)) (= "map" (:name vs))
                         (false? (:macro vs)))}
              {:id :macro-var-flagged
               :ok? (:macro (var-snapshot 'clojure.core/when))}
              {:id :ns-publics-sorted
               :ok? (let [p (ns-publics-snapshot 'pnix-clj.hash)]
                      (and p (= (keys p) (sort (keys p)))))}
              {:id :namespace-diff
               :ok? (= {:added ["z"] :removed ["a"]}
                       (namespace-diff ["a" "b"] ["b" "z"]))}
              {:id :classpath-hashed
               :ok? (and (pos? (:entry-count cp)) (string? (:classpath-hash cp)))}
              {:id :host-lane-id-stable
               :ok? (= (host-lane-id) (host-lane-id))}
              {:id :jvm-version-present
               :ok? (and (:java (jvm-version-id)) (:clojure (jvm-version-id)))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-reflect-report
              :schema :pnix-clj.reflect-report.v0
              :policy :deterministic-host-reflection-snapshots-pin-host-varying-inputs
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :rows (mapv (fn [r] (assoc r :status (if (:ok? r) :accepted :rejected))) rows)}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash (:rows body)))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected]} (report)]
    (println (format "pnix-clj reflect: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
