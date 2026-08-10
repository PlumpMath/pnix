;;; pnix-clj의 방식 - M7g :machine report artifact를 registry/gate 경로로 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/91-machine-report-artifact-gate/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [clojure.java.io :as io]
            [pnix-clj.report-artifact :as artifact]))

(defn delete-tree!
  [path]
  (let [f (io/file path)]
    (when (.exists f)
      (doseq [x (reverse (file-seq f))]
        (.delete x)))))

(let [dir (str (System/getProperty "java.io.tmpdir")
               "/pnix-machine-report-artifact-example-" (System/nanoTime))]
  (try
    (assert (some #{:machine} artifact/supported-kinds))
    (let [{:keys [kind path hash bytes report]} (artifact/write-report! :machine dir)
          file (io/file path)
          disk-report (read-string (slurp file))
          labels (set (get-in report [:derivation :honest-labels]))]
      (println "artifact:" {:kind kind
                            :path path
                            :hash hash
                            :bytes bytes})
      (println "machine report:"
               (select-keys report [:kind :schema :status :row-count]))
      (println "divergent:" (count (:divergent report)))
      (println "constant-stack witness:"
               (select-keys (:constant-stack-witness report) [:depth :ok?]))
      (println "honest labels:" (sort labels))
      (println "gate command:" "clojure -M:report-machine")

      (assert (= :machine kind))
      (assert (.exists file))
      (assert (pos? bytes))
      (assert (string? hash))
      (assert (= :machine-report (:kind report)))
      (assert (= :pnix-clj.machine-report.v0 (:schema report)))
      (assert (= :ok (:status report)))
      (assert (>= (:row-count report) 155))
      (assert (empty? (:divergent report)))
      (assert (true? (get-in report [:constant-stack-witness :ok?])))
      (assert (= 1 (:report-artifact/version report)))
      (assert (= :machine (:report-artifact/kind report)))
      (assert (= :machine-report (:kind disk-report)))
      (assert (= :machine (:report-artifact/kind disk-report)))
      (assert (contains? labels :differential-not-proof))
      (assert (contains? labels :builtin-internals-on-evaluator-recursion))
      (assert (contains? labels :fuel-ticks-approximate)))
    (finally
      (delete-tree! dir))))

(println)
(println "결론: M7g 이후 machine은 내부 실험이 아니라 report-artifact/gate에서 실행되는 1급 capability다.")
(shutdown-agents)
