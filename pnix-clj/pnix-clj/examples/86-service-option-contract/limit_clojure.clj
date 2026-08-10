;;; plain Clojure의 한계 - map destructuring은 옵션 typo/extra key contract를 기본으로 막지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/86-service-option-contract/limit_clojure.clj

(ns service-option-contract-limit)

(defn service-port
  [{:keys [port tls] :or {port 80 tls false}}]
  (if tls
    (+ port 443)
    port))

(def typo-config
  {:prt 8080})

(def extra-config
  {:port 8080 :debug true})

(println "typo config silently falls back:" (service-port typo-config))
(println "extra config accepted:" (service-port extra-config))
(println "strict contract receipt?:" false)

(assert (= 80 (service-port typo-config)))
(assert (= 8080 (service-port extra-config)))

(println)
(println "결론: plain Clojure destructuring은 편하지만, 서비스 옵션 contract에서 typo/extra key를 명시적 verdict로 만들지 않는다.")
