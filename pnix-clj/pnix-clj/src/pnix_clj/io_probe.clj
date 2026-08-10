(ns pnix-clj.io-probe
  (:require [clojure.java.io :as io]
            [pnix-clj.interop :as interop]
            [pnix-clj.json :as json]))

(def lane-classification
  {:lane :core
   :scope :read-only-host-io-probe-adapter
   :role :execute-common-read-only-effect-requests
   :product-runtime :allowed
   :semantic-authority :loads-pnix-meta-owns-no-semantics
   :mutation :read-only-external-filesystem
   :admission :tri-meta-io-gate
   :determinism :required
   :allowed-output :canonical-effect-receipt})

(def capability-class
  {"capability_id" "pnix.io.file-read.v1"
   "entry_point" "host-meta-io-v1"
   "input_signature" "{path:string}"
   "output_shape" "value+receipt"
   "effect_scope" "read-only-filesystem"
   "risk_tier" "bounded-read"
   "discovery_source" "pnix-meta.effect-request.v1"})

(defn request [effect path]
  {"operation_id" effect
   "args" {"path" path}
   "capability_class" capability-class})

(defn probe [root]
  (let [note (str (io/file root "note.txt"))
        missing (str (io/file root "missing.txt"))
        grants #{:file-read}
        exists (interop/apply-effect-request (request "fs.path-exists" note) grants)
        missing-result (interop/apply-effect-request (request "fs.path-exists" missing) grants)
        opened (interop/apply-effect-request (request "fs.open" note) grants)
        typed (interop/apply-effect-request (request "fs.file-type" note) grants)
        listed (interop/apply-effect-request (request "fs.read-dir" root) grants)
        denied (interop/apply-effect-request (request "fs.open" note) #{})
        adapter-error (interop/apply-effect-request (request "fs.open" missing) grants)
        invalid (interop/apply-effect-request (request "fs.open" nil) grants)
        unsupported (interop/apply-effect-request (request "fs.unknown" note) grants)
        report {"schema" "pnix-meta.host-io-probe.v1"
                "adapter_error" (get adapter-error "error")
                "path_exists" (get exists "value")
                "missing_exists" (get missing-result "value")
                "open" (get opened "value")
                "file_type" (get typed "value")
                "read_dir" (get listed "value")
                "denied" (get denied "error")
                "invalid" (get invalid "error")
                "unsupported" (get unsupported "error")
                "receipt_adapter" (get-in opened ["receipt" "adapter"])}]
    (assoc report "all_ok"
           (and (= {"phase" "effect" "class" "effect-adapter-error"}
                   (get report "adapter_error"))
                (= true (get report "path_exists"))
                (= false (get report "missing_exists"))
                (= "hello" (get report "open"))
                (= "regular" (get report "file_type"))
                (= {"note.txt" "regular" "subdir" "directory"}
                   (get report "read_dir"))
                (= {"phase" "effect" "class" "effect-denied"}
                   (get report "denied"))
                (= {"phase" "effect-contract" "class" "invalid-effect-args"}
                   (get report "invalid"))
                (= {"phase" "effect-contract" "class" "unknown-effect-operation"}
                   (get report "unsupported"))
                (= "host-meta-io-v1" (get report "receipt_adapter"))))))

(defn -main [& [root]]
  (let [result (probe root)]
    (println (json/write-json result))
    (when-not (get result "all_ok")
      (System/exit 1))))
