;;; pnix-clj의 방식 - run-witnessed-durable이 tower collapse, mirror chain,
;;; purity rerun, witness admission, persistent evidence를 한 번에 묶는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/29-witnessed-durable-run/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [clojure.java.io :as io]
            [pnix-clj.persist :as persist]
            [pnix-clj.witnessed-run :as wr]))

(defn delete-tree! [path]
  (let [f (io/file path)]
    (when (.exists f)
      (doseq [x (reverse (file-seq f))]
        (.delete x)))))

(let [dir (str (System/getProperty "java.io.tmpdir")
               "/pnix-example-witnessed-" (System/nanoTime))
      run (wr/run-witnessed-durable "let x = 40; in x + 2" dir)
      pstore (persist/open-persistent-store dir)
      loaded (persist/load-events pstore)
      event-kinds (set (map second (:events run)))]
  (println "status:" (:status run))
  (println "collapse/chain/determinism:"
           (:collapse run) (:chain-converged? run) (:determinism run))
  (println "residual key:" (:residual-key run))
  (println "witness:" (select-keys (:witness run)
                                   [:status :witness/id :term-hash :snapshot/id]))
  (println "persisted:" (:persisted run))
  (println "loaded events verify:" (:verify loaded))

  (assert (= :admitted (:status run)))
  (assert (= :agree (:collapse run)))
  (assert (= true (:chain-converged? run)))
  (assert (= :ok (:determinism run)))
  (assert (string? (:residual-key run)))
  (assert (= #{:tower/collapse :mirror/run :purity/run} event-kinds))
  (assert (string? (get-in run [:persisted :term-hash])))
  (assert (pos? (get-in run [:persisted :events-written])))
  (assert (= :intact (get-in loaded [:verify :status])))

  (delete-tree! dir))

(println)
(println "결론: pnix-clj witnessed run은 term/snapshot/tower/mirror/purity/witness/persistence를 한 receipt surface로 묶는다.")
