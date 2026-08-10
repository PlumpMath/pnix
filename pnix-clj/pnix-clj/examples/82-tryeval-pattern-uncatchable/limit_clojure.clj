;;; plain Clojure의 한계 - try/catch는 Nix tryEval catch taxonomy가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/82-tryeval-pattern-uncatchable/limit_clojure.clj

(ns tryeval-pattern-uncatchable-limit)

(defn try-eval
  [f]
  (try
    {:success true :value (f)}
    (catch Throwable _
      {:success false :value false})))

(def thrown
  (try-eval #(throw (ex-info "throw" {}))))

(def bad-pattern
  (try-eval #(throw (ex-info "pattern type error" {:reason :bad-pattern}))))

(println "plain thrown caught:" thrown)
(println "plain pattern error also caught:" bad-pattern)
(println "catchable-vs-uncatchable taxonomy?:" false)

(assert (= false (:success thrown)))
(assert (= false (:success bad-pattern)))

(println)
(println "결론: Clojure try/catch는 모든 Throwable을 같은 catch surface로 다뤄서, Nix tryEval의 catchable/uncatchable 구분을 주지 않는다.")
