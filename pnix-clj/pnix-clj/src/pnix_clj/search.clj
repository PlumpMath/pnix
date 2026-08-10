(ns pnix-clj.search
  "§17 (+ §3c open-term summary) — content-address + event + structural-
  similarity search (spine BUILD 6th; see docs/SPINE_ROADMAP.md).

  Three search modes over the evidence spine:
  - CONTENT-ADDRESS (exact): look a term up by its §3 content hash.
  - EVENT: find §5 log events by kind / field.
  - STRUCTURAL SIMILARITY (§3c): an open term's anonymous SKELETON (structure
    with all leaf values/names blanked) + its FREE-VARIABLE summary + a
    structural DISTANCE, to find near-duplicate terms.

  ★HONEST boundary: the skeleton/distance is a HEURISTIC similarity signal (an
  op-histogram / structural diff), NOT a proof of equivalence -- exactly like a
  content hash it only PROPOSES candidates, to be confirmed by cas/
  structurally-equivalent? (or the coming §3b alpha check). The unsound
  de-Bruijn-Merkle shortcut for open subterms is avoided (Blaauwbroek 2024)."
  (:require [pnix-clj.cas :as cas]
            [pnix-clj.hash :as hash]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :core
   :scope :evidence-spine-search-and-similarity
   :role :content-event-and-heuristic-structural-search
   :product-runtime :allowed
   :semantic-authority :proposal-only
   :equivalence-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :determinism :required
   :allowed-output :search-result-or-similarity-candidate})

;; ---- §3c open-term summary ----------------------------------------------

(defn skeleton
  "Anonymous structural skeleton: keep :op structure, blank every leaf value
  (:value) and variable name (:name) to :_. Two terms with the same skeleton
  share SHAPE (a similarity candidate, not an equivalence)."
  [ast]
  (cond
    (and (map? ast) (:op ast))
    (-> ast
        (cond-> (contains? ast :value) (assoc :value :_))
        (cond-> (= :var (:op ast)) (assoc :name :_))
        (dissoc :span :source-hash)
        (->> (into {} (map (fn [[k v]] [k (skeleton v)])))))
    (vector? ast) (mapv skeleton ast)
    (map? ast) (into {} (map (fn [[k v]] [k (skeleton v)])) ast)
    :else ast))

(def ^:private binder-ops #{:lambda :let})

(defn free-vars
  "The free variable names of an AST (walk, tracking binders: lambda params,
  let names)."
  ([ast] (free-vars ast #{}))
  ([ast bound]
   (cond
     (and (map? ast) (= :var (:op ast)))
     (if (contains? bound (:name ast)) #{} #{(:name ast)})

     (and (map? ast) (= :lambda (:op ast)))
     (free-vars (:body ast) (conj bound (:param ast)))

     (and (map? ast) (= :let (:op ast)))
     (let [names (set (map :name (:bindings ast)))
           bound' (into bound names)]
       (into (set (mapcat #(free-vars (:value %) bound') (:bindings ast)))
             (free-vars (:body ast) bound')))

     (map? ast) (into #{} (mapcat #(free-vars % bound) (vals ast)))
     (vector? ast) (into #{} (mapcat #(free-vars % bound) ast))
     :else #{})))

(defn- op-histogram
  [ast]
  (let [ops (atom {})]
    (letfn [(walk [x]
              (when (and (map? x) (:op x)) (swap! ops update (:op x) (fnil inc 0)))
              (cond (map? x) (run! walk (vals x))
                    (vector? x) (run! walk x)))]
      (walk ast) @ops)))

(defn structural-distance
  "A HEURISTIC structural distance in [0,1]: normalized L1 difference of the two
  terms' op-histograms (0 = same shape). NOT a proof; a similarity proposal."
  [a b]
  (let [ha (op-histogram a) hb (op-histogram b)
        ks (into (set (keys ha)) (keys hb))
        diff (reduce + (map (fn [k] (Math/abs (- (get ha k 0) (get hb k 0)))) ks))
        total (reduce + (map (fn [k] (+ (get ha k 0) (get hb k 0))) ks))]
    (if (zero? total) 0.0 (double (/ diff total)))))

(defn open-term-summary
  "The §3c summary of a term: content hash (§3), anonymous skeleton hash, and
  free-variable set."
  [ast]
  {:term-hash (cas/term-hash ast)
   :skeleton-hash (hash/data-hash (skeleton ast))
   :free-vars (vec (sort (free-vars ast)))})

;; ---- §17 search ----------------------------------------------------------

(defn similar-terms
  "Candidates from `corpus` (a seq of ASTs) ranked by structural distance to
  `query`, within `threshold`. Heuristic proposals -- confirm with cas/
  structurally-equivalent?."
  [query corpus threshold]
  (->> corpus
       (map (fn [c] {:distance (structural-distance query c)
                     :same-skeleton? (= (skeleton query) (skeleton c))
                     :confirmed-equivalent? (cas/structurally-equivalent? query c)}))
       (filter #(<= (:distance %) threshold))
       (sort-by :distance)
       vec))

(defn search-events
  "§5 event search by kind and optional field=value."
  ([log kind] (store/events-of log kind))
  ([log kind field value] (filterv #(= value (get-in % [:payload field]))
                                    (store/events-of log kind))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [parse (requiring-resolve 'pnix-clj.parser/parse-source)
        p #(:ast (parse %))
        rows [{:id :skeleton-blanks-leaves
               ;; 1+2 and 3+4 share the SAME skeleton (shape), distinct terms
               :ok? (and (= (skeleton (p "1 + 2")) (skeleton (p "3 + 4")))
                         (not (cas/structurally-equivalent? (p "1 + 2") (p "3 + 4"))))}
              {:id :free-vars-tracks-binders
               :ok? (and (= #{"y"} (free-vars (p "x: x + y")))       ; x bound, y free
                         (empty? (free-vars (p "let a = 1; in a")))
                         (= #{"z"} (free-vars (p "let a = z; in a"))))}
              {:id :structural-distance-zero-same-shape
               :ok? (zero? (structural-distance (p "1 + 2") (p "9 + 8")))}
              {:id :structural-distance-nonzero-diff-shape
               :ok? (pos? (structural-distance (p "1 + 2") (p "1 * 2 + 3")))}
              {:id :similar-proposes-confirm-separates
               :ok? (let [q (p "a + b")
                          hits (similar-terms q [(p "x + y") (p "1 * 2") (p "a + b")] 0.5)]
                      (and (= 2 (count hits))                ; two same-shape sums
                           (= 1 (count (filter :confirmed-equivalent? hits)))))}
              {:id :open-term-summary
               :ok? (let [s (open-term-summary (p "x: x + y"))]
                      (and (:term-hash s) (:skeleton-hash s)
                           (= ["y"] (:free-vars s))))}
              {:id :event-search
               :ok? (let [log (store/open-store)]
                      (store/append! log :eval/run {:source-hash "s1"})
                      (store/append! log :eval/run {:source-hash "s2"})
                      (and (= 2 (count (search-events log :eval/run)))
                           (= 1 (count (search-events log :eval/run :source-hash "s1")))))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-search-report
              :schema :pnix-clj.search-report.v0
              :policy :content-address-plus-event-plus-heuristic-structural-similarity
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
    (println (format "pnix-clj search: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
