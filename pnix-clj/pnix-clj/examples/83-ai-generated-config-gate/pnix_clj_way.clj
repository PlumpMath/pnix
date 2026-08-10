;;; pnix-clj의 방식 - AI가 만든 pnix config 후보를 purity/eval/D20 gate로 triage한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/83-ai-generated-config-gate/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.safe-eval :as safe]))

(def candidates
  [{:id :pure-service-config
    :source "let port = 8080; in { service = { inherit port; enabled = true; }; }"}
   {:id :reads-home
    :source "builtins.getEnv \"HOME\""}
   {:id :duplicate-generated-name
    :source "{ name = \"svc\"; \"${\"name\"}\" = \"other\"; }"}])

(defn review-row
  [{:keys [id source]}]
  (let [purity (safe/static-purity-check source)
        sandbox (safe/safe-eval source {:pure-only? true})
        evaled (pnix/eval-source source)]
    {:id id
     :pure? (:pure? purity)
     :required-capabilities (mapv :builtin (:impure-uses purity))
     :sandbox (select-keys sandbox [:status :value :reason :limit-exceeded])
     :eval (select-keys evaled [:status :value :reason])}))

(let [rows (mapv review-row candidates)
      by-id (into {} (map (juxt :id identity)) rows)]
  (doseq [row rows]
    (println (:id row) "=>" row))

  (assert (= true (get-in by-id [:pure-service-config :pure?])))
  (assert (= :ok (get-in by-id [:pure-service-config :sandbox :status])))
  (assert (= 8080 (get-in by-id [:pure-service-config :sandbox :value "service" "port"])))

  (assert (= false (get-in by-id [:reads-home :pure?])))
  (assert (= ["getEnv"] (get-in by-id [:reads-home :required-capabilities])))
  (assert (= :held (get-in by-id [:reads-home :sandbox :status])))
  (assert (= :static-impure-use (get-in by-id [:reads-home :sandbox :reason])))

  (assert (= true (get-in by-id [:duplicate-generated-name :pure?])))
  (assert (= :held (get-in by-id [:duplicate-generated-name :eval :status])))
  (assert (= :duplicate-attr (get-in by-id [:duplicate-generated-name :eval :reason]))))

(println)
(println "결론: pnix-clj는 AI config 후보를 merge/배포 전에 pure/impure/duplicate-key verdict로 나눠 review queue에 올릴 수 있다.")
(shutdown-agents)
