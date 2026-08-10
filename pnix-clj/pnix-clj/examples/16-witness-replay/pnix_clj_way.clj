;;; pnix-clj의 방식 - durable witnessed run을 저장하고, replay-witness가
;;; persisted source를 다시 실행해 term/result/snapshot hash를 비교한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/16-witness-replay/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [clojure.java.io :as io]
            [pnix-clj.persist :as persist]
            [pnix-clj.replay :as replay]
            [pnix-clj.witnessed-run :as wr]))

(defn delete-tree! [path]
  (let [f (io/file path)]
    (when (.exists f)
      (doseq [x (reverse (file-seq f))]
        (.delete x)))))

(let [dir (str (System/getProperty "java.io.tmpdir")
               "/pnix-example-replay-" (System/nanoTime))
      durable (wr/run-witnessed-durable "let x = 40; in x + 2" dir)
      pstore (persist/open-persistent-store dir)
      wid (get-in durable [:persisted :witness-id])
      reproduced (replay/replay-witness pstore wid)
      missing (replay/replay-witness pstore "missing-witness")]
  (println "durable status:" (:status durable))
  (println "witness id:" wid)
  (println "replay verdict:" (select-keys reproduced
                                          [:verdict :diffs :runtime-matches?]))
  (println "missing verdict:" (select-keys missing [:verdict :reason]))

  (assert (= :admitted (:status durable)))
  (assert (string? wid))
  (assert (= :reproduced (:verdict reproduced)))
  (assert (empty? (:diffs reproduced)))
  (assert (= true (:runtime-matches? reproduced)))
  (assert (= :missing (:verdict missing)))

  (delete-tree! dir))

(println)
(println "결론: pnix-clj replay는 persisted witness를 fresh run으로 재검증해 reproduced/diverged/missing을 구분한다.")
