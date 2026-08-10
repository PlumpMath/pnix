;;; pnix-clj의 방식 - report-artifact registry가 versioned EDN artifact를 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/54-report-artifact-materialization/pnix_clj_way.clj

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
               "/pnix-report-artifact-example-" (System/nanoTime))]
  (try
    (let [result (artifact/write-report! :form-analysis dir)
          file (io/file (:path result))]
      (println "artifact:" (select-keys result [:kind :path :hash :bytes]))
      (println "report keys:" (keys (:report result)))

      (assert (= :form-analysis (:kind result)))
      (assert (.exists file))
      (assert (pos? (:bytes result)))
      (assert (string? (:hash result)))
      (assert (= 1 (get-in result [:report :report-artifact/version])))
      (assert (= :form-analysis (get-in result [:report :report-artifact/kind]))))
    (finally
      (delete-tree! dir))))

(println)
(println "결론: pnix-clj report-artifact는 report kind를 versioned, hashed EDN 파일로 materialize한다.")
(shutdown-agents)

