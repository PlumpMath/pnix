(ns pnix-clj.wiki
  "Machine-generated project WIKI + gate — the single grep-before-you-build
  source of truth. Registers EVERY capability (implemented: the report-artifact
  registry + each module's docstring) AND the roadmap (landed / in-flight /
  planned: resources/pnix_clj/roadmap.edn), rendered to docs/WIKI.md.

  Gate (two checks, like pnix-hy's --capabilities drift gate but also with
  integrity):
  - DRIFT: the committed docs/WIKI.md must equal a fresh render (code is the
    source of truth; regenerate on any capability/roadmap change).
  - INTEGRITY: every roadmap item marked :landed that names a :capability must
    have that report-kind actually wired in report-artifact/supported-kinds --
    so a 'done' claim can never outrun the code.

  Purpose: nothing is missed (past/current/future all registered) and nothing
  is built twice (grep the capability registry first)."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.hash :as hash]
            [pnix-clj.report-artifact :as report-artifact]))

(def lane-classification
  {:lane :core
   :scope :self-documenting-capability-and-roadmap-substrate
   :role :generated-project-memory-and-anti-duplication-index
   :product-runtime :allowed
   :semantic-authority :documentation-index-only
   :mutation :generated-doc-write-only
   :admission :drift-and-integrity-check-gated
   :determinism :required
   :allowed-output :wiki-document-or-drift-verdict})

(def ^:private doc-path "docs/WIKI.md")
(def ^:private roadmap-resource "pnix_clj/roadmap.edn")

(def ^:private kind->ns-overrides
  {:stage15-exec 'pnix-clj.stage15-execute})

(defn- kind->ns
  [kind]
  (or (kind->ns-overrides kind)
      (symbol (str "pnix-clj." (name kind)))))

(defn- capability-summary
  "First non-blank line of the capability module's docstring, or a marker."
  [kind]
  (let [nsname (kind->ns kind)
        found (try (require nsname) (find-ns nsname) (catch Throwable _ nil))
        doc (some-> found meta :doc)]
    (if doc
      (-> doc str/split-lines first str/trim)
      "(no docstring)")))

(defn capability-registry
  "Every report-artifact capability as data: kind, module, one-line summary,
  and its run alias. Sorted, deterministic."
  []
  (let [aliases (set (map name (keys (:aliases (read-string (slurp "deps.edn"))))))]
    (mapv (fn [kind]
            (let [nm (name kind)]
              {:kind nm
               :module (str (kind->ns kind))
               :summary (capability-summary kind)
               :run (when (contains? aliases nm) (str "-M:" nm))}))
          (sort report-artifact/supported-kinds))))

(defn roadmap
  []
  (-> (io/resource roadmap-resource) slurp read-string))

(defn integrity
  "Every :landed roadmap item that names a :capability must have that
  report-kind wired. Returns {:status :ok} or {:status :failed :violations [..]}."
  []
  (let [kinds (set report-artifact/supported-kinds)
        violations (->> (:items (roadmap))
                        (filter #(= :landed (:status %)))
                        (keep (fn [{:keys [id capability] :as item}]
                                (when (and capability (not (contains? kinds capability)))
                                  {:id id :capability capability
                                   :reason :landed-capability-not-wired})))
                        vec)]
    (if (seq violations)
      {:status :failed :reason :roadmap-integrity :violations violations}
      {:status :ok})))

(defn- registry-table
  [rows]
  (str "| capability | run | summary |\n|---|---|---|\n"
       (str/join "\n"
                 (map (fn [{:keys [kind run summary]}]
                        (format "| `%s` | %s | %s |"
                                kind
                                (if run (str "`" run "`") "—")
                                (str/replace summary "|" "\\|")))
                      rows))))

(defn- roadmap-table
  [items status]
  (let [rows (filter #(= status (:status %)) items)]
    (if (empty? rows)
      "_(none)_"
      (str "| id | capability | title |\n|---|---|---|\n"
           (str/join "\n"
                     (map (fn [{:keys [id capability title]}]
                            (format "| `%s` | %s | %s |"
                                    (name id)
                                    (if capability (str "`" (name capability) "`") "—")
                                    (str/replace (str title) "|" "\\|")))
                           rows))))))

(defn render
  "Deterministic markdown: same code + roadmap → same bytes."
  []
  (let [reg (capability-registry)
        rm (roadmap)
        items (:items rm)
        by (fn [s] (count (filter #(= s (:status %)) items)))]
    (str "# pnix-clj WIKI (generated — do not edit)\n\n"
         "Regenerate with `clojure -M:wiki`; the gate runs `clojure -M:wiki check`\n"
         "and fails on drift OR a `:landed` roadmap item whose capability is not\n"
         "actually wired. **Grep this file before implementing anything** — it is\n"
         "the anti-duplication source of truth.\n\n"
         "epic: " (:epic rm) "\n\n"
         "counts: capabilities=" (count reg)
         " · roadmap-landed=" (by :landed)
         " · in-flight=" (by :in-flight)
         " · planned=" (by :planned) "\n\n"
         "## Implemented capabilities (report-artifact registry)\n\n"
         "Each is regression-pinned by its `*/report` and runnable via its alias.\n\n"
         (registry-table reg) "\n\n"
         "## Roadmap — landed\n\n"
         (roadmap-table items :landed) "\n\n"
         "## Roadmap — in flight\n\n"
         (roadmap-table items :in-flight) "\n\n"
         "## Roadmap — planned\n\n"
         (roadmap-table items :planned) "\n")))

(defn generate!
  []
  (let [content (render)]
    (io/make-parents doc-path)
    (spit doc-path content)
    {:status :ok :path doc-path :hash (hash/sha256 content)}))

(defn check
  "Drift + integrity."
  []
  (let [expected (render)
        actual (when (.exists (io/file doc-path)) (slurp doc-path))
        integ (integrity)]
    (cond
      (not= :ok (:status integ)) integ

      (nil? actual)
      {:status :failed :reason :wiki-doc-missing :path doc-path}

      (= expected actual)
      {:status :ok :path doc-path :hash (hash/sha256 expected)}

      :else
      {:status :failed :reason :wiki-doc-drift
       :path doc-path
       :expected-hash (hash/sha256 expected)
       :actual-hash (hash/sha256 actual)})))

(defn -main
  [& args]
  (let [{:keys [status path hash reason violations] :as r}
        (if (= "check" (first args)) (check) (generate!))]
    (println (format "pnix-clj wiki: %s status=%s path=%s %s"
                     (if (= "check" (first args)) "check" "generate")
                     (name status) (or path doc-path)
                     (or hash (str "reason=" reason
                                   (when violations (str " " (pr-str violations)))))))
    (shutdown-agents)
    (when (not= :ok status) (System/exit 1))))
