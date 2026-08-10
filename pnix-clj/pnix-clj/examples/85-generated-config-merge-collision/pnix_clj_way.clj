;;; pnix-clj의 방식 - generated config의 dynamic key collision을 D20 held reason으로 잡는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/85-generated-config-merge-collision/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(def safe-source
  "let key = \"port\"; in { service = { \"${key}\" = 8080; name = \"api\"; }; }")

(def collision-source
  "{ service = { name = \"api\"; \"${\"name\"}\" = \"api-v2\"; }; }")

(let [safe (pnix/eval-source safe-source)
      collision (pnix/eval-source collision-source)]
  (println "safe generated config:" (select-keys safe [:status :value :reason]))
  (println "collision generated config:" (select-keys collision [:status :value :reason]))

  (assert (= :ok (:status safe)))
  (assert (= 8080 (get-in safe [:value "service" "port"])))
  (assert (= "api" (get-in safe [:value "service" "name"])))

  (assert (= :held (:status collision)))
  (assert (= :duplicate-attr (:reason collision))))

(println)
(println "결론: pnix-clj는 generated config merge에서 silent overwrite 대신 collision을 reviewable held verdict로 만든다.")
(shutdown-agents)
