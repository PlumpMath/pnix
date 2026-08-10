;;; plain Clojure의 한계 - 플러그인/툴 호출은 host 권한을 바로 쓴다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/87-plugin-capability-boundary/limit_clojure.clj

(ns plugin-capability-boundary-limit)

(defn plugin-read-home
  []
  (System/getenv "HOME"))

(defn plugin-read-file
  [path]
  ;; 예제에서는 실제 파일을 읽지 않고, plain Clojure라면 그냥 호출된다는 점만 표시한다.
  {:would-call 'slurp :path path})

(let [home (plugin-read-home)
      file-plan (plugin-read-file "/etc/passwd")]
  (println "plugin got HOME?:" (boolean home))
  (println "plugin file plan:" file-plan)
  (println "capability denied verdict?:" false)
  (println "crossing witness hash?:" false)
  (assert (or (nil? home) (string? home)))
  (assert (= "/etc/passwd" (:path file-plan))))

(println)
(println "결론: plain plugin 호출은 host 권한 사용을 호출자 discipline에 맡기며, deny-by-default evidence가 없다.")
