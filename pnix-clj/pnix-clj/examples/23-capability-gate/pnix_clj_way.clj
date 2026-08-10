;;; pnix-clj의 방식 — capability/purity gate가 host-effect 요구를 실행 전 판정하고,
;;; 허용/거부를 구조화된 verdict로 남긴다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/23-capability-gate/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.safe-eval :as safe]))

(def pure-source
  "let x = 40; in x + 2")

(def env-source
  "builtins.getEnv \"HOME\"")

(def file-source
  "builtins.readFile \"/etc/passwd\"")

(defn capability-verdict
  [source]
  (let [purity (safe/static-purity-check source)
        result (safe/safe-eval source {:pure-only? true})]
    {:source source
     :static-status (:status purity)
     :pure? (:pure? purity)
     :required-capabilities (mapv :builtin (:impure-uses purity))
     :eval-status (:status result)
     :limit-exceeded (:limit-exceeded result)
     :reason (:reason result)
     :value (:value result)}))

(let [pure (capability-verdict pure-source)
      env-denied (capability-verdict env-source)
      file-denied (capability-verdict file-source)]

  (println "pure source:" pure-source)
  (println "pure verdict:"
           "pure?=" (:pure? pure)
           "eval=" (:eval-status pure)
           "value=" (:value pure))

  (println)
  (println "env source:" env-source)
  (println "env verdict:"
           "pure?=" (:pure? env-denied)
           "required=" (:required-capabilities env-denied)
           "eval=" (:eval-status env-denied)
           "limit=" (:limit-exceeded env-denied)
           "reason=" (:reason env-denied))

  (println)
  (println "file source:" file-source)
  (println "file verdict:"
           "pure?=" (:pure? file-denied)
           "required=" (:required-capabilities file-denied)
           "eval=" (:eval-status file-denied)
           "limit=" (:limit-exceeded file-denied)
           "reason=" (:reason file-denied))

  (assert (= true (:pure? pure)))
  (assert (= :ok (:eval-status pure)))
  (assert (= 42 (:value pure)))

  (assert (= false (:pure? env-denied)))
  (assert (= ["getEnv"] (:required-capabilities env-denied)))
  (assert (= :held (:eval-status env-denied)))
  (assert (= :impure (:limit-exceeded env-denied)))
  (assert (= :static-impure-use (:reason env-denied)))

  (assert (= false (:pure? file-denied)))
  (assert (= ["readFile"] (:required-capabilities file-denied)))
  (assert (= :held (:eval-status file-denied)))
  (assert (= :impure (:limit-exceeded file-denied)))
  (assert (= :static-impure-use (:reason file-denied))))

(println)
(println "결론: pnix-clj는 capability가 필요한 host-effect 요구를 실행 전 gate에서 held verdict/receipt로 남긴다.")
