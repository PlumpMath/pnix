;;; pnix-clj의 방식 - lane-classification map을 읽어 policy registry를 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/59-lane-registry-policy-map/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.lane-registry :as registry]))

(let [rows (registry/registry-rows)
      counts (registry/lane-counts rows)
      machine-row (first (filter #(= "pnix-clj.machine" (:namespace %)) rows))]
  (println "row count:" (count rows))
  (println "lane counts:" counts)
  (println "machine row:" machine-row)

  (assert (pos? (count rows)))
  (assert (pos? (get counts :core 0)))
  (assert (pos? (get counts :proof-only 0)))
  (assert (= :proof-only (:lane machine-row)))
  (assert (= :derived-abstract-machine (:scope machine-row))))

(println)
(println "결론: pnix-clj lane registry는 source의 lane-classification을 policy map으로 전시한다.")
(shutdown-agents)
