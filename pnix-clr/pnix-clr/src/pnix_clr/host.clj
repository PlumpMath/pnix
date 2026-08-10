(ns pnix-clr.host
  "CLR-only filesystem adapter for the seed runtime. Root checks are lexical
  bootstrap confinement, not a symlink-safe capability or security boundary."
  (:require [clojure.string :as str]
            [pnix-clr.outcome :as outcome]))

(defn canonical-path
  [path]
  (System.IO.Path/GetFullPath (str path)))

(defn combine
  [& parts]
  (reduce (fn [left right]
            (System.IO.Path/Combine (str left) (str right)))
          parts))

(defn default-root
  "The lexical confinement root for `.px` resolution: PNIX_CLR_ROOT when set,
  otherwise the current directory. pnix-clr resolves only within its own tree —
  it has no sibling-repository dependency."
  []
  (or (System.Environment/GetEnvironmentVariable "PNIX_CLR_ROOT")
      (canonical-path (System.IO.Directory/GetCurrentDirectory))))

(defn read-source
  [path]
  (try
    (System.IO.File/ReadAllText (canonical-path path))
    (catch System.IO.FileNotFoundException _
      (outcome/fail! :resolution :import-module-not-found
                     {:reason "file-read-failed"}))
    (catch System.IO.DirectoryNotFoundException _
      (outcome/fail! :resolution :import-module-not-found
                     {:reason "file-read-failed"}))))

(defn read-file-text
  "Read an entire file as UTF-8 text. Used by builtins.readFile."
  [path]
  (try
    (System.IO.File/ReadAllText (canonical-path path))
    (catch System.Exception e
      (outcome/fail! :eval :type-error
                     {:operation "readFile"
                      :reason (.Message e)
                      :path (str path)}))))

(defn file-exists?
  [path]
  (System.IO.File/Exists (canonical-path path)))

(defn path-exists
  "True if path names an existing file or directory."
  [path]
  (let [p (canonical-path path)]
    (or (System.IO.File/Exists p)
        (System.IO.Directory/Exists p))))

(defn- reparse-point?
  [^System.IO.FileSystemInfo info]
  (not= 0 (bit-and (int (.Attributes info))
                   (int System.IO.FileAttributes/ReparsePoint))))

(defn- entry-kind
  [^System.IO.FileSystemInfo info]
  (cond
    (reparse-point? info) "symlink"
    (instance? System.IO.DirectoryInfo info) "directory"
    (instance? System.IO.FileInfo info) "regular"
    :else "unknown"))

(defn list-directory
  "Map entry name -> \"regular\"|\"directory\"|\"symlink\"|\"unknown\".
   Uses System.IO only."
  [path]
  (let [path (canonical-path path)]
    (when-not (System.IO.Directory/Exists path)
      (outcome/fail! :eval :type-error
                     {:operation "readDir"
                      :reason "not-a-directory"
                      :path path}))
    (let [di (System.IO.DirectoryInfo. path)
          infos (concat (.GetDirectories di) (.GetFiles di))]
      (into {}
            (map (fn [^System.IO.FileSystemInfo info]
                   [(.Name info) (entry-kind info)]))
            infos))))

(defn- hex-lower
  [^bytes data]
  (let [sb (System.Text.StringBuilder. (* 2 (alength data)))]
    (dotimes [i (alength data)]
      (.AppendFormat sb "{0:x2}" (aget data i)))
    (.ToString sb)))

(defn write-store-file
  "Write contents under /tmp/pnix-nix-store/<hash>-name and return absolute path."
  [name contents]
  (let [name (str name)
        contents (str contents)
        encoding (System.Text.UTF8Encoding. false)
        digest (.ComputeHash (System.Security.Cryptography.SHA256/Create)
                             (.GetBytes encoding contents))
        hash (hex-lower digest)
        store-root (combine (System.IO.Path/GetTempPath) "pnix-nix-store")
        file-name (str hash "-" name)
        target (combine store-root file-name)]
    (System.IO.Directory/CreateDirectory store-root)
    (System.IO.File/WriteAllText (canonical-path target) contents)
    (canonical-path target)))

(defn- portable-path
  [path]
  (str/replace (str path) "\\" "/"))

(defn resolve-entry
  [root relative-path]
  (let [root-path (canonical-path root)
        candidate (canonical-path (combine root-path relative-path))
        relative (portable-path (System.IO.Path/GetRelativePath root-path candidate))]
    (when (or (= relative "..")
              (str/starts-with? relative "../")
              (System.IO.Path/IsPathRooted relative))
      (outcome/fail! :resolution :import-module-not-found
                     {:reason "path-outside-root"}))
    (when-not (file-exists? candidate)
      (outcome/fail! :resolution :import-module-not-found
                     {:reason "module-not-found"}))
    candidate))

(defn resolve-import
  [root importer target]
  (when-not (string? target)
    (outcome/fail! :resolution :type-error {:operation "import"}))
  (let [base (System.IO.Path/GetDirectoryName (canonical-path importer))
        candidate (canonical-path (combine base target))
        root-path (canonical-path root)
        relative (portable-path (System.IO.Path/GetRelativePath root-path candidate))]
    (when (or (= relative "..")
              (str/starts-with? relative "../")
              (System.IO.Path/IsPathRooted relative))
      (outcome/fail! :resolution :import-module-not-found
                     {:reason "path-outside-root"}))
    (when-not (file-exists? candidate)
      (outcome/fail! :resolution :import-module-not-found
                     {:reason "module-not-found"}))
    candidate))

(defn exit!
  [code]
  (System.Environment/Exit (int code)))
