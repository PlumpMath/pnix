(ns pnix-clj.persist
  "Durable backing for the evidence-store spine -- content-addressed on-disk
  persistence for §3 terms and §5 events, so the evidence trail survives across
  runs and can be replayed/audited (Unison's content-addressed SQLite codebase /
  the Nix store: artifacts keyed by content hash, never overwritten).

  Layout under a store directory:
    <dir>/terms/<term-hash>.edn   -- one canonical term per content hash
    <dir>/events.log              -- append-only, one EDN event map per line

  ★Purity boundary: persistence is a SIDE EFFECT and therefore an EXPLICIT,
  caller-invoked operation -- never part of pure `eval-source`. Payloads are
  pure EDN (the §5 hermeticity guard already forbids identity/time-bearing
  values), so a persisted store is deterministic and content-addressed. On
  reload the hash chain is re-verified (tamper-evidence), and a persisted term
  is confirmed to hash back to its filename (content-address integrity)."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.cas :as cas]
            [pnix-clj.hash :as hash]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :core
   :scope :durable-evidence-persistence-boundary
   :role :content-addressed-terms-sources-witnesses-and-event-log
   :product-runtime :allowed
   :semantic-authority :durable-evidence-only
   :side-effect :explicit-caller-invoked-only
   :mutation :content-addressed-write-or-append-only
   :admission :forbidden
   :determinism :reload-verification-required
   :allowed-output :persistent-store-handle-or-integrity-report})

(defn open-persistent-store
  "Ensure `dir` and its subdirs exist; return a persistent-store handle."
  [dir]
  (let [d (io/file dir)]
    (doseq [sub ["terms" "sources" "witnesses"]] (.mkdirs (io/file d sub)))
    {:dir (.getPath d)
     :events (.getPath (io/file d "events.log"))}))

;; ---- §3 terms ------------------------------------------------------------

(defn persist-term!
  "Write a term's canonical form to <dir>/terms/<term-hash>.edn (content-
  addressed; a re-write of the same content is idempotent). Returns the hash."
  [pstore ast]
  (when-not (cas/pure-term? ast)
    (throw (ex-info "refusing to persist an impure term" {:reason :non-edn-term})))
  (let [h (cas/term-hash ast)
        f (io/file (:dir pstore) "terms" (str h ".edn"))]
    (when-not (.exists f)
      (spit f (pr-str (cas/canonical-form ast))))
    h))

(defn load-term
  "Read a persisted term by hash, verifying it hashes back to its key (content-
  address integrity). Returns the canonical term, or nil if absent."
  [pstore h]
  (let [f (io/file (:dir pstore) "terms" (str h ".edn"))]
    (when (.exists f)
      (let [term (read-string (slurp f))]
        (when-not (= h (hash/data-hash term))
          (throw (ex-info "persisted term fails content-address check"
                          {:expected h :actual (hash/data-hash term)})))
        term))))

;; ---- sources + witnesses (for replay/audit) -----------------------------

(defn persist-source!
  "Write a source string to <dir>/sources/<input-hash>.px (content-addressed).
  Returns the input hash."
  [pstore source]
  (let [h (hash/sha256 source)
        f (io/file (:dir pstore) "sources" (str h ".px"))]
    (when-not (.exists f) (spit f source))
    h))

(defn load-source
  "Read a persisted source by its input hash, or nil."
  [pstore input-hash]
  (let [f (io/file (:dir pstore) "sources" (str input-hash ".px"))]
    (when (.exists f) (slurp f))))

(defn persist-witness!
  "Write a §15 witness to <dir>/witnesses/<witness-id>.edn. Returns the id."
  [pstore witness]
  (spit (io/file (:dir pstore) "witnesses" (str (:witness/id witness) ".edn"))
        (pr-str witness))
  (:witness/id witness))

(defn load-witness
  "Read a persisted witness by id, or nil."
  [pstore wid]
  (let [f (io/file (:dir pstore) "witnesses" (str wid ".edn"))]
    (when (.exists f) (read-string (slurp f)))))

;; ---- §5 events -----------------------------------------------------------

(defn persist-events!
  "Append every event of an in-memory §5 store to <dir>/events.log (one EDN map
  per line, append-only). Returns the count written."
  [pstore mem-store]
  (with-open [w (io/writer (:events pstore) :append true)]
    (doseq [ev (store/events mem-store)]
      (.write w (pr-str ev))
      (.write w "\n")))
  (count (store/events mem-store)))

(defn load-events
  "Read persisted events back into an in-memory §5 store (rebuilding the hash
  chain) and verify the chain is intact. Returns {:store :verify}."
  [pstore]
  (let [f (io/file (:events pstore))
        s (store/open-store)]
    (when (.exists f)
      (doseq [line (remove str/blank? (str/split-lines (slurp f)))]
        (let [{:keys [kind payload]} (read-string line)]
          (store/append! s kind payload))))
    {:store s :verify (store/verify-chain s)}))

;; ---- report --------------------------------------------------------------

(defn report
  ([] (report (str (System/getProperty "java.io.tmpdir")
                   "/pnix-persist-" (Math/abs (hash (str (into [] (repeatedly 4 #(gensym)))))))))
  ([dir]
   (let [parse (requiring-resolve 'pnix-clj.parser/parse-source)
         pstore (open-persistent-store dir)
         ast (:ast (parse "let a = 1; b = 2; in a + b"))
         h (persist-term! pstore ast)
         reloaded (load-term pstore h)
         ;; alpha-variant persists to the SAME file (content address)
         h2 (persist-term! pstore (:ast (parse "let x = 1; y = 2; in x + y")))
         ;; events
         mem (store/open-store)
         _ (store/append! mem :eval/run {:source-hash "s1" :result-hash "r1"})
         _ (store/append! mem :eval/run {:source-hash "s2" :result-hash "r2"})
         _ (persist-events! pstore mem)
         {:keys [store verify]} (load-events pstore)
         rows [{:id :term-persisted-content-addressed
                :ok? (and (string? h) (.exists (io/file dir "terms" (str h ".edn"))))}
               {:id :term-reload-integrity
                :ok? (= reloaded (cas/canonical-form ast))}
               {:id :alpha-variant-same-address
                :ok? (= h h2)}                              ; §3b: alpha-equal -> one file
               {:id :events-persisted-and-reloaded
                :ok? (= 2 (count (store/events store)))}
               {:id :reloaded-chain-intact
                :ok? (= :intact (:status verify))}
               {:id :reloaded-chain-matches-original
                :ok? (= (store/head-hash mem) (store/head-hash store))}]
         rejected (count (remove :ok? rows))
         body {:kind :pnix-persist-report
               :schema :pnix-clj.persist-report.v0
               :policy :durable-content-addressed-terms-plus-append-only-events-reverified-on-load
               :total (count rows)
               :accepted (- (count rows) rejected)
               :rejected rejected
               :rows (mapv (fn [r] (assoc r :status (if (:ok? r) :accepted :rejected))) rows)}]
     ;; clean up the temp store
     (doseq [f (reverse (file-seq (io/file dir)))] (.delete f))
     (assoc body
            :status (if (zero? rejected) :ok :failed)
            :report-hash (hash/data-hash (:rows body))))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected]} (report)]
    (println (format "pnix-clj persist: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
