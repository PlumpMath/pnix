(ns pnix-clj.primitive-kernel
  (:require [clojure.data.json :as json]))

(def lane-classification
  {:lane :core
   :scope :production-checked-i64-primitive-kernel
   :role :closed-native-primitive-kernel
   :product-runtime :allowed
   :semantic-authority :pnix-meta-manifest-only
   :mutation :forbidden
   :admission :forbidden
   :determinism :required
   :allowed-output :sealed-primitive-outcome})

(def abi-version "pnix.primitive-abi.v1")
(def manifest-digest "f133ee0f3a5c6073eabb6855f3abf44bf36366083f26fbe76e9524521a2a5fd6")

(def checked-integer-primitive-ids
  ["i64-add-checked"
   "i64-sub-checked"
   "i64-mul-checked"
   "i64-div-checked"])

(def operator->primitive-id
  {"+" "i64-add-checked"
   "-" "i64-sub-checked"
   "*" "i64-mul-checked"
   "/" "i64-div-checked"})

(defn- ok [value]
  {:kind :ok :value value})

(defn- failure [phase class]
  {:kind :error :phase phase :class class})

(defn- contract-failure []
  (failure :primitive-contract :primitive-contract-violation))

(defn- i64? [value]
  (and (integer? value)
       (<= Long/MIN_VALUE value Long/MAX_VALUE)))

(defn- checked-op [primitive-id left right]
  (case primitive-id
    "i64-add-checked" (ok (Math/addExact (long left) (long right)))
    "i64-sub-checked" (ok (Math/subtractExact (long left) (long right)))
    "i64-mul-checked" (ok (Math/multiplyExact (long left) (long right)))
    "i64-div-checked" (cond
                        (zero? right)
                        (failure :eval :division-by-zero)

                        (and (= left Long/MIN_VALUE) (= right -1))
                        (failure :eval :integer-overflow)

                        :else
                        (ok (quot (long left) (long right))))
    (contract-failure)))

(defn invoke
  [{:keys [abi_version manifest_sha256 primitive_id operands]}]
  (try
    (cond
      (not= abi_version abi-version) (contract-failure)
      (not= manifest_sha256 manifest-digest) (contract-failure)
      (not (some #{primitive_id} checked-integer-primitive-ids)) (contract-failure)
      (not= 2 (count operands)) (contract-failure)
      (not (every? i64? operands)) (failure :eval :type-error)
      :else (let [[left right] operands]
              (checked-op primitive_id left right)))
    (catch ArithmeticException _
      (failure :eval :integer-overflow))
    (catch Throwable _
      (contract-failure))))

(defn- legacy-invoke [operator left right]
  (try
    (case operator
      "+" (ok (+ (long left) (long right)))
      "-" (ok (- (long left) (long right)))
      "*" (ok (* (long left) (long right)))
      "/" (cond
            (zero? right) (failure :eval :division-by-zero)
            (and (= left Long/MIN_VALUE) (= right -1))
            (failure :eval :integer-overflow)
            :else (ok (quot (long left) (long right))))
      (contract-failure))
    (catch ArithmeticException _
      (failure :eval :integer-overflow))
    (catch Throwable _
      (contract-failure))))

(defn invoke-shadow [operator left right]
  (if-let [primitive-id (get operator->primitive-id operator)]
    (let [legacy (legacy-invoke operator left right)
          routed (invoke {:abi_version abi-version
                          :manifest_sha256 manifest-digest
                          :primitive_id primitive-id
                          :operands [left right]})]
      (if (= legacy routed) routed (contract-failure)))
    (contract-failure)))

(defn- public-outcome [case-name outcome]
  (cond-> {"case" case-name
           "kind" (name (:kind outcome))}
    (contains? outcome :value) (assoc "value" (:value outcome))
    (:phase outcome) (assoc "phase" (name (:phase outcome)))
    (:class outcome) (assoc "class" (name (:class outcome)))))

(defn- matrix []
  (mapv (fn [[case-name operator left right]]
          (public-outcome case-name (invoke-shadow operator left right)))
        [["add-positive" "+" 1 2]
         ["sub-signed" "-" -7 5]
         ["mul-signed" "*" -7 -6]
         ["div-negative-left" "/" -7 3]
         ["div-negative-right" "/" 7 -3]
         ["add-overflow" "+" Long/MAX_VALUE 1]
         ["sub-overflow" "-" Long/MIN_VALUE 1]
         ["mul-overflow" "*" Long/MAX_VALUE 2]
         ["div-overflow" "/" Long/MIN_VALUE -1]
         ["division-by-zero" "/" 1 0]]))

(defn- contract-matrix []
  (let [base {:abi_version abi-version
              :manifest_sha256 manifest-digest
              :primitive_id "i64-add-checked"
              :operands [1 2]}]
    [(public-outcome "wrong-abi" (invoke (assoc base :abi_version "wrong")))
     (public-outcome "wrong-digest" (invoke (assoc base :manifest_sha256 "wrong")))
     (public-outcome "unknown-id" (invoke (assoc base :primitive_id "unknown")))
     (public-outcome "wrong-arity" (invoke (assoc base :operands [1])))]))

(defn report []
  {"schema" "pnix.production-primitive-gate.v1"
   "abi_version" abi-version
   "manifest_digest" manifest-digest
   "checked_integer_primitive_ids" checked-integer-primitive-ids
   "strict_args" (zipmap checked-integer-primitive-ids (repeat [0 1]))
   "execution_error_classes"
   {"i64-add-checked" ["integer-overflow"]
    "i64-sub-checked" ["integer-overflow"]
    "i64-mul-checked" ["integer-overflow"]
    "i64-div-checked" ["division-by-zero" "integer-overflow"]}
   "validation_error_classes" (zipmap checked-integer-primitive-ids
                                            (repeat ["type-error"]))
   "force_order" [0 1]
   "shadow_mode" true
   "matrix" (matrix)
   "contract_matrix" (contract-matrix)
   "status" {"production_checked_i64_manifest_enforced" true
             "production_evaluator_manifest_enforced" false
             "full_builtin_surface_manifest_enforced" false}})

(defn -main [& _]
  (println (json/write-str (report))))
