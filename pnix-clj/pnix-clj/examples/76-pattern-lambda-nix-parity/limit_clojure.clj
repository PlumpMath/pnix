;;; plain Clojure의 한계 - map destructuring/수동 binding은 Nix pattern lambda 의미와 다르다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/76-pattern-lambda-nix-parity/limit_clojure.clj

(ns pattern-lambda-nix-parity-limit)

(defn naive-sequential-defaults
  [arg]
  ;; 잘못 만들기 쉬운 방식: default를 순서대로 즉시 계산한다.
  ;; a의 default가 뒤 formal b를 참조해야 하는 Nix knot-tied scope와 다르다.
  (let [env (atom {})]
    (doseq [[name default]
            [[:a #(get @env :b)]
             [:b (constantly 2)]]]
      (swap! env assoc name (if (contains? arg name)
                              (get arg name)
                              (default))))
    (:a @env)))

(defn clojure-map-destructure
  [{:keys [a]}]
  ;; Clojure map destructuring은 extra key를 기본적으로 허용한다.
  a)

(defn eager-default-even-when-unused
  []
  ;; 수동 evaluator를 잘못 짜면, body가 default를 쓰지 않아도 default effect가 터진다.
  (try
    (let [_a (throw (ex-info "unused default evaluated" {}))]
      1)
    (catch Throwable _
      :threw)))

(println "later-formal default via naive env:" (naive-sequential-defaults {}))
(println "extra key accepted by Clojure destructuring:"
         (clojure-map-destructure {:a 1 :b 2}))
(println "unused eager default result:" (eager-default-even-when-unused))
(println "three-lane Nix receipt?:" false)

(assert (nil? (naive-sequential-defaults {})))
(assert (= 1 (clojure-map-destructure {:a 1 :b 2})))
(assert (= :threw (eager-default-even-when-unused)))

(println)
(println "결론: plain Clojure destructuring/수동 binding은 D19의 lazy recursive defaults, extra-key error, lane receipt를 기본으로 주지 않는다.")
