(ns pnix-clj.stage15
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.error :as err]
            [pnix-clj.hash :as hash])
  (:import [java.util.concurrent TimeUnit]))

(def lane-classification
  {:lane :proof-only
   :scope :bounded-stage-tower-control
   :backend :clj-meta
   :product-runtime :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :default-status :held
   :allowed-output :stage-control-plan-or-execution-report})

(defn repo-root
  []
  (.getCanonicalFile (io/file (System/getProperty "user.dir") "..")))

(defn clj-meta-root
  []
  (io/file (repo-root) "clj-meta"))

(def stage-range
  {:floor 15
   :ceiling :N
   :meaning "clj-meta meta-circular compiler/evaluator stages 15 and above"})

(def mirror-spine
  [:clojure-mirror
   :pnix-runtime-px
   :pnix-mirror])

(defn- file-row
  [relative-path]
  (let [f (io/file (clj-meta-root) relative-path)]
    {:path relative-path
     :exists (.exists f)
     :hash (when (.exists f)
             (hash/sha256 (slurp f)))}))

(def stage15-commands
  [{:id :compiler-smoke
    :command ["clojure" "-M:compiler-smoke"]
    :purpose "backend compiler smoke, direct/fallback surface"}
   {:id :conformance
    :command ["clojure" "-M:conformance"]
    :purpose "host/evaluator/compiler behavioral comparison"}
   {:id :gate
    :command ["clojure" "-M:gate"]
    :purpose "integrated clj-meta READY gate"}
   {:id :audit-self-source
    :command ["clojure" "-M:audit-self-source"]
    :purpose "compiler self-source analyzer/op policy census"}
   {:id :full-source-stage1
    :command ["clojure" "-M:full-source-stage1"]
    :purpose "full-source stage1 frontier receipt"}
   {:id :determinism-policy
    :command ["clojure" "-M:determinism-policy"]
    :purpose "generated names and byte digest determinism policy"}])

(def default-execute-command-ids
  [:compiler-smoke :conformance :determinism-policy])

(def default-timeout-ms
  120000)

(defn control-plan
  "Human-trackable stage15 control surface for the clj-meta backend. This does
  not execute or mutate clj-meta; it records what must be run and which backend
  source hashes the plan controls."
  []
  (let [root (clj-meta-root)
        inputs [(file-row "deps.edn")
                (file-row "src/pnix/clj_meta/compiler.clj")
                (file-row "src/pnix/clj_meta/gate.clj")
                (file-row "src/pnix/clj_meta/full_source_stage1.clj")
                (file-row "src/pnix/clj_meta/determinism_policy.clj")]]
    {:kind :stage15-control-plan
     :status :pending
     :reason :stage15-gates-not-executed
     :backend :clj-meta
     :backend-role :meta-circular-compiler-evaluator
     :stage-range stage-range
     :backend-root (.getCanonicalPath root)
     :write-policy :read-only-backend
     :mirror-spine mirror-spine
     :truth-boundary :pnix-runtime-and-pnix-mirror-required
     :inputs inputs
     :commands stage15-commands
     :command-count (count stage15-commands)
     :missing-inputs (vec (remove :exists inputs))}))

(defn command-by-id
  []
  (into {} (map (juxt :id identity) stage15-commands)))

(defn- preview
  [s]
  (let [s (or s "")]
    (subs s 0 (min 1000 (count s)))))

(defn run-command
  [{:keys [id command purpose] :as command-row} timeout-ms]
  (let [started (System/nanoTime)
        process (-> (ProcessBuilder. ^java.util.List command)
                    (.directory (clj-meta-root))
                    (.start))
        stdout* (future (slurp (.getInputStream process)))
        stderr* (future (slurp (.getErrorStream process)))
        done? (.waitFor process (long timeout-ms) TimeUnit/MILLISECONDS)
        duration-ms (long (/ (- (System/nanoTime) started) 1000000))]
    (if-not done?
      (do
        (.destroyForcibly process)
        (merge {:id id
                :command command
                :purpose purpose
                :duration-ms duration-ms}
               (err/failed :infrastructure
                           :stage15-command-timeout
                           {:timeout-ms timeout-ms})))
      (let [stdout @stdout*
            stderr @stderr*
            exit (.exitValue process)]
        {:id id
         :command command
         :purpose purpose
         :status (if (zero? exit) :ok :failed)
         :reason (if (zero? exit)
                   :stage15-command-ok
                   :stage15-command-nonzero-exit)
         :exit exit
         :duration-ms duration-ms
         :stdout-hash (hash/sha256 stdout)
         :stderr-hash (hash/sha256 stderr)
         :stdout-preview (preview stdout)
         :stderr-preview (preview stderr)
         :error (when-not (zero? exit)
                  (err/value :infrastructure
                             :stage15-command-nonzero-exit
                             {:exit exit}))}))))

(defn execute-plan
  "Execute selected clj-meta stage15 commands from the controlled plan.

  The default command set is intentionally bounded. Heavy or frontier commands
  from `stage15-commands` remain selectable by id, but are not implicit."
  ([] (execute-plan {}))
  ([{:keys [command-ids timeout-ms runner]
     :or {command-ids default-execute-command-ids
          timeout-ms default-timeout-ms
          runner run-command}}]
   (let [plan (control-plan)
         by-id (command-by-id)
         selected (mapv (fn [id]
                          (or (get by-id id)
                              (throw (ex-info "unknown stage15 command id"
                                              {:id id
                                               :known (vec (keys by-id))}))))
                        command-ids)
         rows (mapv #(runner % timeout-ms) selected)
         held (filter #(not= :ok (:status %)) rows)
         canonical {:command-ids (vec command-ids)
                    :timeout-ms timeout-ms
                    :rows (mapv #(select-keys % [:id :status :reason :exit
                                                 :stdout-hash :stderr-hash])
                                rows)}]
     {:kind :stage15-execution-report
      :schema :pnix-clj.stage15-execution-report.v0
      :status (if (seq held) :failed :ok)
      :reason (if (seq held)
                :stage15-command-failed
                :stage15-commands-executed)
      :backend :clj-meta
      :backend-root (:backend-root plan)
      :write-policy :execute-clj-meta-commands-only
      :control-plan-hash (hash/data-hash plan)
      :selected-command-ids (vec command-ids)
      :selected-command-count (count selected)
      :timeout-ms timeout-ms
      :rows rows
      :failed-count (count held)
      :first-failed (first held)
      :receipt-hash (hash/data-hash canonical)})))

(defn parse-command-ids
  [s]
  (if (or (nil? s) (str/blank? s))
    default-execute-command-ids
    (mapv (comp keyword str/trim)
          (str/split s #","))))
