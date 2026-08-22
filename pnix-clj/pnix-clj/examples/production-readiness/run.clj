(require '[clojure.java.io :as io]
         '[pnix-clj.core :as pnix])

(def example-root (.getParentFile (io/file *file*)))
(defn example [name]
  (.getCanonicalPath (io/file example-root name)))
(defn done-value [name]
  (let [result (pnix/eval-file (example name))]
    (assert (= :ok (:status result)) (pr-str result))
    (:value result)))

(let [direct (done-value "direct.px")
      consumer (done-value "consumer.px")
      self-hosted (done-value "self_interpreter.px")
      called-double (pnix/call-file (example "library.px") "double" [21])
      called-map (pnix/call-file (example "library.px") "mapDouble" [[1 2 3]])
      meta-result (pnix/compile-source "let double = x: x * 2; in double 21")]
  ;; Host import returns native Clojure maps/vectors/numbers.
  (assert (map? direct))
  (assert (= 42 (get direct "value")))
  (assert (= {"answer" 42
              "count" 4
              "mapped" [2 4 6]
              "total" 10
              "version" "pnix-library-seed-v1"}
             consumer))
  (assert (= "pnix-in-pnix" (get self-hosted "mode")))
  (assert (= 42 (get self-hosted "value")))
  (assert (= {:status :ok :value 42} called-double))
  (assert (= {:status :ok :value [2 4 6]} called-map))

  ;; PNIX -> clj-meta is a live execution path, not only a receipt check.
  (assert (= :ok (:status meta-result)) (pr-str meta-result))
  (assert (= 42 (get-in meta-result [:clj-meta-result :value])))
  (println "PASS pnix-clj production-readiness"))

(shutdown-agents)
