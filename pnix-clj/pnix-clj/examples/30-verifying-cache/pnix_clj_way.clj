;;; pnix-clj의 방식 — content-addressed cache hit를 fresh eval, purity verdict,
;;; cache key와 대조해서 verified reuse로 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/30-verifying-cache/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.cached-eval :as ce]
            [pnix-clj.core :as pnix]
            [pnix-clj.safe-eval :as safe]))

(def source-a
  "1 + 2")

(def source-b
  "1 +   2")

(def source-c
  "(1 + 2)")

(def impure-source
  "builtins.getEnv \"HOME\"")

(defn verified-cache-row
  [label source expected-cache-status]
  (let [cached (ce/cached-eval source)
        fresh (pnix/eval-source source)
        purity (safe/static-purity-check source)
        cache-status (get-in cached [:cache :status])
        verified? (and (= expected-cache-status cache-status)
                       (= :ok (:status cached))
                       (= :ok (:status fresh))
                       (= (:value cached) (:value fresh))
                       (= true (:pure? purity)))]
    {:label label
     :source source
     :cache-status cache-status
     :cache-key (get-in cached [:cache :key])
     :cached-value (:value cached)
     :fresh-value (:value fresh)
     :pure? (:pure? purity)
     :verified? verified?}))

(defn bypass-row
  [label source expected-reason]
  (let [cached (ce/cached-eval source)
        purity (safe/static-purity-check source)]
    {:label label
     :source source
     :cache-status (get-in cached [:cache :status])
     :cache-reason (get-in cached [:cache :reason])
     :pure? (:pure? purity)
     :required-capabilities (mapv :builtin (:impure-uses purity))
     :verified-bypass? (and (= :bypass (get-in cached [:cache :status]))
                            (= expected-reason (get-in cached [:cache :reason]))
                            (= false (:pure? purity)))}))

(ce/clear-eval-cache!)

(let [row-a (verified-cache-row :first-miss source-a :miss)
      row-b (verified-cache-row :whitespace-hit source-b :hit)
      row-c (verified-cache-row :paren-hit source-c :hit)
      impure (bypass-row :impure-bypass impure-source :statically-impure)
      keys-same? (= (:cache-key row-a) (:cache-key row-b) (:cache-key row-c))
      values-same? (= (:cached-value row-a)
                      (:cached-value row-b)
                      (:cached-value row-c)
                      (:fresh-value row-a)
                      (:fresh-value row-b)
                      (:fresh-value row-c))
      all-verified? (and (:verified? row-a)
                         (:verified? row-b)
                         (:verified? row-c)
                         (:verified-bypass? impure)
                         keys-same?
                         values-same?)]

  (println "source-a:" source-a)
  (println "cache-a:" (:cache-status row-a)
           "cached=" (:cached-value row-a)
           "fresh=" (:fresh-value row-a)
           "verified?=" (:verified? row-a))

  (println)
  (println "source-b:" source-b)
  (println "cache-b:" (:cache-status row-b)
           "cached=" (:cached-value row-b)
           "fresh=" (:fresh-value row-b)
           "verified?=" (:verified? row-b))

  (println)
  (println "source-c:" source-c)
  (println "cache-c:" (:cache-status row-c)
           "cached=" (:cached-value row-c)
           "fresh=" (:fresh-value row-c)
           "verified?=" (:verified? row-c))

  (println)
  (println "content keys same?:" keys-same?)
  (println "cached/fresh values same?:" values-same?)
  (println "cache stats:" (ce/eval-cache-stats))

  (println)
  (println "impure source:" impure-source)
  (println "impure cache:"
           "status=" (:cache-status impure)
           "reason=" (:cache-reason impure)
           "pure?=" (:pure? impure)
           "required=" (:required-capabilities impure)
           "verified-bypass?=" (:verified-bypass? impure))

  (println)
  (println "all verified?:" all-verified?)

  (assert (= :miss (:cache-status row-a)))
  (assert (= :hit (:cache-status row-b)))
  (assert (= :hit (:cache-status row-c)))
  (assert (= true (:verified? row-a)))
  (assert (= true (:verified? row-b)))
  (assert (= true (:verified? row-c)))
  (assert (= true keys-same?))
  (assert (= true values-same?))

  (assert (= :bypass (:cache-status impure)))
  (assert (= :statically-impure (:cache-reason impure)))
  (assert (= false (:pure? impure)))
  (assert (= ["getEnv"] (:required-capabilities impure)))
  (assert (= true (:verified-bypass? impure)))

  (assert (= true all-verified?)))

(println)
(println "결론: pnix-clj는 cache hit를 semantic authority로 믿지 않고 fresh eval/purity/key와 대조해 verified reuse로 만든다.")
