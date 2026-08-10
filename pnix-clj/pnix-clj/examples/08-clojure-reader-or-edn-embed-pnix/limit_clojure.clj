;;; plain Clojure/EDN의 한계 — tagged literal을 데이터로 읽을 수는 있지만,
;;; pnix parse/purity/tower/witness 검증은 기본으로 생기지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/08-clojure-reader-or-edn-embed-pnix/limit_clojure.clj

(ns reader-edn-embed-limit
  (:require [clojure.edn :as edn]))

(def edn-cell
  "{:cell #px \"let x = 40; in x + 2\" :label \"answer\"}")

(defn px-reader
  [source]
  {:tag :px
   :source source})

(let [without-reader
      (try
        (edn/read-string edn-cell)
        {:status :unexpected-ok}
        (catch RuntimeException e
          {:status :rejected
           :error-class (-> e class str)}))

      with-reader
      (edn/read-string {:readers {'px px-reader}} edn-cell)]

  (println "edn cell:" edn-cell)
  (println "without custom reader:" without-reader)
  (println "with custom reader:" with-reader)
  (println "pnix parse verdict:" nil)
  (println "purity verdict:" nil)
  (println "tower collapse:" nil)
  (println "tower witness:" nil)

  (assert (= :rejected (:status without-reader)))
  (assert (= {:cell {:tag :px :source "let x = 40; in x + 2"}
              :label "answer"}
             with-reader)))

(println)
(println "결론: plain EDN reader는 #px 데이터를 만들 수는 있지만 pnix 검증/tower/witness를 기본으로 주지 않는다.")
