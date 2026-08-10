(ns pnix.clj-meta.io
  "Pnix-agnostic, capability-gated read-only host I/O substrate.

  This namespace knows nothing about pnix effect requests or values. Active
  runtimes adapt these four operations at their own interop boundary."
  (:import [java.nio.charset CharacterCodingException StandardCharsets]
           [java.nio.file Files LinkOption NoSuchFileException
            NotDirectoryException Path Paths]))

(def file-read-capability "file-read")

(defn- capability-name [cap]
  (cond
    (keyword? cap) (name cap)
    (string? cap) cap
    :else (str cap)))

(defn file-read-granted? [granted]
  (boolean (some #(= file-read-capability (capability-name %)) granted)))

(defn- require-file-read! [granted]
  (when-not (file-read-granted? granted)
    (throw (ex-info "file-read capability denied"
                    {:error-class :capability-denied
                     :capability file-read-capability}))))

(defn- as-path ^Path [path]
  (Paths/get (str path) (make-array String 0)))

(defn- classify-path [^Path path]
  (cond
    (Files/isSymbolicLink path) "symlink"
    (Files/isDirectory path (make-array LinkOption 0)) "directory"
    (Files/isRegularFile path (make-array LinkOption 0)) "regular"
    :else "unknown"))

(defn path-exists [path granted]
  (require-file-read! granted)
  (Files/exists (as-path path) (make-array LinkOption 0)))

(defn file-type [path granted]
  (require-file-read! granted)
  (let [p (as-path path)]
    (when-not (or (Files/exists p (make-array LinkOption 0))
                  (Files/isSymbolicLink p))
      (throw (ex-info "file type target not found"
                      {:error-class :not-found})))
    (classify-path p)))

(defn read-utf8 [path granted]
  (require-file-read! granted)
  (try
    (Files/readString (as-path path) StandardCharsets/UTF_8)
    (catch NoSuchFileException e
      (throw (ex-info "read target not found" {:error-class :not-found} e)))
    (catch CharacterCodingException e
      (throw (ex-info "read target is not UTF-8" {:error-class :invalid-utf8} e)))
    (catch java.io.IOException e
      (throw (ex-info "read failed" {:error-class :io-error} e)))))

(defn read-dir [path granted]
  (require-file-read! granted)
  (try
    (with-open [stream (Files/newDirectoryStream (as-path path))]
      (into (sorted-map)
            (map (fn [^Path entry]
                   [(str (.getFileName entry)) (classify-path entry)]))
            (iterator-seq (.iterator stream))))
    (catch NoSuchFileException e
      (throw (ex-info "directory not found" {:error-class :not-found} e)))
    (catch NotDirectoryException e
      (throw (ex-info "target is not a directory" {:error-class :not-directory} e)))
    (catch java.io.IOException e
      (throw (ex-info "directory read failed" {:error-class :io-error} e)))))

(defn report []
  (let [granted #{file-read-capability}
        denied (try
                 (path-exists "deps.edn" #{})
                 false
                 (catch clojure.lang.ExceptionInfo e
                   (= :capability-denied (:error-class (ex-data e)))))]
    {:schema :clj-meta.io.v1
     :ready (and denied
                 (path-exists "deps.edn" granted)
                 (= "regular" (file-type "deps.edn" granted))
                 (contains? (read-dir "." granted) "src"))
     :capability file-read-capability
     :effects ["path-exists" "open" "file-type" "read-dir"]}))

(defn -main [& _]
  (let [result (report)]
    (prn result)
    (when-not (:ready result)
      (System/exit 1))))
