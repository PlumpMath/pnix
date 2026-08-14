(ns smoke
  "다중 모듈 host-main 데모: eval-source-with-imports 가 형제 .px 를
  메모리 모듈 맵으로 로드한다 (우리가 넘긴 것 밖의 FS import 해석기는 없음).

  이 디렉터리에서:
    clojure -M -m smoke
  기대: 3"
  (:require [clojure.java.io :as io]
            [pnix-clj.core :as c]))

(defn- slurp-px
  [name]
  (slurp (io/file name)))

(defn -main
  [& _]
  (let [modules {"./lib.px" (slurp-px "lib.px")}
        result (c/eval-source-with-imports (slurp-px "main.px") modules)]
    (when-not (= :ok (:status result))
      (binding [*out* *err*]
        (println "eval-source-with-imports 실패:" (pr-str result)))
      (System/exit 1))
    (println (:value result))))
