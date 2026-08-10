(ns pnix.clj-meta.mirror
  "Analyzer AST 를 안정적인 mirror IR 데이터로 낮춘다.

  목표는 전체 analyzer 내부 구조를 노출하는 것이 아니라, op/form/value/children 중심의
  비교 가능한 canonical view 를 제공하는 것이다. env, atom, ns 객체처럼 실행마다
  달라지는 포인터성 값은 제외한다."
  (:require [clojure.tools.analyzer.jvm :as ana]
            [clojure.pprint :as pp]))

(defn- stable-value
  [v]
  (cond
    (class? v) {:class (.getName ^Class v)}
    (var? v)   {:var (str (:ns (meta v)) "/" (:name (meta v)))}
    (symbol? v) v
    (keyword? v) v
    (or (nil? v) (string? v) (number? v) (boolean? v)) v
    :else (pr-str v)))

(declare mirror-node)

(defn- child-value
  [v]
  (cond
    (map? v) (mirror-node v)
    (vector? v) (mapv child-value v)
    :else (stable-value v)))

(defn mirror-node
  [node]
  (let [children (:children node)]
    (cond-> (array-map :op (:op node))
      (contains? node :form) (assoc :form (pr-str (:form node)))
      (contains? node :val)  (assoc :val (stable-value (:val node)))
      (contains? node :var)  (assoc :var (stable-value (:var node)))
      (contains? node :name) (assoc :name (stable-value (:name node)))
      (seq children)         (assoc :children
                                    (into (array-map)
                                          (map (fn [k] [k (child-value (get node k))])
                                               children))))))

(defn mirror-form
  [form]
  (mirror-node (ana/analyze form)))

(defn run-smoke
  []
  (let [fn-ir   (mirror-form '(fn [n] (* n n)))
        when-ir (mirror-form '(when true (inc 41)))]
   [{:desc "fn mirror" :ok (= :fn (:op fn-ir)) :ir fn-ir}
    {:desc "macroexpanded when mirror"
      :ok (= :if (:op when-ir))
      :ir when-ir}]))

(defn -main
  [& _]
  (let [rows (run-smoke)
        ok?  (every? :ok rows)]
    (doseq [{:keys [desc ok ir]} rows]
      (println (format "  [%s] %s" (if ok "OK" "FAIL") desc))
      (pp/pprint ir))
    (println (str "mirror smoke: " (if ok? "ALL OK" "FAILED")
                  " (" (count (filter :ok rows)) "/" (count rows) ")"))
    (shutdown-agents)
    (when-not ok?
      (System/exit 1))))
