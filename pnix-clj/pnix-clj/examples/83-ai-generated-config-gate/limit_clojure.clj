;;; plain Clojure의 한계 - AI가 만든 config를 바로 실행/merge하면 effect와 충돌을 늦게 본다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/83-ai-generated-config-gate/limit_clojure.clj

(ns ai-generated-config-gate-limit)

(def ai-config
  ;; 예: AI가 "편의상 HOME도 넣어뒀다"고 생성한 Clojure config.
  ;; 값을 만드는 순간 host 환경을 이미 읽었다.
  {:service {:port 8080 :enabled true}
   :home (System/getenv "HOME")})

(def merged
  ;; 예: AI가 같은 key를 두 번 제안했는데 마지막 값이 조용히 이긴다.
  (merge {:name "svc"} {:name "other"}))

(println "config has home value already?:" (boolean (:home ai-config)))
(println "merge result name:" (:name merged))
(println "pre-execution capability verdict?:" false)
(println "duplicate-key receipt?:" false)

(assert (= 8080 (get-in ai-config [:service :port])))
(assert (= "other" (:name merged)))

(println)
(println "결론: plain Clojure config는 host effect와 key 충돌을 실행/merge 뒤에야 알며, AI 출력 검토용 gate receipt가 없다.")
