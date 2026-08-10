(ns pnix-clj.synthesize
  "Reverse projection (roadmap M3): a whitelisted CORE of ordinary Clojure
  expression forms -> pnix AST/source. NOT the inverse of lowering's emitted
  forms (those are runtime-helper-specialized); like pnix-hy's
  synthesize_pnix_from_hy this projects the human-shaped expression subset,
  deny-by-default: any form outside the whitelist fails
  :non-projectable-form with the offending form recorded.

  Semantic trap handled explicitly: Clojure let is SEQUENTIAL, pnix let is
  RECURSIVE. A let only projects when no binding value references its own or
  a LATER binding name (then sequential == recursive); otherwise it fails
  :sequential-let-not-projectable rather than silently changing meaning.

  Verification (report): clj-meta compiles/evaluates the ORIGINAL Clojure form
  (bytecode receipt included) and the synthesized pnix must collapse through
  the whole M2 tower to the same value — reverse projection proven across
  every lane, reusing M1's unparser and M2's tower."
  (:require [clojure.set :as set]
            [clojure.string :as str]
            [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.error :as err]
            [pnix-clj.hash :as hash]
            [pnix-clj.unparse :as unparse]))

(def lane-classification
  {:lane :experimental
   :scope :bounded-candidate-synthesis
   :admission :forbidden
   :auto-promotion :forbidden
   :runtime-write :forbidden
   :mutation :forbidden
   :allowed-input :whitelisted-clojure-form
   :allowed-output :pnix-candidate-source})

(def ^:private binary-ops
  {'+ "+" '- "-" '* "*" '/ "/"
   '< "<" '> ">" '<= "<=" '>= ">="
   '= "==" 'not= "!="})

(declare form->ast)

(defn- bail
  [reason form]
  (throw (ex-info "non-projectable"
                  {::failure (err/failed :projection
                                         reason
                                         {:offending-form form})})))

(defn- free-symbols
  "Free symbols of a whitelisted form (approximate is fine: used only to
  refuse sequential-dependent lets, and over-approximation only refuses more)."
  [form bound]
  (cond
    (symbol? form) (if (contains? bound form) #{} #{form})
    (vector? form) (reduce into #{} (map #(free-symbols % bound) form))
    (map? form) (reduce into #{} (map #(free-symbols % bound) (vals form)))
    (seq? form)
    (condp = (first form)
      'let (let [bindings (partition 2 (second form))
                 [inner acc]
                 (reduce (fn [[b acc] [n v]]
                           [(conj b n) (into acc (free-symbols v b))])
                         [bound #{}]
                         bindings)]
             (into acc (free-symbols (nth form 2) inner)))
      'fn (free-symbols (nth form 2) (into bound (second form)))
      (reduce into #{} (map #(free-symbols % bound) (rest form))))
    :else #{}))

(defn- binary-ast
  [op-str l r]
  {:op :binary :operator op-str :left (form->ast l) :right (form->ast r)})

(defn- let-ast
  [form]
  (let [[_ binding-vec body & extra] form
        pairs (partition 2 binding-vec)
        names (mapv (comp str first) pairs)]
    (when (or (seq extra) (odd? (count binding-vec)))
      (bail :let-shape-not-projectable form))
    (when (not= (count (distinct names)) (count names))
      (bail :sequential-let-not-projectable form))
    ;; a binding value may not reference its own or a later binding name —
    ;; that is where Clojure's sequential let and pnix's recursive let differ
    (doseq [[i [_ v]] (map-indexed vector pairs)]
      (let [later (set (map symbol (subvec names i)))]
        (when (seq (set/intersection (free-symbols v #{}) later))
          (bail :sequential-let-not-projectable form))))
    {:op :let
     :bindings (mapv (fn [[n v]] {:name (str n) :value (form->ast v)}) pairs)
     :body (form->ast body)}))

(defn- form->ast
  [form]
  (cond
    (integer? form) {:op :int :value form}
    (float? form) {:op :float :value form}
    (boolean? form) {:op :bool :value form}
    (nil? form) {:op :null :value nil}
    (string? form) {:op :string :value form}
    (symbol? form)
    (if (or (namespace form)
            (str/starts-with? (name form) "."))
      ;; qualified (ns/sym) and Java-interop (.method) symbols are host
      ;; machinery, never pnix variables — statically denied
      (bail :non-projectable-form form)
      {:op :var :name (str form)})

    (vector? form)
    {:op :list :items (mapv form->ast form)}

    (map? form)
    (if (every? string? (keys form))
      {:op :attrset
       :recursive false
       :attrs (mapv (fn [[k v]] {:key k :value (form->ast v)})
                    (sort-by key form))}
      (bail :non-string-map-key form))

    (seq? form)
    (let [[head & args] form]
      (cond
        (and (contains? binary-ops head) (= 2 (count args)))
        (binary-ast (binary-ops head) (first args) (second args))

        (and (= 'str head) (<= 2 (count args)))
        (reduce (fn [acc a]
                  {:op :binary :operator "+" :left acc :right (form->ast a)})
                (form->ast (first args))
                (rest args))

        (and (= 'not head) (= 1 (count args)))
        {:op :not :expr (form->ast (first args))}

        (and (= 'if head) (= 3 (count args)))
        {:op :if
         :condition (form->ast (first args))
         :then (form->ast (second args))
         :else (form->ast (nth args 2))}

        (= 'let head)
        (let-ast form)

        (and (= 'fn head) (vector? (first args)) (= 2 (count args)))
        (if (= 1 (count (first args)))
          {:op :lambda
           :param (str (ffirst args))
           :param-pattern nil
           :body (form->ast (second args))}
          (bail :multi-arg-fn-not-projectable form))

        (and (= 'get head) (= 2 (count args)) (string? (second args)))
        {:op :select :target (form->ast (first args)) :attr (second args)}

        (and (= 'contains? head) (= 2 (count args)) (string? (second args)))
        {:op :has-attr :target (form->ast (first args)) :attr (second args)}

        (and (= 'count head) (= 1 (count args)))
        {:op :call
         :fn {:op :select :target {:op :var :name "builtins"} :attr "length"}
         :arg (form->ast (first args))}

        ;; single-argument application of a projectable callee
        (and (= 1 (count args)) (or (symbol? head) (seq? head)))
        {:op :call :fn (form->ast head) :arg (form->ast (first args))}

        :else (bail :non-projectable-form form)))

    :else (bail :non-projectable-form form)))

(defn form->pnix
  "Project a whitelisted Clojure expression form to pnix. Returns
  {:status :ok :ast .. :source ..} or a failure with the offending form."
  [form]
  (try
    (let [ast (form->ast form)]
      {:status :ok
       :ast ast
       :source (unparse/unparse ast)})
    (catch clojure.lang.ExceptionInfo e
      (or (::failure (ex-data e)) (throw e)))))

;; --- report ---------------------------------------------------------------------

(def cases
  [{:id :literal-int :form 42}
   {:id :vector-literal :form [1 2 3]}
   {:id :attrset-literal :form {"b" 2 "a" 1}}
   {:id :arithmetic :form '(+ 1 (* 2 3))}
   {:id :comparison-if :form '(if (< 1 2) "y" "n")}
   {:id :let-independent :form '(let [x 40 y 2] (+ x y))}
   {:id :fn-applied :form '((fn [x] (+ x 1)) 41)}
   {:id :select :form '(get {"a" 1} "a")}
   {:id :has-attr :form '(contains? {"a" 1} "a")}
   {:id :str-concat :form '(str "a" "b" "c")}
   {:id :count-to-length :form '(count [1 2 3])}])

(def held-cases
  [{:id :sequential-shadow-let
    :form '(let [x 1 x (+ x 1)] x)
    :reason :sequential-let-not-projectable}
   {:id :forward-dependent-let
    :form '(let [a (+ b 1) b 2] a)
    :reason :sequential-let-not-projectable}
   {:id :multi-arg-fn
    :form '(fn [a b] (+ a b))
    :reason :multi-arg-fn-not-projectable}
   {:id :host-interop
    :form '(println "x")
    ;; println projects as a 1-arg call of an unbound name, so it must FAIL at
    ;; verification (tower cannot collapse an unbound var) — see run-held-case
    :reason :verification-rejects}
   {:id :java-interop
    :form '(.length "abc")
    :reason :non-projectable-form}])

(defn- run-case
  [{:keys [id form]}]
  (let [run-tower (requiring-resolve 'pnix-clj.tower/run-tower)
        projected (form->pnix form)
        expected (clj-meta/eval-lowered form)
        tower (when (= :ok (:status projected))
                (run-tower (:source projected)))
        collapse (get-in tower [:collapse :status])
        match? (and (= :ok (:status projected))
                    (= :ok (:status expected))
                    (= :collapsed collapse)
                    (= (:value expected) (get-in tower [:collapse :value])))]
    {:id id
     :form (pr-str form)
     :status (if match? :accepted :rejected)
     :source (:source projected)
     :expected-value (:value expected)
     :tower-collapse collapse
     :collapsed-value (get-in tower [:collapse :value])
     :bytecode-determinism (get-in expected [:compile-receipt
                                             :determinism :status])}))

(defn- run-held-case
  [{:keys [id form reason]}]
  (let [projected (form->pnix form)
        honest?
        (if (= :verification-rejects reason)
          ;; projects syntactically but must NOT verify end-to-end
          (let [run-tower (requiring-resolve 'pnix-clj.tower/run-tower)]
            (and (= :ok (:status projected))
                 (not= :collapsed
                       (get-in (run-tower (:source projected))
                               [:collapse :status]))))
          (and (= :held (:status projected))
               (= reason (:reason projected))))]
    {:id id
     :form (pr-str form)
     :status (if honest? :accepted :rejected)
     :expected-reason reason
     :actual (select-keys projected [:status :reason])}))

(defn report
  []
  (let [rows (mapv run-case cases)
        hrows (mapv run-held-case held-cases)
        rejected (count (remove #(= :accepted (:status %)) (concat rows hrows)))
        body {:kind :pnix-synthesize-report
              :schema :pnix-clj.synthesize-report.v0
              :policy :whitelisted-core-reverse-projection-tower-verified
              :total (+ (count rows) (count hrows))
              :accepted (- (+ (count rows) (count hrows)) rejected)
              :rejected rejected
              :projected-total (count rows)
              :held-total (count hrows)
              :rows rows
              :held-rows hrows}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash [rows hrows]))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected rows held-rows]} (report)]
    (println (format "pnix-clj synthesize: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (doseq [{:keys [id status source tower-collapse expected-value]} rows]
      (println (format "  [%s] %s -> %s collapse=%s value=%s"
                       (if (= :accepted status) "OK" "REJECT")
                       (name id) (pr-str source)
                       (some-> tower-collapse name) (pr-str expected-value))))
    (doseq [{:keys [id status expected-reason]} held-rows]
      (println (format "  [%s] held:%s (%s)"
                       (if (= :accepted status) "OK" "REJECT")
                       (name id) (name expected-reason))))
    (shutdown-agents)))
