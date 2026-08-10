;;; plain Clojure의 한계 — host object를 그대로 넘기면
;;; object identity/mutation/access boundary가 pnix식 held verdict로 관리되지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/31-compartment-isolation/limit_clojure.clj

(ns compartment-isolation-limit)

(def host-object
  (atom {:secret 42 :public 7}))

(defn read-secret
  [obj]
  (:secret @obj))

(defn mutate-secret!
  [obj v]
  (swap! obj assoc :secret v))

(println "host object class:" (.getName (class host-object)))
(println "direct secret read:" (read-secret host-object))
(println "is opaque ref?:" nil)
(println "release verdict:" nil)
(println "deref-after-release verdict:" nil)

(mutate-secret! host-object 99)

(println "after mutation secret:" (read-secret host-object))

(assert (= 99 (read-secret host-object)))

(println)
(println "결론: plain Clojure는 host object를 직접 넘기고 직접 변경할 수 있지만 opaque-ref/release/held 경계를 기본으로 주지 않는다.")
