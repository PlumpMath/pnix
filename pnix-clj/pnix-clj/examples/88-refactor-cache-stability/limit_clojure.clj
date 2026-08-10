;;; plain Clojure의 한계 - source string memoization은 포맷 refactor에 약하다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/88-refactor-cache-stability/limit_clojure.clj

(ns refactor-cache-stability-limit)

(def cache (atom {}))

(defn string-keyed-eval
  [source f]
  (if-let [hit (get @cache source)]
    {:status :hit :value hit}
    (let [v (f)]
      (swap! cache assoc source v)
      {:status :miss :value v})))

(let [a (string-keyed-eval "1 + 2" (constantly 3))
      b (string-keyed-eval " 1   +   2 " (constantly 3))]
  (println "first:" a)
  (println "formatted:" b)
  (println "same semantic cache key?:" false)
  (assert (= :miss (:status a)))
  (assert (= :miss (:status b)))
  (assert (= 2 (count @cache))))

(println)
(println "결론: plain source-string cache는 의미가 같은 포맷 변경도 cache miss로 보고, content-address proof가 없다.")
