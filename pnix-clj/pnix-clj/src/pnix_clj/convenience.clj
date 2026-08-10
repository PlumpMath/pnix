(ns pnix-clj.convenience
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.evaluator :as evaluator])
  (:import [java.nio.charset StandardCharsets]
           [java.nio.file Files Path]))

(def lane-classification
  {:lane :core
   :scope :explicit-filesystem-convenience-boundary
   :role :load-px-source-and-delegate-to-basic-runtime
   :product-runtime :allowed
   :semantic-authority :pnix-meta-px
   :mutation :forbidden
   :admission :none
   :determinism :required-after-source-read
   :allowed-output :runtime-result})

(defn- result-value
  [result]
  (if (= :ok (:status result))
    (:value result)
    (throw (ex-info "PNIX evaluation failed"
                    (select-keys result [:status :reason :error])))))

(defn- import-targets
  [source source-path]
  (let [{:keys [status ast] :as parsed} (pnix/parse-source source)]
    (when-not (= :ok status)
      (throw (ex-info "PNIX module parse failed"
                      (assoc (select-keys parsed [:status :reason :error])
                             :path (str source-path)))))
    (->> (tree-seq coll? seq ast)
         (keep (fn [node]
                 (when (and (map? node) (= :import (:op node)))
                   (:target node))))
         distinct)))

(defn- module-path
  [^Path entry-directory logical-target]
  (let [target (Path/of (str logical-target) (make-array String 0))]
    (.normalize
     (.toAbsolutePath
      (if (.isAbsolute target)
        target
        (.resolve entry-directory target))))))

(defn- import-closure
  "Read the static import closure rooted at one entry source. Filesystem IO is confined to this explicit convenience boundary; pnix-clj.core receives only the existing pure target->source module map.
  하나의 항목 소스에 뿌리를 둔 정적 가져오기 클로저를 읽어보세요. 파일 시스템 IO는 이러한 명시적인 편의 경계로 제한됩니다. pnix-clj.core는 기존 순수 대상->소스 모듈 맵만 수신합니다.
"
  [^Path entry-path entry-source]
  (let [entry-directory (.getParent entry-path)
        modules (atom {})
        loading (atom #{})]
    (letfn [(load-imports! [origin source source-path]
              (doseq [target (import-targets source source-path)]
                (let [logical-target
                      (evaluator/contextual-import-target
                       (if origin [origin] []) target)
                      resolved-path (module-path entry-directory logical-target)]
                  (when-not (contains? @modules logical-target)
                    (when (contains? @loading logical-target)
                      (throw (ex-info "PNIX import cycle"
                                      {:status :failed
                                       :reason :import-cycle
                                       :error {:phase :resolution
                                               :class :import-cycle
                                               :evidence {:target logical-target}}})))
                    (swap! loading conj logical-target)
                    (let [module-source
                          (Files/readString resolved-path StandardCharsets/UTF_8)]
                      (swap! modules assoc logical-target module-source)
                      (load-imports! logical-target module-source resolved-path))
                    (swap! loading disj logical-target)))))]
      (load-imports! nil entry-source entry-path)
      @modules)))

(defn format-source
  "Replace ordered %s source slots. This constructs source text; it is not type representation or a host ABI encoding.
  주문한 %s 소스 슬롯을 교체하십시오. 이는 소스 텍스트를 구성합니다. 이는 유형 표현이나 호스트 ABI 인코딩이 아닙니다.
"
  [source & arguments]
  (when-not (string? source)
    (throw (IllegalArgumentException. "PNIX source must be text. (PNIX 소스는 텍스트여야 합니다.)")))
  (loop [formatted source
         remaining arguments]
    (if-let [argument (first remaining)]
      (let [index (.indexOf ^String formatted "%s")]
        (when (neg? index)
          (throw (IllegalArgumentException.
                   "too many PNIX source format arguments")))
        (recur (str (subs formatted 0 index)
                    argument
                    (subs formatted (+ index 2)))
               (next remaining)))
      (do
        (when (.contains ^String formatted "%s")
          (throw (IllegalArgumentException.
                   "missing PNIX source format argument")))
        formatted))))

(defn px
  "Evaluate PNIX source and return its actual Clojure guest value. (PNIX 소스를 평가하고 실제 Clojure 게스트 값을 반환합니다.)"
  [source & format-arguments]
  (result-value
   (pnix/eval-source (apply format-source source format-arguments))))

(defn px-import
  "Read and evaluate a UTF-8 .px module closure, with optional substitutions applied only to the entry module. Imported modules retain their source text.
  항목 모듈에만 선택적 대체가 적용되는 UTF-8 .px 모듈 클로저를 읽고 평가합니다. 가져온 모듈은 소스 텍스트를 유지합니다.
"
  [path & format-arguments]
  (let [resolved (.normalize
                  (.toAbsolutePath
                   (Path/of (str path) (make-array String 0))))
        source (apply format-source
                      (Files/readString resolved StandardCharsets/UTF_8)
                      format-arguments)
        modules (import-closure resolved source)]
    (result-value (pnix/eval-source-with-imports source modules))))
