{
  description = "PNIX ClojureScript and JavaScript host";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.clj-nix = {
    url = "github:jlesquembre/clj-nix";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  
  outputs = { self, nixpkgs, clj-nix }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs {
		  	   inherit system;
			   overlays = [ clj-nix.overlays.default ];
		  };

          # deps.edn들이 참조하는 의존성(로컬 clojurescript가 물고 오는
          # org.clojure/clojure 등 진짜 Maven 좌표들)을 오프라인으로 제공하는 캐시
          depsCache = pkgs.mk-deps-cache { lockfile = ./deps-lock.json; };

          cljsMeta = pkgs.stdenvNoCC.mkDerivation {
            pname = "cljs-meta";
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = [ pkgs.makeWrapper pkgs.nodejs pkgs.clojure pkgs.unzip ];
            dontBuild = false;
buildPhase = ''
  runHook preBuild

export HOME="$TMPDIR/home"
              mkdir -p "$HOME"

              export XDG_CACHE_HOME="$TMPDIR/cache"
              mkdir -p "$XDG_CACHE_HOME"

              # 오프라인 클래스패스 리졸브: Maven/git 리졸브 지점만
              # 미리 채워둔 read-only 캐시로 돌린다.
			  export JAVA_TOOL_OPTIONS="-Duser.home=${depsCache}"
              # export JAVA_TOOL_OPTIONS="-Duser.home=$HOME -Dmaven.repo.local=${depsCache}/.m2/repository"
              export CLJ_CONFIG="${depsCache}/.clojure"
              export GITLIBS="${depsCache}/.gitlibs"
              export CLJ_CACHE="$TMPDIR/cp_cache"
              mkdir -p "$CLJ_CACHE"

  # The fixed-point proof compiles the ClojureScript compiler's own sources
  # (cljs/js.cljs and friends) with itself, so it needs those sources on disk.
  # They ship inside the org.clojure/clojurescript jar from the Maven cache;
  # unpack them into the source layout the proof expects. pnix-cljs therefore
  # vendors no compiler tree of its own.
  cljs_jar="$(find -L ${depsCache} -name 'clojurescript-*.jar' -not -name '*-sources.jar' | head -1)"
  if [ -z "$cljs_jar" ]; then
    echo "cljs-meta: no clojurescript jar in the offline deps cache" >&2
    exit 1
  fi
  mkdir -p clojurescript-r1.12.145/src/main/cljs
  (cd clojurescript-r1.12.145/src/main/cljs && unzip -qo "$cljs_jar" -x 'META-INF/*')
  ln -sfn cljs clojurescript-r1.12.145/src/main/clojure

  # cljs-meta
  cd cljs-meta
  rm -rf target dist
  mkdir -p dist

  clojure -Srepro -M -m cljs.main \
    -co build-cli.edn \
    -c pnix.cljs-meta.main

  clojure -Srepro -M -m cljs.main \
    -co build-module.edn \
    -c pnix.cljs-meta.module

  clojure -Srepro -M -m cljs.main \
    -co build-stage-runtime.edn \
    -c pnix.cljs-meta.stage-runtime

  cd ..

  if node bin/meta-artifact-identity.js --check >/dev/null 2>&1; then
      node bin/meta-artifact-identity.js --check
      echo "reusing content-identical cljs-meta fixed point"
  else
      node bin/build-fixed-point.js "$PWD"
      node bin/meta-artifact-identity.js --write
  fi

  # pnix-cljs
  cd pnix-cljs
  rm -rf target dist
  mkdir -p dist

  clojure -Srepro -M -m cljs.main \
    -co build-cli.edn \
    -c pnix-cljs.main

  clojure -Srepro -M -m cljs.main \
    -co build-module.edn \
    -c pnix-cljs.module

  clojure -Srepro -M -m cljs.main \
    -co build-test.edn \
    -c pnix-cljs.self-test

  cd ..

  node bin/artifact-identity.js --write

  runHook postBuild
'';

            installPhase = ''
  node bin/artifact-identity.js --check
  test -f cljs-meta/dist/cljs-meta.js
  test -f cljs-meta/dist/cljs-meta-module.js
  test -f cljs-meta/dist/cljs-meta-stage-runtime.js
  test -f cljs-meta/dist/fixed-point/cljs-meta-fixed.js
  test -f cljs-meta/dist/fixed-point/cljs-meta-fixed-cli.js
  test -f cljs-meta/dist/fixed-point/receipt.json
  
              mkdir -p $out/bin $out/share/cljs-meta
              cp cljs-meta/dist/cljs-meta.js \
                 cljs-meta/dist/cljs-meta-module.js \
                 cljs-meta/dist/cljs-meta-stage-runtime.js \
                 $out/share/cljs-meta/
              cp cljs-meta/dist/fixed-point/cljs-meta-fixed.js \
                 cljs-meta/dist/fixed-point/cljs-meta-fixed-cli.js \
                 cljs-meta/dist/fixed-point/receipt.json \
                 $out/share/cljs-meta/
              makeWrapper ${pkgs.nodejs}/bin/node $out/bin/cljs-meta-bootstrap \
                --add-flags $out/share/cljs-meta/cljs-meta.js
              makeWrapper ${pkgs.nodejs}/bin/node $out/bin/cljs-meta \
                --add-flags $out/share/cljs-meta/cljs-meta-fixed-cli.js
              makeWrapper ${pkgs.nodejs}/bin/node $out/bin/pnix-cljs-cljs \
                --add-flags $out/share/cljs-meta/cljs-meta-fixed-cli.js
            '';
          };

          pnixCljs = pkgs.stdenvNoCC.mkDerivation {
            pname = "pnix-cljs";
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = [ pkgs.makeWrapper pkgs.nodejs pkgs.clojure ];
			buildInputs = [ cljsMeta ]; # <- 이미 빌드된 cljs-meta
            dontBuild = false;
# buildPhase = ''
#   runHook preBuild

# export HOME="$TMPDIR/home"
#               mkdir -p "$HOME"

#               export XDG_CACHE_HOME="$TMPDIR/cache"
#               mkdir -p "$XDG_CACHE_HOME"

#               # 오프라인 클래스패스 리졸브: Maven/git 리졸브 지점만
#               # 미리 채워둔 read-only 캐시로 돌린다.
# 			  export JAVA_TOOL_OPTIONS="-Duser.home=${depsCache}"
# 			  # export JAVA_TOOL_OPTIONS="-Duser.home=$HOME -Dmaven.repo.local=${depsCache}/.m2/repository"
#               export CLJ_CONFIG="${depsCache}/.clojure"
#               export GITLIBS="${depsCache}/.gitlibs"
#               export CLJ_CACHE="$TMPDIR/cp_cache"
#               mkdir -p "$CLJ_CACHE"

#   # cljs-meta
#   cd cljs-meta
#   rm -rf target dist
#   mkdir -p dist

#   clojure -Srepro -M -m cljs.main \
#     -co build-cli.edn \
#     -c pnix.cljs-meta.main

#   clojure -Srepro -M -m cljs.main \
#     -co build-module.edn \
#     -c pnix.cljs-meta.module

#   clojure -Srepro -M -m cljs.main \
#     -co build-stage-runtime.edn \
#     -c pnix.cljs-meta.stage-runtime

#   cd ..

#   if node bin/meta-artifact-identity.js --check >/dev/null 2>&1; then
#       node bin/meta-artifact-identity.js --check
#       echo "reusing content-identical cljs-meta fixed point"
#   else
#       node bin/build-fixed-point.js "$PWD"
#       node bin/meta-artifact-identity.js --write
#   fi

#   # pnix-cljs
#   cd pnix-cljs
#   rm -rf target dist
#   mkdir -p dist

#   clojure -Srepro -M -m cljs.main \
#     -co build-cli.edn \
#     -c pnix-cljs.main

#   clojure -Srepro -M -m cljs.main \
#     -co build-module.edn \
#     -c pnix-cljs.module

#   clojure -Srepro -M -m cljs.main \
#     -co build-test.edn \
#     -c pnix-cljs.self-test

#   cd ..

#   node bin/artifact-identity.js --write

#   runHook postBuild
# '';

buildPhase = ''
    runHook preBuild

    export HOME="$TMPDIR/home"
    mkdir -p "$HOME"
    export XDG_CACHE_HOME="$TMPDIR/cache"
    mkdir -p "$XDG_CACHE_HOME"
    export JAVA_TOOL_OPTIONS="-Duser.home=${depsCache}"
    export CLJ_CONFIG="${depsCache}/.clojure"
    export GITLIBS="${depsCache}/.gitlibs"
    export CLJ_CACHE="$TMPDIR/cp_cache"
    mkdir -p "$CLJ_CACHE"

    # cljs-meta를 다시 빌드하는 대신, 이미 만들어진 산출물을 그대로 배치
    mkdir -p cljs-meta/dist/fixed-point
    cp ${cljsMeta}/share/cljs-meta/cljs-meta.js \
       ${cljsMeta}/share/cljs-meta/cljs-meta-module.js \
       ${cljsMeta}/share/cljs-meta/cljs-meta-stage-runtime.js \
       cljs-meta/dist/
    cp ${cljsMeta}/share/cljs-meta/cljs-meta-fixed.js \
       ${cljsMeta}/share/cljs-meta/cljs-meta-fixed-cli.js \
       ${cljsMeta}/share/cljs-meta/receipt.json \
       cljs-meta/dist/fixed-point/

    node bin/meta-artifact-identity.js --write  # 필요하다면

    # pnix-cljs
    cd pnix-cljs
    rm -rf target dist
    mkdir -p dist

    clojure -Srepro -M -m cljs.main -co build-cli.edn -c pnix-cljs.main
    clojure -Srepro -M -m cljs.main -co build-module.edn -c pnix-cljs.module
    clojure -Srepro -M -m cljs.main -co build-test.edn -c pnix-cljs.self-test

    cd ..
    node bin/artifact-identity.js --write

    runHook postBuild
  '';
  
	installPhase = ''
              node bin/artifact-identity.js --check
              test -f pnix-cljs/dist/pnix-cljs.js
              test -f pnix-cljs/dist/pnix-cljs-module.js
              mkdir -p $out/bin $out/share/pnix-cljs
              cp pnix-cljs/dist/pnix-cljs.js \
                 pnix-cljs/dist/pnix-cljs-module.js $out/share/pnix-cljs/
              cp pnix-cljs/package.json $out/share/pnix-cljs/package.json
              makeWrapper ${pkgs.nodejs}/bin/node $out/bin/pnix-cljs \
                --add-flags $out/share/pnix-cljs/pnix-cljs.js
            '';
          };
        in {
          inherit pnixCljs cljsMeta;
          "pnix-cljs" = pnixCljs;
          "cljs-meta" = cljsMeta;
          "pnix-cljs-cljs" = cljsMeta;
          default = pkgs.symlinkJoin {
            name = "pnix-cljs-toolkit";
            paths = [ pnixCljs cljsMeta ];
          };
        });

      # App names follow the scheme shared by every pnix host:
      #   pnix-<host>          the runtime CLI
      #   pnix-<host>-<lang>   an interactive REPL in <lang>
      #   <host>-meta          the host-language mechanism CLI
      #   gate                 this host's full gate
      apps = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ clj-nix.overlays.default ];
          };
          pnixCljsRepl = pkgs.writeShellApplication {
            name = "pnix-cljs-pnix";
            text = ''
              exec ${self.packages.${system}.pnix-cljs}/bin/pnix-cljs --repl "$@"
            '';
          };
          # The gate reads the source tree (identity + artifact digests), so it
          # runs from the working directory rather than from the store.
          gateApp = pkgs.writeShellApplication {
            name = "pnix-cljs-gate";
            runtimeInputs = [ pkgs.nodejs pkgs.ripgrep ];
            text = ''
              if [ ! -x "$PWD/bin/pnix-cljs-gate" ]; then
                echo "pnix-cljs gate: run from the pnix-cljs source root" >&2
                exit 2
              fi
              # `bin/pnix-cljs-gate` reads compiled artifacts under dist/; build
              # them unless the caller already ran `--rebuild` or a manual build.
              if [ ! -f "$PWD/pnix-cljs/dist/pnix-cljs.js" ]; then
                exec "$PWD/bin/pnix-cljs-gate" --rebuild "$@"
              fi
              exec "$PWD/bin/pnix-cljs-gate" "$@"
            '';
          };
        in {
          pnix-cljs = {
            type = "app";
            program = "${self.packages.${system}.pnix-cljs}/bin/pnix-cljs";
          };
          pnix-cljs-pnix = { type = "app"; program = pkgs.lib.getExe pnixCljsRepl; };
          cljs-meta = {
            type = "app";
            program = "${self.packages.${system}.cljs-meta}/bin/cljs-meta";
          };
          pnix-cljs-cljs = {
            type = "app";
            program = "${self.packages.${system}.pnix-cljs-cljs}/bin/pnix-cljs-cljs";
          };
          gate = { type = "app"; program = pkgs.lib.getExe gateApp; };
          deps-lock = { type = "app"; program = "${clj-nix.packages."${system}".deps-lock}/bin/deps-lock"; };
          default = self.apps.${system}.pnix-cljs;
        });

      devShells = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; };
        in {
          default = pkgs.mkShell {
            packages = [ pkgs.clojure pkgs.nodejs pkgs.git pkgs.ripgrep ];
            shellHook = ''
              echo "pnix-cljs: ./bin/build-cljs"
            '';
          };
        });

      checks = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          package = self.packages.${system}.pnix-cljs;
        in {
          identity = pkgs.runCommand "pnix-cljs-identity" {
            nativeBuildInputs = [ pkgs.bash pkgs.ripgrep ];
            src = self;
          } ''
            cp -R $src source
            chmod -R u+w source
            cd source
            ./bin/pnix-cljs-identity-gate
            touch $out
          '';

          smoke = pkgs.runCommand "pnix-cljs-smoke" {
            nativeBuildInputs = [ package ];
          } ''
            pnix-cljs -e '20 + 22' > result.json
            grep -q '"outcome_kind":"done"' result.json
            grep -q '"value":42' result.json
            touch $out
          '';

          runtime = pkgs.runCommand "pnix-cljs-runtime-matrix" {
            nativeBuildInputs = [ package pkgs.nodejs ];
          } ''
            pnix-cljs -e 'let double = x: x * 2; in double 21' > lambda.json
            grep -q '"value":42' lambda.json

            pnix-cljs -e 'rec { answer = base + 2; base = 40; }.answer' > rec.json
            grep -q '"value":42' rec.json

            if pnix-cljs -e '1 / 0' > failed.json; then
              echo "division by zero unexpectedly succeeded" >&2
              exit 1
            fi
            grep -q '"class":"division-by-zero"' failed.json

            node - <<'NODE'
            const pnix = require("${package}/share/pnix-cljs/pnix-cljs-module.js");
            const result = pnix.evalSource("if true then 42 else missing");
            if (result.outcome_kind !== "done" || result.value !== 42n) process.exit(1);
            NODE
            touch $out
          '';

          meta = pkgs.runCommand "cljs-meta-runtime" {
            nativeBuildInputs = [ self.packages.${system}.cljs-meta pkgs.nodejs ];
          } ''
            cljs-meta -e '(+ 20 22)' > result.json
            grep -q '"outcome_kind":"done"' result.json
            grep -q '"value":42' result.json
            grep -q '"fixed_point": true' \
              ${self.packages.${system}.cljs-meta}/share/cljs-meta/receipt.json
            grep -q '"stage0_compiler_embedded": false' \
              ${self.packages.${system}.cljs-meta}/share/cljs-meta/receipt.json
            grep -q '"minimum_stage_count": 15' \
              ${self.packages.${system}.cljs-meta}/share/cljs-meta/receipt.json
            node - <<'NODE'
            const fixed = require("${self.packages.${system}.cljs-meta}/share/cljs-meta/cljs-meta-fixed.js");
            fixed.compile("(defn answer [] 42)").then((result) => {
              if (result.outcome_kind !== "done") process.exitCode = 1;
            });
            NODE
            touch $out
          '';
        });
    };
}
