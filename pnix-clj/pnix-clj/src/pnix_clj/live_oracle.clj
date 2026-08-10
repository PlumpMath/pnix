(ns pnix-clj.live-oracle
  "Live Nix oracle: differential-tests pnix values against nix-instantiate when it is available, grounding the runtime in real Nix semantics."
  (:require [clojure.data.json :as json]
            [clojure.java.io :as io]
            [clojure.java.shell :as sh]
            [clojure.string :as str]
            [pnix-clj.core :as pnix]
            [pnix-clj.error :as pnix-error]
            [pnix-clj.grammar-fuzzer :as grammar-fuzzer])
  (:import [java.util.concurrent TimeUnit]))

(def lane-classification
  {:lane :proof-only
   :scope :optional-external-reference-oracle
   :product-runtime :forbidden
   :external-authority :comparison-only
   :mutation :forbidden
   :admission :forbidden
   :network :forbidden
   :process-execution :bounded-reference-check-only
   :default-when-missing :skipped
   :allowed-output :oracle-comparison-report})

(def default-positive-count
  5)

(def default-timeout-ms
  10000)

(defn- executable-file
  [path]
  (let [f (io/file path)]
    (when (and (.isFile f) (.canExecute f))
      (.getPath f))))

(defn discover-command
  []
  (or (some #(executable-file (.getPath (io/file % "nix-instantiate")))
            (str/split (or (System/getenv "PATH") "") #":"))
      (some executable-file
            ["/run/current-system/sw/bin/nix-instantiate"
             "/nix/var/nix/profiles/default/bin/nix-instantiate"
             "/usr/bin/nix-instantiate"
             "/usr/local/bin/nix-instantiate"])
      (let [{:keys [exit out]} (sh/sh "sh" "-lc"
                                      "command -v nix-instantiate || true")
            candidates (remove str/blank? (str/split-lines (or out "")))]
        (when (zero? exit)
          (some executable-file candidates)))))

(defn- process-output
  [process]
  (let [out (future (slurp (.getInputStream process)))
        err (future (slurp (.getErrorStream process)))]
    [out err]))

(defn eval-with-nix-instantiate
  ([source]
   (eval-with-nix-instantiate (discover-command) source {}))
  ([command source {:keys [timeout-ms]
                    :or {timeout-ms default-timeout-ms}}]
   (if-not (seq command)
     {:status :skipped
      :reason :live-oracle-command-not-found}
     (try
       (let [timeout-ms (or timeout-ms default-timeout-ms)
             process (-> (ProcessBuilder. [command "--eval" "--strict" "--json"
                                           "-E" source])
                         (.start))
             [out err] (process-output process)
             done? (.waitFor process timeout-ms TimeUnit/MILLISECONDS)]
         (if-not done?
           (do
             (.destroyForcibly process)
             (pnix-error/failed :oracle
                                :live-oracle-timeout
                                {:timeout-ms timeout-ms}))
           (let [stdout @out
                 stderr @err
                 exit (.exitValue process)]
             (if (zero? exit)
               {:status :ok
                :value (json/read-str stdout)}
               (pnix-error/failed :oracle
                                  :live-oracle-nonzero-exit
                                  {:exit exit
                                   :stderr stderr
                                   :stdout stdout})))))
       (catch Throwable t
         (pnix-error/failed-throwable :oracle
                                      :live-oracle-threw
                                      t))))))

(defn- positive-cases
  [{:keys [seed positive-count]
    :or {seed 0
         positive-count default-positive-count}}]
  (filterv #(= :positive (:fixture-class %))
           (grammar-fuzzer/generated-cases {:seed seed
                                            :positive-count positive-count
                                            :error-count 0})))

(defn- oracle-fn
  [{:keys [oracle-fn command timeout-ms discover?]
    :or {discover? true}}]
  (cond
    oracle-fn
    oracle-fn

    command
    #(eval-with-nix-instantiate command % {:timeout-ms timeout-ms})

    discover?
    (when-let [command (discover-command)]
      #(eval-with-nix-instantiate command % {:timeout-ms timeout-ms}))

    :else
    nil))

(defn- compare-row
  [oracle case]
  (let [source (:source case)
        pnix-result (pnix/eval-source source)
        oracle-result (oracle source)
        status (cond
                 (not= :ok (:status pnix-result)) :pnix-held
                 (not= :ok (:status oracle-result)) :oracle-held
                 (= (:value pnix-result) (:value oracle-result)) :matched
                 :else :mismatched)]
    (-> case
        (dissoc :source)
        (assoc :status status
               :pnix-status (:status pnix-result)
               :pnix-reason (:reason pnix-result)
               :pnix-value (when (= :ok (:status pnix-result))
                             (:value pnix-result))
               :oracle-status (:status oracle-result)
               :oracle-reason (:reason oracle-result)
               :oracle-value (when (= :ok (:status oracle-result))
                               (:value oracle-result))))))

(defn report
  ([] (report {}))
  ([opts]
   (if-let [oracle (oracle-fn opts)]
     (let [cases (positive-cases opts)
           rows (mapv #(compare-row oracle %) cases)
           counts (frequencies (map :status rows))
           mismatched (filter #(= :mismatched (:status %)) rows)]
       {:kind :pnix-live-oracle-report
        :schema :pnix-clj.live-oracle-report.v0
        :policy :optional-reference-nix-json-oracle
        :status (if (seq mismatched) :failed :ok)
        :seed (or (:seed opts) 0)
        :source-count (count rows)
        :matched (get counts :matched 0)
        :mismatched (get counts :mismatched 0)
        :pnix-held (get counts :pnix-held 0)
        :oracle-held (get counts :oracle-held 0)
        :first-mismatched (first mismatched)
        :rows rows})
     {:kind :pnix-live-oracle-report
      :schema :pnix-clj.live-oracle-report.v0
      :policy :optional-reference-nix-json-oracle
      :status :skipped
      :reason :live-oracle-command-not-found
      :source-count 0
      :matched 0
      :mismatched 0
      :rows []})))

(defn -main
  [& [positive-count seed]]
  (let [{:keys [status source-count matched mismatched pnix-held oracle-held
                first-mismatched] :as report}
        (report {:positive-count (if positive-count
                                   (parse-long positive-count)
                                   default-positive-count)
                 :seed (if seed (parse-long seed) 0)})]
    (println (format "pnix-clj live oracle: status=%s sources=%d matched=%d mismatched=%d pnix-held=%d oracle-held=%d"
                     (name status) source-count matched mismatched pnix-held oracle-held))
    (when first-mismatched
      (println "first mismatched:" (pr-str first-mismatched)))
    (shutdown-agents)
    (when (pos? mismatched)
      (System/exit 1))))
