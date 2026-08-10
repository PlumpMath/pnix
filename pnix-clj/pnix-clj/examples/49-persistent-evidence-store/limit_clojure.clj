;;; plain Clojure의 한계 - 파일 쓰기는 가능하지만 content-address/hash-chain store가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/49-persistent-evidence-store/limit_clojure.clj

(ns persistent-evidence-store-limit
  (:require [clojure.java.io :as io]))

(def file
  (java.io.File/createTempFile "pnix-example-plain-" ".edn"))

(spit file (pr-str {:event 1}))
(spit file (pr-str {:event 2 :overwrote true}))

(def loaded
  (read-string (slurp file)))

(println "loaded:" loaded)
(println "append-only chain verified?:" false)

(assert (= true (:overwrote loaded)))

(.delete file)

(println)
(println "결론: plain file IO는 쉽게 덮어쓰며 content-addressed term/event integrity를 보장하지 않는다.")

