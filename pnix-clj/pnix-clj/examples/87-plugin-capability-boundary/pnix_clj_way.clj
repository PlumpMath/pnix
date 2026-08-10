;;; pnix-clj의 방식 - agent/plugin tool call을 capability boundary와 witness로 감싼다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/87-plugin-capability-boundary/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.interop :as interop]
            [pnix-clj.safe-eval :as safe]))

(def tool-meta
  (interop/interop-meta {:direction :agent-tool->host
                         :effect-class :file-read
                         :loss-status :opaque}))

(let [pnix-plugin-check (safe/safe-eval "builtins.readFile \"/etc/passwd\""
                                        {:pure-only? true})
      denied (interop/run-crossing :tool-read
                                   tool-meta
                                   {:path "/tmp/demo"}
                                   #{:pure}
                                   (fn [] {:status :ok :value "secret"}))
      allowed (interop/run-crossing :tool-read
                                    tool-meta
                                    {:path "/tmp/demo"}
                                    #{:pure :file-read}
                                    (fn [] {:status :ok :value "demo"}))]
  (println "pnix plugin check:" (select-keys pnix-plugin-check
                                             [:status :reason :limit-exceeded]))
  (println "interop denied:" (select-keys denied [:status :reason :capability :witness]))
  (println "interop allowed:" (select-keys allowed [:status :value :capability :witness]))

  (assert (= :held (:status pnix-plugin-check)))
  (assert (= :static-impure-use (:reason pnix-plugin-check)))
  (assert (= :impure (:limit-exceeded pnix-plugin-check)))

  (assert (= :held (:status denied)))
  (assert (= :capability-denied (:reason denied)))
  (assert (= :file-read (get-in denied [:witness :effect-class])))

  (assert (= :ok (:status allowed)))
  (assert (= "demo" (:value allowed)))
  (assert (string? (get-in allowed [:witness :witness-hash]))))

(println)
(println "결론: pnix-clj는 plugin/tool host 권한을 deny-by-default로 막고, 허용된 crossing에도 witness를 남긴다.")
(shutdown-agents)
