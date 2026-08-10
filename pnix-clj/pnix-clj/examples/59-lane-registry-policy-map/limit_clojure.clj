;;; plain Clojure의 한계 - 파일 목록은 lane policy map이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/59-lane-registry-policy-map/limit_clojure.clj

(ns lane-registry-policy-map-limit
  (:require [clojure.java.io :as io]))

(def files
  (->> (file-seq (io/file "src/pnix_clj"))
       (filter #(.endsWith (.getName %) ".clj"))
       (map #(.getName %))
       sort
       vec))

(println "source file count:" (count files))
(println "lane classification parsed?:" false)
(println "admission/product-runtime policy?:" false)

(assert (pos? (count files)))

(println)
(println "결론: plain file scan은 namespace별 lane 정책과 driftable boundary를 읽지 않는다.")

