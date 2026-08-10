(ns pnix-clj.benchmark
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.oracle :as oracle]
            [pnix-clj.parser :as parser]
            [pnix-clj.stage7-core :as stage7-core]))

(def lane-classification
  {:lane :experimental
   :scope :local-performance-measurement
   :role :developer-benchmark-report
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :cache-reset-only
   :admission :forbidden
   :timing-authority :measurement-only
   :allowed-output :benchmark-report})

(def default-iterations
  3)

(def default-run-iterations
  1)

(defn default-sources
  []
  (vec (concat (map :source (oracle/ground-truth-cases))
               (map :source (stage7-core/cases)))))

(defn- elapsed-nanos
  [f]
  (let [started (System/nanoTime)]
    (f)
    (- (System/nanoTime) started)))

(defn- nanos->millis
  [nanos]
  (/ (double nanos) 1000000.0))

(defn- measure
  [id iterations f]
  (let [nanos (vec (for [_ (range iterations)]
                     (elapsed-nanos f)))
        total (reduce + 0 nanos)]
    {:id id
     :iterations iterations
     :total-nanos total
     :total-ms (nanos->millis total)
     :avg-ms (if (pos? iterations)
               (nanos->millis (/ total iterations))
               0.0)
     :min-ms (if (seq nanos)
               (nanos->millis (apply min nanos))
               0.0)
     :max-ms (if (seq nanos)
               (nanos->millis (apply max nanos))
               0.0)}))

(defn- report-row
  [source]
  {:source-id (keyword "benchmark" (subs (hash/sha256 source) 0 12))
   :source source})

(defn run-benchmark
  ([] (run-benchmark {}))
  ([{:keys [sources iterations run-iterations]
     :or {iterations default-iterations
          run-iterations default-run-iterations}}]
   (let [sources (vec (or sources (default-sources)))
         rows (mapv report-row sources)
         preflight (pnix/report rows)
         semantic-ready? (and (zero? (:held preflight))
                              (zero? (:rejected preflight)))]
     (parser/clear-parse-cache!)
     (lowering/clear-lower-cache!)
     (let [parse-cold (measure :parse-source-cold
                               1
                               #(doseq [source sources]
                                  (parser/parse-source source)))
           parse-warm (measure :parse-source-warm
                               iterations
                               #(doseq [source sources]
                                  (parser/parse-source source)))
           asts (mapv (comp :ast parser/parse-source) sources)
           lower-cold (do
                        (lowering/clear-lower-cache!)
                        (measure :lower-ast-cold
                                 1
                                 #(doseq [ast asts]
                                    (lowering/lower-ast ast))))
           lower-warm (measure :lower-ast-warm
                               iterations
                               #(doseq [ast asts]
                                  (lowering/lower-ast ast)))
           full-report (measure :full-report
                                run-iterations
                                #(pnix/report rows))]
       {:kind :pnix-clj-benchmark
        :schema :pnix-clj.benchmark.v0
        :status (if semantic-ready? :ok :failed)
        :reason (if semantic-ready?
                  :semantic-receipts-stable
                  :semantic-preflight-not-clean)
        :source-count (count sources)
        :iterations iterations
        :run-iterations run-iterations
        :preflight (select-keys preflight
                                [:total :accepted :rejected :held
                                 :first-frontier :first-rejected])
        :lanes [parse-cold parse-warm lower-cold lower-warm full-report]
        :parse-cache (parser/parse-cache-stats)
        :lower-cache (lowering/lower-cache-stats)}))))

(defn -main
  [& [iterations run-iterations]]
  (let [result (run-benchmark {:iterations (if iterations
                                             (Long/parseLong iterations)
                                             default-iterations)
                               :run-iterations (if run-iterations
                                                 (Long/parseLong run-iterations)
                                                 default-run-iterations)})]
    (println (format "pnix-clj benchmark: status=%s reason=%s sources=%d"
                     (name (:status result))
                     (name (:reason result))
                     (:source-count result)))
    (println "preflight:" (pr-str (:preflight result)))
    (doseq [{:keys [id iterations avg-ms min-ms max-ms]} (:lanes result)]
      (println (format "  %s iterations=%d avg=%.3fms min=%.3fms max=%.3fms"
                       (name id)
                       iterations
                       avg-ms
                       min-ms
                       max-ms)))
    (println "parse-cache:" (pr-str (:parse-cache result)))
    (println "lower-cache:" (pr-str (:lower-cache result)))
    (shutdown-agents)
    (when (not= :ok (:status result))
      (System/exit 1))))
