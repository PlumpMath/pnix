;;; plain Clojure의 한계 - generated config merge에서 같은 key가 조용히 덮인다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/85-generated-config-merge-collision/limit_clojure.clj

(ns generated-config-merge-collision-limit)

(def base
  {"service" {"name" "api" "port" 8080}})

(def ai-overrides
  {"service" {"name" "api-v2"}})

(def merged
  (merge-with merge base ai-overrides))

(println "merged service:" (get merged "service"))
(println "collision blocked?:" false)
(println "reviewer sees overwrite only after merge:" (get-in merged ["service" "name"]))

(assert (= "api-v2" (get-in merged ["service" "name"])))
(assert (= 8080 (get-in merged ["service" "port"])))

(println)
(println "결론: plain merge는 생성된 중복 의도를 덮어쓰기 결과로 바꾸며, collision 자체를 증거로 남기지 않는다.")
