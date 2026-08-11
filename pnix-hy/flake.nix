{
  description = "pnix-hy + hy-meta: a Hy(Python) <-> pnix meta-circular projection toolkit (pure resource-bounded pnix eval + Hy<->pnix projection facilities).";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  # Hy 1.3.1 from the official upstream repo (tag `1.3.1`), locked by flake.lock.
  inputs.hy-src = {
    url = "github:hylang/hy/1.3.1";
    flake = false;
  };

  outputs = { self, nixpkgs, hy-src }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
    in
    {
      # ---- packages ------------------------------------------------------
      packages = forAll (pkgs:
        let
          python = pkgs.python311;

          # Hy 1.3.1 from the OFFICIAL upstream repo (github:hylang/hy tag 1.3.1, pinned in
          # flake.lock) -- the exact version the 4-lane mirror + rust corpus are proven against,
          # rather than a possibly-different nixpkgs Hy.
          hy = python.pkgs.buildPythonPackage {
            pname = "hy";
            version = "1.3.1";
            src = hy-src;
            format = "setuptools";
            propagatedBuildInputs = [ python.pkgs.funcparserlib ];
            nativeBuildInputs = [ python.pkgs.setuptools python.pkgs.wheel ];
            doCheck = false; # upstream test suite is heavy + network-ish; not needed to install
            pythonImportsCheck = [ "hy" ];
            meta = {
              description = "Hy 1.3.1 (Lisp dialect embedded in Python), fetched from upstream for pnix-hy";
              license = pkgs.lib.licenses.mit;
            };
          };

          # The installable pnix-hy CLI. The pnix RUNTIME + safe_eval/purity/cache/IR/gate
          # are pure stdlib (no deps), so this alone gives a working `pnix-hy-project`.
          # The Hy<->pnix PROJECTION reports additionally need Hy + the repo tree at HY_ROOT
          # (run those from a source checkout / the devShell -- see below).
          pnix-hy = python.pkgs.buildPythonApplication {
            pname = "pnix-hy";
            version = "0.1.0";
            src = ./pnix-hy;
            format = "pyproject";
            nativeBuildInputs = [ python.pkgs.setuptools python.pkgs.wheel ];
            doCheck = false;
            pythonImportsCheck = [ "pnix_hy" ];
            meta = {
              description = "Meta-circular projection toolkit between Hy/Python and pnix";
              mainProgram = "pnix-hy-project";
            };
          };

          # A Python that carries Hy 1.3.1 -- used as the projection "proof Python"
          # (PNIX_HY_PYTHON) and to run the source tree's --check / --gate.
          # pytest is required transitively: tests/resources/__init__.py (part of
          # the native Hy corpus pulled in below) imports it at module load time,
          # even though hy-meta never invokes pytest itself.
          proofPython = python.withPackages (ps: [ hy ps.pytest ]);
        in
        {
          default = pnix-hy;
          inherit pnix-hy hy proofPython;
        });

      # ---- apps ----------------------------------------------------------
      # `nix run .#pnix-hy-project -- ...`  : the installed CLI (pure facilities work anywhere).
      # `nix run .#check` / `.#gate`        : the FULL projection self-checks -- MUST be run from
      #                                       the repo root (needs ./hy-meta and ./pnix-hy at
      #                                       HY_ROOT); they use the source shim + set PNIX_HY_PYTHON.
      # `nix run .#hy-meta -- <args>`       : the hy-meta host proof lane (bootstrap.py) from the repo.
      apps = forAll (pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system};
          # run a source-tree entrypoint from $PWD with Hy available as the proof Python
          srcRunner = { name, argv }: pkgs.writeShellApplication {
            inherit name;
            runtimeInputs = [ p.proofPython ];
            text = ''
              py="${p.proofPython}/bin/python"
              export PNIX_HY_PYTHON="$py"
              # Hy itself comes from the pinned flake input (proofPython), so only
              # this repo's own trees are required at HY_ROOT.
              if [ ! -e "$PWD/pnix-hy/bin/pnix-hy" ] || [ ! -e "$PWD/hy-meta" ]; then
                echo "pnix-hy: run this from the repo root (needs ./pnix-hy and ./hy-meta at HY_ROOT)." >&2
                exit 2
              fi
              exec "$py" ${argv} "$@"
            '';
          };
          checkApp = srcRunner { name = "pnix-hy-check"; argv = ''"$PWD/pnix-hy/bin/pnix-hy" --check''; };
          gateApp = srcRunner { name = "pnix-hy-gate"; argv = ''"$PWD/pnix-hy/bin/pnix-hy" --gate''; };
          hyMetaApp = srcRunner { name = "hy-meta"; argv = ''"$PWD/hy-meta/bootstrap.py"''; };
          # the 5 context-retaining REPL modes (proposal 0008), all warm + from the repo root
          replPnix = srcRunner { name = "pnix-hy-pnix"; argv = ''"$PWD/pnix-hy/bin/pnix-hy" --repl pnix''; };
          replHy = srcRunner { name = "pnix-hy-hy"; argv = ''"$PWD/pnix-hy/bin/pnix-hy" --repl hy''; };
          replPy = srcRunner { name = "pnix-hy-python"; argv = ''"$PWD/pnix-hy/bin/pnix-hy" --repl python''; };
          replMetaHy = srcRunner { name = "hy-meta-hy"; argv = ''-m hy''; };
          replMetaPy = srcRunner { name = "hy-meta-python"; argv = ''-i -c "import sys; sys.path[:0]=['hy-meta','.']; print('hy-meta python REPL: sys.path has ./hy-meta and repo root -> import bootstrap / import hy')"''; };
        in
        {
          default = { type = "app"; program = pkgs.lib.getExe p.pnix-hy; };
          pnix-hy = { type = "app"; program = pkgs.lib.getExe p.pnix-hy; };
          check = { type = "app"; program = pkgs.lib.getExe checkApp; };
          gate = { type = "app"; program = pkgs.lib.getExe gateApp; };
          hy-meta = { type = "app"; program = pkgs.lib.getExe hyMetaApp; };
          pnix-hy-pnix = { type = "app"; program = pkgs.lib.getExe replPnix; };
          pnix-hy-hy = { type = "app"; program = pkgs.lib.getExe replHy; };
          pnix-hy-python = { type = "app"; program = pkgs.lib.getExe replPy; };
          hy-meta-hy = { type = "app"; program = pkgs.lib.getExe replMetaHy; };
          hy-meta-python = { type = "app"; program = pkgs.lib.getExe replMetaPy; };
        });

      # ---- devShell ------------------------------------------------------
      # `nix develop` then, from the repo root:
      #   python pnix-hy/bin/pnix-hy --check   (54 toolkit self-checks)
      #   python pnix-hy/bin/pnix-hy --gate    (sacred lanes + toolkit)
      devShells = forAll (pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system};
          # `pnix-hy-project` on PATH, running the SOURCE tree (so projection sees HY_ROOT) with
          # the flake's Hy 1.3.1 as the proof Python.
          pnixHyProjectSrc = pkgs.writeShellApplication {
            name = "pnix-hy-project";
            runtimeInputs = [ p.proofPython ];
            text = ''
              export PNIX_HY_PYTHON="${p.proofPython}/bin/python"
              if [ ! -e "$PWD/pnix-hy/bin/pnix-hy" ]; then
                echo "pnix-hy-project: run from the repo root (needs ./pnix-hy and ./hy-meta)." >&2
                exit 2
              fi
              exec "${p.proofPython}/bin/python" "$PWD/pnix-hy/bin/pnix-hy" "$@"
            '';
          };
        in
        {
          default = pkgs.mkShell {
            packages = [ p.proofPython pnixHyProjectSrc pkgs.git ];
            shellHook = ''
              export PNIX_HY_PYTHON="${p.proofPython}/bin/python"
              # hy-meta's native-Hy-corpus checks (diverse-double-compile-check,
              # parity-ledger-check, native-subset-test) need tests/ materialized
              # from the pinned hy-src input; it is gitignored, not committed.
              # Idempotent and cheap (528K), so just refresh it every shell entry.
              if [ -e "$PWD/hy-meta/bootstrap.py" ]; then
                rm -rf "$PWD/tests"
                cp -R "${hy-src}/tests" "$PWD/tests"
                chmod -R u+w "$PWD/tests"
              fi
              echo "pnix-hy devShell -- Hy 1.3.1 at PNIX_HY_PYTHON; python/hy/pnix-hy-project on PATH"
              echo "  from the repo root:"
              echo "    pnix-hy-project --check              # 54 toolkit self-checks"
              echo "    pnix-hy-project --gate               # sacred lanes + toolkit"
              echo "    pnix-hy-project --safe-eval '1 + 2'  # any facility"
              echo "    python hy-meta/bootstrap.py <args>   # host proof lane"
            '';
          };
        });
    };
}
