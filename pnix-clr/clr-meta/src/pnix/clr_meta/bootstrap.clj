(ns pnix.clr-meta.bootstrap
  "Small, pnix-agnostic ClojureCLR meta-circular bootstrap.

  The host ClojureCLR compiler is used exactly once to seed an evaluator.  Each
  later evaluator is produced by asking the previous evaluator to interpret
  this evaluator's source.  This is an evaluator self-interpretation witness;
  it is deliberately not presented as compiler or CLR IL self-reproduction.")

(defn bind-params
  "Bind fixed parameters and one optional `&` rest parameter.

  This is an explicitly injected runtime primitive.  The interpreted evaluator
  owns evaluation order and closure semantics, while collection operations stay
  on the ClojureCLR runtime boundary."
  [params args env]
  (loop [i 0
         next-env env]
    (if (< i (count params))
      (if (= (nth params i) '&)
        (assoc next-env (nth params (inc i)) (seq (drop i args)))
        (recur (inc i)
               (assoc next-env (nth params i) (nth args i))))
      next-env)))

(defn bind-locals
  "Evaluate a sequential let binding vector and extend env."
  [bindings env evaluator]
  (loop [i 0
         next-env env]
    (if (< i (count bindings))
      (recur (+ i 2)
             (assoc next-env
                    (nth bindings i)
                    (evaluator (nth bindings (inc i)) next-env)))
      next-env)))

(def base-env
  "Closed host-runtime seam used by the interpreted evaluator.

  `eval` is intentionally absent.  Target programs can only reach primitives
  explicitly present in this map or supplied by a focused conformance case."
  {'symbol? symbol?
   'vector? vector?
   'seq? seq?
   'get get
   'nth nth
   'first first
   'rest rest
   '= =
   'mapv mapv
   'assoc assoc
   'apply apply
   'bind-params bind-params
   'bind-locals bind-locals})

(def evaluator-source
  "Evaluator source kept as ordinary Clojure data.

  Its interpreted surface is intentionally small: symbols, vectors, literals,
  quote, if, sequential let, anonymous/named/variadic fn, and application."
  '(fn evaluate [form env]
     (if (symbol? form)
       (get env form)
       (if (vector? form)
         (mapv (fn [item] (evaluate item env)) form)
         (if (seq? form)
           (let [head (first form)]
             (if (= head 'quote)
               (nth form 1)
               (if (= head 'if)
                 (if (evaluate (nth form 1) env)
                   (evaluate (nth form 2) env)
                   (evaluate (nth form 3) env))
                 (if (= head 'let)
                   (let [bindings (nth form 1)]
                     (evaluate (nth form 2)
                               (bind-locals bindings env evaluate)))
                   (if (= head 'fn)
                     (let [named? (symbol? (nth form 1))
                           function-name (if named? (nth form 1) nil)
                           params (if named? (nth form 2) (nth form 1))
                           body (if named? (nth form 3) (nth form 2))]
                       (fn this-function [& args]
                         (evaluate
                          body
                          (let [next-env (bind-params params args env)]
                            (if function-name
                              (assoc next-env function-name this-function)
                              next-env)))))
                     (apply (evaluate head env)
                            (mapv (fn [arg] (evaluate arg env))
                                  (rest form))))))))
           form)))))

(def conformance-cases
  "Host-neutral programs used at every stage of the evaluator tower."
  [{:id :literal
    :program 42
    :env {}
    :expected 42}
   {:id :symbol
    :program 'answer
    :env {'answer 42}
    :expected 42}
   {:id :quote
    :program '(quote (alpha beta gamma))
    :env {}
    :expected '(alpha beta gamma)}
   {:id :if-true
    :program '(if condition yes no)
    :env {'condition true 'yes :yes 'no :no}
    :expected :yes}
   {:id :if-false
    :program '(if condition yes no)
    :env {'condition false 'yes :yes 'no :no}
    :expected :no}
   {:id :sequential-let
    :program '(let [x twenty
                    y (add x two)]
                (multiply y two))
    :env {'twenty 20 'two 2 'add + 'multiply *}
    :expected 44}
   {:id :closure
    :program '(((fn [x] (fn [y] (add x y))) twenty) twenty-two)
    :env {'add + 'twenty 20 'twenty-two 22}
    :expected 42}
   {:id :named-recursion
    :program '((fn factorial [n]
                 (if (less-than n two)
                   one
                   (multiply n (factorial (subtract n one)))))
               five)
    :env {'less-than <
          'two 2
          'one 1
          'multiply *
          'subtract -
          'five 5}
    :expected 120}
   {:id :variadic-rest
    :program '((fn [head & tail] (make-vector head tail)) one two three)
    :env {'make-vector vector 'one 1 'two 2 'three 3}
    :expected [1 '(2 3)]}])

(defn build-stage-chain
  "Return the host seed followed by `self-steps` self-interpreted evaluators."
  [self-steps]
  (loop [stages [(binding [*ns* (the-ns 'pnix.clr-meta.bootstrap)]
                   (eval evaluator-source))]
         remaining self-steps]
    (if (zero? remaining)
      stages
      (let [previous (peek stages)
            next-stage (previous evaluator-source base-env)]
        (recur (conj stages next-stage) (dec remaining))))))

(defn evaluate-case
  "Evaluate one case with every stage and return deterministic evidence only."
  [stages {:keys [id program env expected]}]
  (let [program-env (merge base-env env)
        values (mapv (fn [stage] (stage program program-env)) stages)]
    {:id id
     :program program
     :expected expected
     :stage-values values
     :ok (and (apply = values)
              (= expected (first values)))}))

(defn run-gate
  "Build three evaluator generations and return an explicit proof boundary.

  Evaluator generations are not CLR compiler Stage1..N labels."
  []
  (let [stages (build-stage-chain 2)
        rows (mapv (fn [case] (evaluate-case stages case)) conformance-cases)
        quoted-source (list 'quote evaluator-source)
        source-replay (mapv (fn [stage]
                              (= evaluator-source
                                 (stage quoted-source base-env)))
                            stages)
        callable (mapv ifn? stages)
        distinct-stages (mapv (fn [[left right]]
                                (not (identical? left right)))
                              (partition 2 1 stages))
        ready (and (every? true? callable)
                   (every? true? distinct-stages)
                   (every? true? source-replay)
                   (every? :ok rows))]
    {:schema :pnix.clr-meta.bootstrap-receipt.v1
     :lane :clr-meta
     :runtime {:implementation :clojureclr
               :clojure-version (clojure-version)
               :target-framework :net10.0}
     :claim {:kind :evaluator-self-interpretation
             :seed-eval-count 1
             :self-interpreted-stages 2
             :physical-generations 3
             :description
             "Each non-seed evaluator is produced by the previous evaluator interpreting evaluator-source."}
     :not-claimed [:clojureclr-compiler-self-reproduction
                   :clr-il-fixed-point
                   :compiler-stage15-n
                   :aot-byte-reproducibility
                   :full-clojureclr-tool-replacement
                   :full-clojure-language-surface
                   :bootstrap-without-host-clojureclr
                   :pnix-language-semantics]
     :naming {:physical-sequence :evaluator-generation
              :compiler-stage-sequence :absent
              :compiler-stage15-n false}
     :boundary {:portable-special-forms [:quote :if :let :fn :application]
                :portable-values [:nil :boolean :number :string :keyword :symbol
                                  :list :vector]
                :host-runtime-primitives (vec (sort (keys base-env)))
                :target-can-call-host-eval false}
     :stage-chain [{:generation 0 :producer :host-clojureclr-compiler}
                   {:generation 1 :producer :generation-0-evaluator}
                   {:generation 2 :producer :generation-1-evaluator}]
     :stages-callable callable
     :stages-distinct distinct-stages
     :source-replay source-replay
     :cases rows
     :ready ready}))

(defn -main
  [& _]
  (let [receipt (run-gate)]
    (prn receipt)
    (flush)
    (when-not (:ready receipt)
      (System.Environment/Exit 1))))
