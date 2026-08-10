;;; plain Clojure의 한계 - 파일 목록은 px runtime import graph/boundary plan이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/56-px-runtime-run-plan/limit_clojure.clj

(ns px-runtime-run-plan-limit
  (:require [clojure.java.io :as io]))

(def root
  (io/file "resources/pnix_clj/pnix_runtime"))

(def px-files
  (when (.exists root)
    (filter #(.endsWith (.getName %) ".px") (file-seq root))))

(println "px file count:" (count px-files))
(println "import graph checked?:" false)
(println "runtime boundary verdict?:" false)

(assert (pos? (count px-files)))

(println)
(println "결론: plain file scan은 px runtime boundary, entry parse, import edge 검증이 아니다.")

