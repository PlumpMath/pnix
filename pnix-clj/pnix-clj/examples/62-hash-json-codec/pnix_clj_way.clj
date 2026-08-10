;;; pnix-clj의 방식 - JSON codec과 stable string hash를 명시적으로 사용한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/62-hash-json-codec/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.hash :as hash]
            [pnix-clj.json :as json]))

(let [value {"b" 2
             "a" [1 true nil]}
      encoded (json/write-json value)
      decoded (json/read-json encoded)
      encoded-again (json/write-json decoded)
      h1 (hash/sha256 encoded)
      h2 (hash/sha256 encoded-again)]
  (println "json:" encoded)
  (println "decoded:" decoded)
  (println "hash:" h1)

  (assert (= "{\"a\":[1,true,null],\"b\":2}" encoded))
  (assert (= value decoded))
  (assert (= encoded encoded-again))
  (assert (= h1 h2))
  (assert (re-matches #"[0-9a-f]{64}" h1)))

(println)
(println "결론: pnix-clj는 JSON rendering과 string hash를 deterministic evidence surface로 제공한다.")
(shutdown-agents)
