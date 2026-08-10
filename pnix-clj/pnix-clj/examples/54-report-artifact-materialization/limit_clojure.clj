;;; plain Clojure의 한계 - prn/spit은 report registry artifact가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/54-report-artifact-materialization/limit_clojure.clj

(ns report-artifact-materialization-limit
  (:require [clojure.java.io :as io]))

(def file
  (java.io.File/createTempFile "plain-report-" ".edn"))

(spit file (pr-str {:status :ok}))

(def loaded
  (read-string (slurp file)))

(println "plain file:" (.getPath file))
(println "loaded:" loaded)
(println "supported kind registry?:" false)
(println "artifact hash/version?:" false)

(assert (= :ok (:status loaded)))

(.delete file)

(println)
(println "결론: plain spit은 파일을 만들지만 report kind dispatch와 artifact hash/version을 남기지 않는다.")

