(ns pnix-clj.clojure-projection.host
  "Host-side crossings used by the Clojure projection fixture lane.

  This namespace owns reader/eval/reflection/classloader/etc. host effects for
  projection fixtures. The parent projection namespace should only turn returned
  host values into canonical terms and validate those terms against the .px
  validator."
  (:require [pnix-clj.interop :as interop]))

(defn read-host-value
  [source]
  (try
    {:status :ok
     :value (binding [*read-eval* false]
              (read-string source))}
    (catch Throwable _
      {:status :failed
       :reason :clojure-reader-failed
       :error {:phase :host-read
               :class :clojure-reader-failed}})))

(defn read-form
  [source]
  (try
    {:status :ok
     :value (binding [*read-eval* false]
              (read-string source))}
    (catch Throwable _
      {:status :failed
       :reason :clojure-form-read-failed
       :error {:phase :host-read
               :class :clojure-form-read-failed}})))

(defn projection-capabilities
  [effect]
  (conj interop/default-capabilities effect))

(defn projection-interop-meta
  [effect]
  (interop/interop-meta {:direction :clojure-projection->host-value
                         :effect-class effect
                         :loss-status :opaque}))

(defn host-form-crossing
  [{:keys [source-id source prefix effect kind]} ok-fn error-fn]
  (let [read-result (read-form source)]
    (if (not= :ok (:status read-result))
      read-result
      (let [form (:value read-result)
            target (interop/fresh-host-ns prefix source-id)
            meta (projection-interop-meta effect)]
        (interop/run-crossing kind
                              meta
                              {:source-id source-id
                               :source source
                               :form form}
                              (projection-capabilities effect)
                              (fn []
                                (try
                                  (let [v (ok-fn target form)]
                                    (if (and (map? v) (contains? v :status))
                                      v
                                      {:status :ok :value v}))
                                  (catch Throwable t
                                    (let [v (error-fn t)]
                                      (if (and (map? v) (contains? v :status))
                                        v
                                        {:status :ok :value v}))))))))))

(defn host-eval-source
  [source-id source]
  (host-form-crossing
   {:source-id source-id
    :source source
    :prefix "pnix-clj.projection-host"
    :effect :host-eval
    :kind :clojure-projection-host-form}
   (fn [target form]
     (binding [*ns* target]
       (eval form)))
   (fn [_]
     {:status :failed
      :reason :clojure-host-form-eval-failed
      :error {:phase :host-eval
              :class :clojure-host-form-eval-failed}})))
