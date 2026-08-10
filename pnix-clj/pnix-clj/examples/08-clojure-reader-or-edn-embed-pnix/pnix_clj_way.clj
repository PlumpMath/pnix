;;; pnix-clj의 방식 — EDN tagged literal은 데이터로만 읽고,
;;; 그 안의 pnix source를 parse/purity/tower로 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/08-clojure-reader-or-edn-embed-pnix/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [clojure.edn :as edn]
            [pnix-clj.parser :as parser]
            [pnix-clj.safe-eval :as safe]
            [pnix-clj.tower :as tower]))

(def pure-edn
  "{:cell #px \"let x = 40; in x + 2\" :label \"answer\"}")

(def impure-edn
  "{:cell #px \"builtins.getEnv \\\"HOME\\\"\" :label \"env\"}")

(defn px-reader
  [source]
  {:kind :pnix-edn-embed
   :source source})

(defn read-px-doc
  [s]
  (edn/read-string {:readers {'px px-reader}} s))

(defn verify-px-cell
  [{:keys [kind source] :as cell}]
  (let [parsed (parser/parse-source source)
        purity (safe/static-purity-check source)
        required (mapv :builtin (:impure-uses purity))]
    (cond
      (not= :pnix-edn-embed kind)
      {:status :held
       :reason :not-pnix-edn-embed
       :cell cell}

      (not= :ok (:status parsed))
      {:status :held
       :reason :parse-failed
       :parse-status (:status parsed)
       :source source}

      (not (:pure? purity))
      {:status :held
       :reason :impure-embed
       :parse-status (:status parsed)
       :pure? (:pure? purity)
       :required-capabilities required
       :source source}

      :else
      (let [t (tower/run-tower source)
            collapse (:collapse t)
            witness (:witness collapse)
            ok? (= :collapsed (:status collapse))]
        {:status (if ok? :ok :held)
         :reason (when-not ok? :tower-did-not-collapse)
         :source source
         :parse-status (:status parsed)
         :pure? (:pure? purity)
         :required-capabilities required
         :tower-collapse (:status collapse)
         :tower-value (:value collapse)
         :tower-witness witness}))))

(let [pure-doc (read-px-doc pure-edn)
      impure-doc (read-px-doc impure-edn)
      pure-result (verify-px-cell (:cell pure-doc))
      impure-result (verify-px-cell (:cell impure-doc))]

  (println "pure EDN:" pure-edn)
  (println "pure read:" pure-doc)
  (println "pure verification:")
  (println " parse=" (:parse-status pure-result))
  (println " pure?=" (:pure? pure-result))
  (println " required=" (:required-capabilities pure-result))
  (println " tower-collapse=" (:tower-collapse pure-result)
           "value=" (:tower-value pure-result))
  (println " witness=" (:tower-witness pure-result))
  (println " status=" (:status pure-result))

  (println)
  (println "impure EDN:" impure-edn)
  (println "impure read:" impure-doc)
  (println "impure verification:")
  (println " parse=" (:parse-status impure-result))
  (println " pure?=" (:pure? impure-result))
  (println " required=" (:required-capabilities impure-result))
  (println " reason=" (:reason impure-result))
  (println " status=" (:status impure-result))

  (assert (= :ok (:status pure-result)))
  (assert (= :ok (:parse-status pure-result)))
  (assert (= true (:pure? pure-result)))
  (assert (= [] (:required-capabilities pure-result)))
  (assert (= :collapsed (:tower-collapse pure-result)))
  (assert (= 42 (:tower-value pure-result)))
  (assert (string? (:source-hash (:tower-witness pure-result))))
  (assert (string? (:ast-hash (:tower-witness pure-result))))
  (assert (= :ok (:cross-mirror (:tower-witness pure-result))))

  (assert (= :held (:status impure-result)))
  (assert (= :ok (:parse-status impure-result)))
  (assert (= false (:pure? impure-result)))
  (assert (= ["getEnv"] (:required-capabilities impure-result)))
  (assert (= :impure-embed (:reason impure-result))))

(println)
(println "결론: pnix-clj는 EDN #px 데이터를 실행하지 않고 읽은 뒤 pnix parse/purity/tower witness로 검증한다.")
