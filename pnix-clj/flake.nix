{
  description = "pnix-clj + clj-meta: a Clojure(clj-meta) <-> pnix meta-circular toolkit (Clojure/JVM-hosted pnix runtime + clj-meta host-proof lane: pure eval, Futamura specialize, 4-substrate self-hosting tower, content-addressed cache, capability index).";
  # Scope is locked by ./pnix-clj/SCOPE_LOCK.md. pnix-clj core is the Clojure-hosted pnix meta-circular proof lane; NL/MSV/Hangul/gate-graph/multi-language emit/coding-agent lanes are out of scope.

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
    in
    {
      # ---- packages ------------------------------------------------------
      # The pnix-clj CLI runs the SOURCE tree from the repo root: `clojure`
      # resolves deps.edn (`../clj-meta` local/root + a few maven jars) on first
      # use. The JDK + clojure toolchain is what the flake pins/installs; the
      # runtime/host proof are the in-repo `./pnix-clj` and `./clj-meta` trees.
      packages = forAll (pkgs:
        let
          jdk = pkgs.temurin-bin-21 or pkgs.jdk21;
          # clojure carries its own JDK dep, but we pin one for a stable JAVA_HOME.
          toolchain = [ pkgs.clojure jdk pkgs.git pkgs.rlwrap ];
          pnix-deps = pkgs.callPackage ./pnix-clj/deps.nix { };
          clojure-tools-jar =
            "${pkgs.clojure}/libexec/clojure-tools-${pkgs.clojure.version}.jar";
          pnix-runtime-classpath = pkgs.lib.concatStringsSep ":" ([
            clojure-tools-jar
            "${self}/pnix-clj/src"
            "${self}/pnix-clj/resources"
            "${self}/clj-meta/src"
          ] ++ pnix-deps.makePaths { });

          pnix-clj = pkgs.writeShellApplication {
            name = "pnix-clj";
            runtimeInputs = toolchain;
            text = ''
              export JAVA_HOME="${jdk}"
              if [ ! -e "$PWD/pnix-clj/deps.edn" ] || [ ! -e "$PWD/clj-meta/deps.edn" ]; then
                echo "pnix-clj: run from the repo root (needs ./pnix-clj and ./clj-meta)." >&2
                exit 2
              fi
              # thin wrapper: forward args to `clojure` inside ./pnix-clj, e.g.
              #   pnix-clj -M:tower        pnix-clj -M:test        pnix-clj -M:capabilities-check
              cd "$PWD/pnix-clj" && exec clojure "$@"
            '';
          };

          # Clojure-host mode for an arbitrary caller project.  Unlike the
          # historical source-tree-only app below, this command deliberately
          # stays in the caller's cwd so Clojure CLI keeps the project's
          # deps.edn, aliases, paths, and ~/.clojure configuration.  pnix-clj
          # enters as one additional local/root dependency; it does not replace
          # or reinterpret the project's dependency graph.
          pnix-clj-clj-command = pkgs.writeShellApplication {
            name = "pnix-clj-clj";
            runtimeInputs = toolchain;
            text = ''
              export JAVA_HOME="${jdk}"
              pnix_deps='{:deps {pnix/pnix-clj {:local/root "${self}/pnix-clj"}}}'

              # An explicit caller -Sdeps owns that invocation.  Passing two
              # -Sdeps values would make ordinary Clojure CLI behavior
              # ambiguous, so preserve the caller's value without injection.
              inject_pnix=1
              for arg in "$@"; do
                case "$arg" in
                  -Sdeps|-Sdeps=*) inject_pnix=0 ;;
                esac
              done

              if [ "$inject_pnix" -eq 1 ]; then
                exec ${pkgs.clojure}/bin/clojure -Sdeps "$pnix_deps" "$@"
              else
                exec ${pkgs.clojure}/bin/clojure "$@"
              fi
            '';
          };

          # Install both names.  Kimchi replaces its default `clojure` command
          # with this package while retaining `pnix-clj-clj` as the explicit
          # lane name.
          pnix-clj-clj = pkgs.symlinkJoin {
            name = "pnix-clj-clj";
            paths = [ pnix-clj-clj-command ];
            postBuild = ''ln -s pnix-clj-clj "$out/bin/clojure"'';
            meta.mainProgram = "pnix-clj-clj";
          };

          # Hermetic product runner. Unlike pnix-clj-clj's caller-project mode,
          # this command never resolves Maven dependencies at runtime.
          pnix-clj-runtime = pkgs.writeShellApplication {
            name = "pnix-clj-runtime";
            runtimeInputs = [ jdk ];
            text = ''
              export JAVA_HOME="${jdk}"
              exec ${jdk}/bin/java \
                -Duser.home="''${HOME:-/tmp}" \
                -cp "${pnix-runtime-classpath}" \
                clojure.main "$@"
            '';
          };
        in
        {
          default = pnix-clj;
          inherit pnix-clj pnix-clj-clj pnix-clj-runtime;
        });

      # ---- apps ----------------------------------------------------------
      # All run from the repo root against the SOURCE tree (needs ./pnix-clj and
      # ./clj-meta). `nix develop` for an interactive shell.
      #
      # THREE INDEPENDENT RUNNERS (Clojure-hosted Clojure extensions; each can
      # later carry its own deps.edn and be started as a network/nREPL server):
      #   nix run .#pnix-clj-pnix -- -e '1 + 2'   : the pnix language lane
      #   nix run .#pnix-clj-pnix                  : eval ./default.px, else REPL
      #   nix run .#pnix-clj-clj                   : pnix-clj's Clojure host REPL
      #   nix run .#clj-meta -- -e '(+ 1 2)'       : clj-meta meta-circular Clojure
      # server seams:
      #   nix run .#pnix-clj-pnix-server           : pnix network REPL (port 7888)
      #   nix run .#pnix-clj-nrepl                 : Clojure-host nREPL (port 7888)
      #   nix run .#pnix-clj-pnix-nrepl            : pnix-language nREPL, eval via pnix (7890)
      #   nix run .#clj-meta-nrepl                 : clj-meta nREPL, eval via clj-meta backend (7889)
      # gates / reports:
      #   nix run .#gate               : full test gate (164 tests / 3250 assertions)
      #   nix run .#tower              : self-hosting 4-substrate tower report
      #   nix run .#capabilities-check : machine-generated capability index drift gate
      #   nix run .#clj-meta-gate      : clj-meta's own meta-circular self-host gate
      #   nix run .#examples           : run every examples/*/pnix_clj_way.clj
      #   nix run .#safe-eval -- '1+2' : the pure sandbox on an expression
      apps = forAll (pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          jdk = pkgs.temurin-bin-21 or pkgs.jdk21;
          runtimeInputs = [ pkgs.clojure jdk pkgs.git pkgs.rlwrap ];
          # run a `clojure` invocation inside ./pnix-clj from the repo root
          cljRunner = { name, argv, help }: pkgs.writeShellApplication {
            inherit name runtimeInputs;
            text = ''
              export JAVA_HOME="${jdk}"
              if [ ! -e "$PWD/pnix-clj/deps.edn" ] || [ ! -e "$PWD/clj-meta/deps.edn" ]; then
                echo "${name}: ${help}" >&2
                echo "  run this from the repo root (needs ./pnix-clj and ./clj-meta)." >&2
                exit 2
              fi
              cd "$PWD/pnix-clj"
              exec clojure ${argv} "$@"
            '';
          };
          # run a `clojure` invocation inside ./clj-meta -- clj-meta is an
          # INDEPENDENT meta-circular Clojure compiler/evaluator, so it is
          # runnable on its own (mode a: clj-meta AS the runner), distinct from
          # pnix-clj using it as a :local/root dependency (mode b, `nix run
          # .#gate`). Both modes are first-class; the pnix tower's clj-meta-host
          # lane already runs every pnix program through clj-meta's bytecode
          # compiler + cross-checks it against the direct evaluator.
          cljMetaRunner = { name, argv, help }: pkgs.writeShellApplication {
            inherit name runtimeInputs;
            text = ''
              export JAVA_HOME="${jdk}"
              if [ ! -e "$PWD/clj-meta/deps.edn" ]; then
                echo "${name}: ${help}" >&2
                echo "  run this from the repo root (needs ./clj-meta)." >&2
                exit 2
              fi
              cd "$PWD/clj-meta"
              exec clojure ${argv} "$@"
            '';
          };
          examplesApp = pkgs.writeShellApplication {
            name = "pnix-clj-examples";
            inherit runtimeInputs;
            text = ''
              export JAVA_HOME="${jdk}"
              if [ ! -e "$PWD/pnix-clj/deps.edn" ]; then
                echo "run from the repo root (needs ./pnix-clj)." >&2
                exit 2
              fi
              cd "$PWD/pnix-clj"
              status=0
              for f in examples/*/pnix_clj_way.clj; do
                echo "===== $f ====="
                clojure -M "$f" || status=1
                echo
              done
              exit "$status"
            '';
          };
          gate = cljRunner { name = "pnix-clj-gate"; argv = "-M:test";
                             help = "full test gate (pnix runtime + clj-meta host proof)"; };
          tower = cljRunner { name = "pnix-clj-tower"; argv = "-M:tower";
                              help = "self-hosting 4-substrate tower report"; };
          caps = cljRunner { name = "pnix-clj-capabilities-check"; argv = "-M:capabilities-check";
                             help = "capability index drift gate"; };
          safeEval = cljRunner { name = "pnix-clj-safe-eval"; argv = "-M:safe-eval";
                                 help = "pure resource-bounded sandbox"; };
          specialize = cljRunner { name = "pnix-clj-specialize"; argv = "-M:specialize";
                                   help = "Futamura specialize report"; };
          # ── THREE INDEPENDENT RUNNERS (like pnix-hy: pnix / hy / hy-meta) ──
          # pnix-clj is an independent runner for TWO lanes (pnix + its Clojure
          # host), and clj-meta is an independent meta-circular Clojure runner.
          #   pnix-clj-pnix : the pnix language          (≈ repl-pnix-hy-pnix)
          #   pnix-clj-clj  : pnix-clj's Clojure host     (≈ repl-pnix-hy-hy)
          #   clj-meta      : clj-meta meta-circular clj  (≈ repl-hy-meta-hy)
          pnixPnix = cljRunner { name = "pnix-clj-pnix"; argv = "-M:repl-pnix";
                                 help = "pnix language runner: <file.px> | -e EXPR | ./default.px | interactive | --server [port]"; };
          # Historical source-tree-only host mode (retired):
          # pnixClj = cljRunner { name = "pnix-clj-clj";
          #   argv = "-M:test -e nil -r";
          #   help = "pnix-clj Clojure host REPL"; };
          #
          # The package-owned mode now preserves the caller's deps.edn and can
          # therefore serve both pnix-clj itself and external Clojure projects.
          pnixClj = self.packages.${system}.pnix-clj-clj;
          cljMeta = cljMetaRunner { name = "clj-meta"; argv = "-M:repl";
                                    help = "clj-meta meta-circular Clojure REPL/runner: <file.clj> | -e FORM | interactive"; };
          # network/nREPL server seams (editors + tooling; own deps.edn later)
          pnixPnixServer = cljRunner { name = "pnix-clj-pnix-server"; argv = "-M:repl-pnix-server";
                                       help = "pnix network REPL (socket server evaluating pnix, port 7888)"; };
          pnixCljNrepl = cljRunner { name = "pnix-clj-nrepl"; argv = "-M:nrepl";
                                     help = "pnix-clj Clojure-host nREPL server (port 7888)"; };
          pnixPnixNrepl = cljRunner { name = "pnix-clj-pnix-nrepl"; argv = "-M:nrepl-pnix";
                                      help = "pnix-language nREPL server -- eval routed through pnix (port 7890)"; };
          cljMetaNrepl = cljMetaRunner { name = "clj-meta-nrepl"; argv = "-M:nrepl";
                                         help = "clj-meta nREPL server -- eval routed through clj-meta's OWN compiler (port 7889)"; };
          # clj-meta self-checks (the independent compiler proving itself)
          cljMetaGate = cljMetaRunner { name = "clj-meta-gate"; argv = "-M:gate";
                                        help = "clj-meta's own meta-circular self-host gate"; };
          cljMetaKernel = cljMetaRunner { name = "clj-meta-kernel"; argv = "-M:kernel-smoke";
                                          help = "clj-meta kernel bytecode-compiler smoke"; };
        in
        {
          # default runner = the pnix language lane
          default = { type = "app"; program = pkgs.lib.getExe pnixPnix; };
          # the three independent runners
          pnix-clj-pnix = { type = "app"; program = pkgs.lib.getExe pnixPnix; };
          pnix-clj-clj = { type = "app"; program = "${pnixClj}/bin/pnix-clj-clj"; };
          clj-meta = { type = "app"; program = pkgs.lib.getExe cljMeta; };
          # server seams
          pnix-clj-pnix-server = { type = "app"; program = pkgs.lib.getExe pnixPnixServer; };
          pnix-clj-nrepl = { type = "app"; program = pkgs.lib.getExe pnixCljNrepl; };
          pnix-clj-pnix-nrepl = { type = "app"; program = pkgs.lib.getExe pnixPnixNrepl; };
          clj-meta-nrepl = { type = "app"; program = pkgs.lib.getExe cljMetaNrepl; };
          # gates / reports
          gate = { type = "app"; program = pkgs.lib.getExe gate; };
          tower = { type = "app"; program = pkgs.lib.getExe tower; };
          capabilities-check = { type = "app"; program = pkgs.lib.getExe caps; };
          safe-eval = { type = "app"; program = pkgs.lib.getExe safeEval; };
          specialize = { type = "app"; program = pkgs.lib.getExe specialize; };
          examples = { type = "app"; program = pkgs.lib.getExe examplesApp; };
          clj-meta-gate = { type = "app"; program = pkgs.lib.getExe cljMetaGate; };
          clj-meta-kernel = { type = "app"; program = pkgs.lib.getExe cljMetaKernel; };
        });

      # ---- devShell ------------------------------------------------------
      # `nix develop` then, from the repo root:
      #   cd pnix-clj && clojure -M:test        # full gate
      #   cd pnix-clj && clojure -M:tower       # tower report
      #   clojure -M examples/11-self-hosting-convergence/pnix_clj_way.clj
      devShells = forAll (pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          jdk = pkgs.temurin-bin-21 or pkgs.jdk21;
          pnixCljClj = self.packages.${system}.pnix-clj-clj;
        in
        {
          default = pkgs.mkShell {
            packages = [ pnixCljClj jdk pkgs.git pkgs.rlwrap ];
            shellHook = ''
              export JAVA_HOME="${jdk}"
              echo "pnix-clj devShell -- Clojure(clj-meta) <-> pnix meta-circular toolkit"
              echo "  JDK: ${jdk}"
              echo "  from the repo root:"
              echo "    cd pnix-clj && clojure -M:test               # full gate (164 tests)"
              echo "    cd pnix-clj && clojure -M:tower              # 4-substrate tower report"
              echo "    cd pnix-clj && clojure -M:capabilities-check # capability drift gate"
              echo "    cd pnix-clj && clojure -M examples/01-pure-sandbox/pnix_clj_way.clj"
            '';
          };
        });
    };
}
