;;; plain Clojure의 한계 - pr-str/hash는 JSON codec policy와 stable data hash가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/62-hash-json-codec/limit_clojure.clj

(ns hash-json-codec-limit)

(def value
  {"b" 2
   "a" [1 true nil]})

(println "pr-str:" (pr-str value))
(println "plain hash:" (hash value))
(println "JSON key-order policy?:" false)
(println "stable sha256 data hash?:" false)

(assert (= 2 (get value "b")))

(println)
(println "결론: plain pr-str/hash는 pnix report와 runtime receipt에 쓸 JSON codec/hash contract가 아니다.")

