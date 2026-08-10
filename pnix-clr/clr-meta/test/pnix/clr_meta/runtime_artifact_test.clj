(ns pnix.clr-meta.runtime-artifact-test
  (:require [clojure.test :refer [deftest is testing]]
            [pnix.clr-meta.runtime-artifact :as artifact]))

(defn- temp-directory
  []
  (let [path (System.IO.Path/Combine
              (System.IO.Path/GetTempPath)
              (str "pnix-clr-meta-artifact-test-"
                   (.ToString (System.Guid/NewGuid) "N")))]
    (System.IO.Directory/CreateDirectory path)
    path))

(defn- write-text!
  [path text]
  (let [parent (System.IO.Path/GetDirectoryName path)]
    (when (and parent (not= "" parent))
      (System.IO.Directory/CreateDirectory parent))
    (System.IO.File/WriteAllText
     path text (System.Text.UTF8Encoding. false true))))

(defn- failure-class
  [f]
  (try
    (f)
    nil
    (catch clojure.lang.ExceptionInfo cause
      (:class (ex-data cause)))))

(def valid-plan
  {:schema :pnix.clr-meta.runtime-artifact-plan.v1
   :entry 'fixture.main
   :namespaces ['fixture.core 'fixture.main]})

(deftest hashes-exact-utf8-and-ordered-closure-bytes
  (is (= "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
         (artifact/sha256-string "abc")))
  (is (= (artifact/sha256-string "aa  one\nbb  two\n")
         (artifact/closure-hash [{:path "one" :sha256 "aa"}
                                 {:path "two" :sha256 "bb"}]))))

(deftest plan-validation-is-strict-and-pnix-agnostic
  (let [validated (artifact/validate-plan valid-plan)]
    (is (= ['fixture.core 'fixture.main] (:namespaces validated)))
    (is (= ["fixture/core.clj" "fixture/main.clj"]
           (:source-paths validated)))
    (is (= ["fixture.core.clj.dll" "fixture.main.clj.dll"]
           (:output-paths validated))))
  (testing "schema, exact keys, ordering type, identity, and safe paths fail closed"
    (is (= :plan-schema
           (failure-class #(artifact/validate-plan
                            (assoc valid-plan :schema :wrong)))))
    (is (= :plan-key-set
           (failure-class #(artifact/validate-plan
                            (assoc valid-plan :extra true)))))
    (is (= :namespaces-not-vector
           (failure-class #(artifact/validate-plan
                            (assoc valid-plan :namespaces '(fixture.main))))))
    (is (= :duplicate-namespace
           (failure-class #(artifact/validate-plan
                            (assoc valid-plan
                                   :namespaces ['fixture.main 'fixture.main])))))
    (is (= :entry-not-declared
           (failure-class #(artifact/validate-plan
                            (assoc valid-plan :entry 'fixture.missing)))))
    (is (= :invalid-namespace
           (failure-class #(artifact/validate-plan
                            (assoc valid-plan
                                   :namespaces ['fixture.core '../escape])))))
    (is (= :source-path-collision
           (failure-class #(artifact/validate-plan
                            {:schema artifact/plan-schema
                             :entry 'fixture.foo-bar
                             :namespaces ['fixture.foo-bar 'fixture.foo_bar]}))))))

(deftest source-closure-must-equal-the-declared-plan
  (let [root (temp-directory)
        source-root (System.IO.Path/Combine root "src")]
    (try
      (write-text! (System.IO.Path/Combine source-root "fixture" "core.clj")
                   "(ns fixture.core)\n")
      (is (= :source-set-mismatch
             (failure-class #(artifact/validate-plan valid-plan source-root))))
      (write-text! (System.IO.Path/Combine source-root "fixture" "main.clj")
                   "(ns fixture.main)\n")
      (is (= ["fixture/core.clj" "fixture/main.clj"]
             (mapv :path (:sources
                          (artifact/validate-plan valid-plan source-root)))))
      (write-text! (System.IO.Path/Combine source-root "fixture" "extra.clj")
                   "(ns fixture.extra)\n")
      (is (= :source-set-mismatch
             (failure-class #(artifact/validate-plan valid-plan source-root))))
      (finally
        (System.IO.Directory/Delete root true)))))

(deftest build-emits-a-closed-hash-bound-host-aot-artifact
  (let [root (temp-directory)
        source-root (System.IO.Path/Combine root "src")
        output-root (System.IO.Path/Combine root "artifact")
        plan-path (System.IO.Path/Combine root "runtime-artifact.edn")]
    (try
      (write-text! (System.IO.Path/Combine source-root "fixture" "core.clj")
                   "(ns fixture.core)\n(defn answer [] 42)\n")
      (write-text! (System.IO.Path/Combine source-root "fixture" "main.clj")
                   (str "(ns fixture.main\n"
                        "  (:require [fixture.core :as core]))\n"
                        "(defn -main [& _] (core/answer))\n"))
      (write-text! plan-path (str (pr-str valid-plan) "\n"))
      (let [manifest (artifact/build! plan-path output-root source-root)
            manifest-path (System.IO.Path/Combine output-root "manifest.json")]
        (is (= artifact/manifest-schema (get manifest "schema")))
        (is (= "clr-meta" (get manifest "producer")))
        (is (= "host-clojureclr-aot" (get manifest "backend")))
        (is (= "net10.0" (get manifest "target")))
        (is (= "fixture.main" (get manifest "entry")))
        (is (= 3 (get manifest "evaluator_generations")))
        (is (false? (get manifest "compiler_stage15_n")))
        (is (false? (get manifest "compiler_self_reproduction")))
        (is (false? (get manifest "il_fixed_point")))
        (is (= (artifact/sha256-file plan-path)
               (get manifest "plan_sha256")))
        (is (= ["fixture/core.clj" "fixture/main.clj"]
               (mapv #(get % "path") (get manifest "sources"))))
        (is (= ["fixture.core.clj.dll" "fixture.main.clj.dll"]
               (mapv #(get % "path") (get manifest "outputs"))))
        (is (every? #(System.IO.File/Exists
                      (System.IO.Path/Combine output-root (get % "path")))
                    (get manifest "outputs")))
        (is (= (str (artifact/manifest-json manifest) "\n")
               (System.IO.File/ReadAllText
                manifest-path (System.Text.UTF8Encoding. false true)))))
      (finally
        (System.IO.Directory/Delete root true)))))

(deftest build-rejects-a-require-outside-the-declared-source-closure
  (let [root (temp-directory)
        source-root (System.IO.Path/Combine root "src")
        output-root (System.IO.Path/Combine root "artifact")
        plan-path (System.IO.Path/Combine root "runtime-artifact.edn")
        escape-plan {:schema artifact/plan-schema
                     :entry 'escape.main
                     :namespaces ['escape.main]}]
    (try
      ;; pnix.clr-meta.bootstrap is already loaded in this parent test process.
      ;; Only a fresh child with a replaced load path can prove it is absent
      ;; from the declared artifact closure.
      (write-text! (System.IO.Path/Combine source-root "escape" "main.clj")
                   (str "(ns escape.main\n"
                        "  (:require [pnix.clr-meta.bootstrap]))\n"
                        "(defn -main [& _] 42)\n"))
      (write-text! plan-path (str (pr-str escape-plan) "\n"))
      (is (= :aot-child-failed
             (failure-class
              #(artifact/build! plan-path output-root source-root))))
      (is (false? (System.IO.Directory/Exists output-root)))
      (is (empty?
           (System.IO.Directory/GetDirectories
            root "artifact.building.*" System.IO.SearchOption/TopDirectoryOnly)))
      (finally
        (System.IO.Directory/Delete root true)))))

(defn- write-single-namespace-fixture!
  [source-root plan-path]
  (let [plan {:schema artifact/plan-schema
              :entry 'fixture.main
              :namespaces ['fixture.main]}]
    (write-text! (System.IO.Path/Combine source-root "fixture" "main.clj")
                 "(ns fixture.main)\n(defn -main [& _] 42)\n")
    (write-text! plan-path (str (pr-str plan) "\n"))))

(deftest build-refuses-every-destructive-path-overlap-before-publication
  (testing "output below source root"
    (let [root (temp-directory)
          source-root (System.IO.Path/Combine root "src")
          plan-path (System.IO.Path/Combine root "plan.edn")
          output-root (System.IO.Path/Combine source-root "artifact")]
      (try
        (write-single-namespace-fixture! source-root plan-path)
        (is (= :path-overlap
               (failure-class
                #(artifact/build! plan-path output-root source-root))))
        (is (System.IO.File/Exists
             (System.IO.Path/Combine source-root "fixture" "main.clj")))
        (is (false? (System.IO.Directory/Exists output-root)))
        (finally
          (System.IO.Directory/Delete root true)))))
  (testing "source root below an existing output"
    (let [root (temp-directory)
          output-root (System.IO.Path/Combine root "artifact")
          source-root (System.IO.Path/Combine output-root "src")
          plan-path (System.IO.Path/Combine root "plan.edn")]
      (try
        (write-single-namespace-fixture! source-root plan-path)
        (is (= :path-overlap
               (failure-class
                #(artifact/build! plan-path output-root source-root))))
        (is (System.IO.File/Exists plan-path))
        (is (System.IO.File/Exists
             (System.IO.Path/Combine source-root "fixture" "main.clj")))
        (finally
          (System.IO.Directory/Delete root true)))))
  (testing "plan below an existing output"
    (let [root (temp-directory)
          source-root (System.IO.Path/Combine root "src")
          output-root (System.IO.Path/Combine root "artifact")
          plan-path (System.IO.Path/Combine output-root "plan.edn")]
      (try
        (write-single-namespace-fixture! source-root plan-path)
        (is (= :path-overlap
               (failure-class
                #(artifact/build! plan-path output-root source-root))))
        (is (System.IO.File/Exists plan-path))
        (is (System.IO.File/Exists
             (System.IO.Path/Combine source-root "fixture" "main.clj")))
        (finally
          (System.IO.Directory/Delete root true)))))
  (testing "plan below source root is also rejected by pairwise disjointness"
    (let [root (temp-directory)
          source-root (System.IO.Path/Combine root "src")
          plan-path (System.IO.Path/Combine source-root "plan.edn")
          output-root (System.IO.Path/Combine root "artifact")]
      (try
        (write-single-namespace-fixture! source-root plan-path)
        (is (= :path-overlap
               (failure-class
                #(artifact/build! plan-path output-root source-root))))
        (is (System.IO.File/Exists plan-path))
        (is (false? (System.IO.Directory/Exists output-root)))
        (finally
          (System.IO.Directory/Delete root true))))))

(deftest plan-reader-rejects-empty-and-trailing-edn
  (let [root (temp-directory)
        plan-path (System.IO.Path/Combine root "plan.edn")]
    (try
      (write-text! plan-path "")
      (is (= :empty-plan
             (failure-class #(artifact/read-plan plan-path))))
      (write-text! plan-path "{} {}")
      (is (= :trailing-plan-data
             (failure-class #(artifact/read-plan plan-path))))
      (finally
        (System.IO.Directory/Delete root true)))))
