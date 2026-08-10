(ns pnix.cljs-meta.core
  (:require [cljs.js :as cljs]))

(defonce compiler-state (cljs/empty-state))

(defn evaluate
  "Evaluate one ClojureScript expression through cljs.js.

  The callback receives a native ClojureScript result map."
  [source callback]
  (cljs/eval-str compiler-state
                 source
                 "cljs-meta-input.cljs"
                 {:eval cljs/js-eval
                  :context :expr
                  :source-map false}
                 callback))

(defn result-projection [result]
  (if-let [error (:error result)]
    {"schema" "pnix.cljs-meta.result.v1"
     "outcome_kind" "failed"
     "error" {"phase" "host-eval"
              "class" "clojurescript-evaluation-error"
              "message" (or (.-message error) (str error))}}
    {"schema" "pnix.cljs-meta.result.v1"
     "outcome_kind" "done"
     "value" (:value result)}))

(defn evaluate-promise [source]
  (js/Promise.
    (fn [resolve _reject]
      (evaluate source
                (fn [result]
                  (resolve (clj->js (result-projection result))))))))

(defn compile-promise [source]
  (js/Promise.
    (fn [resolve _reject]
      (cljs/compile-str compiler-state
                        source
                        "cljs-meta-user.cljs"
                        {:context :statement
                         :source-map false
                         :target :nodejs}
                        (fn [result]
                          (resolve (clj->js (result-projection result))))))))
