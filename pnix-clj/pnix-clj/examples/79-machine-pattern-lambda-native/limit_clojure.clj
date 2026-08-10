;;; plain Clojure의 한계 - Clojure map destructuring은 machine-native Nix pattern semantics가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/79-machine-pattern-lambda-native/limit_clojure.clj

(ns machine-pattern-lambda-native-limit)

(defn clojure-pattern
  [{:keys [x y] :or {y 2}}]
  (+ x y))

(def extra-key-result
  (clojure-pattern {:x 3 :z 100}))

(def missing-required-result
  (try
    (clojure-pattern {:y 2})
    (catch Throwable t
      {:status :threw :class (.getName (class t))})))

(println "extra key accepted by Clojure:" extra-key-result)
(println "missing required becomes host exception:" missing-required-result)
(println "derived machine pattern-bind frame?:" false)

(assert (= 5 extra-key-result))
(assert (= :threw (:status missing-required-result)))

(println)
(println "결론: Clojure destructuring은 편하지만, M7d의 machine-native pattern-bind와 Nix held reason을 제공하지 않는다.")
