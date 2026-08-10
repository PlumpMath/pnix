;;; pnix-clj의 방식 - CI smoke를 값뿐 아니라 receipt/held reason matrix로 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/84-ci-receipt-matrix/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(def ci-cases
  [{:id :arith :source "1 + 2" :mode :receipt :expected 3}
   {:id :builtin-len :source "builtins.length [ 1 2 3 ]" :mode :receipt :expected 3}
   {:id :bad-div :source "1 / 0" :mode :held :reason :eval-binary-failed}])

(defn row
  [{:keys [id source mode expected reason]}]
  (case mode
    :receipt
    (let [r (pnix/run-source source)]
      {:id id
       :source source
       :status (:status r)
       :reason (:reason r)
       :value (get-in r [:eval-result :value])
       :has-lanes? (and (contains? r :eval-result)
                        (contains? r :clj-meta-result)
                        (contains? r :px-runtime))
       :ok? (and (= :accepted (:status r))
                 (= :all-lanes-agree (:reason r))
                 (= expected (get-in r [:eval-result :value])))})

    :held
    (let [r (pnix/eval-source source)]
      {:id id
       :source source
       :status (:status r)
       :reason (:reason r)
       :ok? (and (= :held (:status r))
                 (= reason (:reason r)))})))

(let [rows (mapv row ci-cases)
      report {:kind :example-ci-receipt-matrix
              :status (if (every? :ok? rows) :pass :fail)
              :rows rows}]
  (doseq [row rows]
    (println row))
  (println "report:" (select-keys report [:kind :status]))

  (assert (= :pass (:status report)))
  (assert (every? :ok? rows))
  (assert (every? :has-lanes? (filter #(= :receipt (:mode (some (fn [c] (when (= (:id c) (:id %)) c)) ci-cases))) rows))))

(println)
(println "결론: pnix-clj CI는 단순 eval 성공이 아니라 lane receipt와 held reason까지 matrix로 남겨 semantic regression을 잡는다.")
(shutdown-agents)
