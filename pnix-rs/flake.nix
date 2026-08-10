{
  description = "rs-meta + pnix-rs: a Rust <-> pnix meta-circular toolkit. rs-meta is a standalone Rust-in-Rust meta-circular compiler/evaluator (stage15-N); pnix-rs is the rs-meta-backed pnix runtime front-end that projects Rust <-> px. Zero crates.io dependencies (std only); rustc is invoked as a toolchain for the native tier.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
    in
    {
      # ---- packages ------------------------------------------------------
      packages = forAll (pkgs:
        let
          # The native tier (Evcxr-style) shells out to `rustc` and links with a
          # C toolchain. Both binaries need them on PATH at RUN time.
          nativeTier = [ pkgs.rustc pkgs.stdenv.cc ];

          # rs-meta: standalone Rust meta-circular compiler/evaluator. Zero
          # crates.io deps, so the vendored dep set is empty; Cargo.lock is
          # trivial. `bootstrap` is wrapped so the native tier finds rustc/cc.
          rs-meta = pkgs.rustPlatform.buildRustPackage {
            pname = "rs-meta";
            version = "0.0.1";
            src = ./rs-meta;
            cargoLock.lockFile = ./rs-meta/Cargo.lock;
            doCheck = false; # the self-host check nests evaluators + needs rustc; run it via `nix run`
            nativeBuildInputs = [ pkgs.makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/bootstrap \
                --prefix PATH : ${pkgs.lib.makeBinPath nativeTier}
            '';
            meta = {
              description = "Standalone Rust-in-Rust meta-circular compiler/evaluator (stage15-N)";
              mainProgram = "bootstrap";
            };
          };

          # pnix-rs: rs-meta-backed pnix runtime front-end (Rust <-> px
          # projection). Pure facilities (px-eval/mirror/ir/gate/...) work
          # standalone; the substrate/rust-mirror/cross-host lanes additionally
          # need rs-meta at RS_META_BOOTSTRAP + rustc. The wrapper wires both.
          pnix-rs = pkgs.rustPlatform.buildRustPackage {
            pname = "pnix-rs";
            version = "0.0.1";
            src = ./pnix-rs;
            cargoLock.lockFile = ./pnix-rs/Cargo.lock;
            doCheck = false;
            nativeBuildInputs = [ pkgs.makeWrapper ];
            postPatch = ''
              # interop.rs includes the pnix-agnostic rs-meta IO adapter by
              # source path. Keep pnix-rs as the package root while making
              # that declared sibling seam explicit inside the Nix sandbox.
              mkdir -p ../rs-meta/src
              ln -s ${./rs-meta/src/io.rs} ../rs-meta/src/io.rs
            '';
            postInstall = ''
              wrapProgram $out/bin/pnix-rs \
                --set-default RS_META_BOOTSTRAP ${rs-meta}/bin/bootstrap \
                --prefix PATH : ${pkgs.lib.makeBinPath nativeTier}
            '';
            meta = {
              description = "rs-meta-backed pnix runtime front-end (Rust <-> px projection)";
              mainProgram = "pnix-rs";
            };
          };

          # One embeddable runtime source, projected to each target. This does
          # not introduce platform-specific evaluators: every artifact exports
          # src/lib.rs and delegates to the same src/px.rs implementation.
          mkPnixRsLibrary = targetPkgs: pname:
            targetPkgs.rustPlatform.buildRustPackage {
              inherit pname;
              version = "0.0.1";
              src = ./pnix-rs;
              cargoLock.lockFile = ./pnix-rs/Cargo.lock;
              cargoBuildFlags = [ "--lib" ];
              doCheck = false;
              installPhase = ''
                runHook preInstall
                mkdir -p "$out/lib" "$out/include" "$out/share/pnix-rs"
                find target -type f \
                  \( -name 'libpnix_rs.rlib' \
                     -o -name 'libpnix_rs.a' \
                     -o -name 'libpnix_rs.so' \
                     -o -name 'libpnix_rs.dylib' \
                     -o -name 'pnix_rs.wasm' \) \
                  ! -path '*/deps/*' -exec cp -v {} "$out/lib/" \;
                cp ${./pnix-rs/include/pnix_rs.h} "$out/include/pnix_rs.h"
                printf '%s\n' '${targetPkgs.stdenv.hostPlatform.config}' \
                  > "$out/share/pnix-rs/target"
                runHook postInstall
              '';
              meta = {
                description = "Embeddable PNIX runtime library for ${targetPkgs.stdenv.hostPlatform.config}";
                platforms = [ targetPkgs.stdenv.hostPlatform.system ];
              };
            };

          pnixRsLibrary = mkPnixRsLibrary pkgs "pnix-rs-library";
          pnixRsAndroidArm64 =
            mkPnixRsLibrary pkgs.pkgsCross.aarch64-android
              "pnix-rs-android-arm64";
          pnixRsWasmWasi =
            mkPnixRsLibrary pkgs.pkgsCross.wasi32 "pnix-rs-wasm-wasi";
          pnixRsIosArm64 = if pkgs.stdenv.isDarwin then
            mkPnixRsLibrary pkgs.pkgsCross.iphone64 "pnix-rs-ios-arm64"
          else null;
          pnixRsDesktop = pkgs.symlinkJoin {
            name = "pnix-rs-desktop";
            paths = [ pnix-rs rs-meta pnixRsLibrary ];
            meta.mainProgram = "pnix-rs";
          };
        in
        {
          default = pnix-rs;
          inherit rs-meta pnix-rs;
          pnix-rs-library = pnixRsLibrary;
          pnix-rs-android-arm64 = pnixRsAndroidArm64;
          pnix-rs-wasm-wasi = pnixRsWasmWasi;
          pnix-rs-desktop = pnixRsDesktop;
        } // pkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
          pnix-rs-ios-arm64 = pnixRsIosArm64;
        });

      # ---- apps (runners, classified by ROLE) ----------------------------
      # Two languages, two roles each.  rs-meta is the RUST engine (a meta-
      # circular compiler/evaluator: a subset-Rust `run` interpreter kept equal
      # to a `native-run` rustc tier by translation validation -- a peer engine
      # that USES the rustc toolchain, NOT a cargo/rustc drop-in).  pnix-rs is
      # the pnix (px) engine + the Rust<->px front-end; it also drives the
      # interactive REPLs (rs-meta stays a pure floor -- no interactive io).
      #
      # RUST compiler/evaluator:
      #   nix run .#rs-meta -- run -c '<rust>'          # interpret (trusted floor)
      #   nix run .#rs-meta -- native-run -c '<rust>'   # compile via rustc (native tier)
      #   nix run .#pnix-rs-rust                        # interactive Rust REPL (drives rs-meta interp)
      # PNIX (px) compiler/evaluator:
      #   nix run .#pnix-rs-px-eval -- -f default.px    # evaluate a .px file (compiler mode)
      #   nix run .#pnix-rs-px-eval -- -c '<px>'        # evaluate inline px
      #   nix run .#pnix-rs-pnix                        # interactive px REPL (interpreter mode)
      # FRONT-END / CHECKS:
      #   nix run .#pnix-rs -- <cmd>                    # full pnix-rs CLI (mirror/ir/gate/engine/...)
      #   nix run .#rs-meta-check                       # rs-meta self-check   (from rs-meta/)
      #   nix run .#pnix-rs-check                       # pnix-rs all_ready     (from pnix-rs/)
      #   nix run .#substrate-check                     # rs-meta<->pnix-rs 3-way proof (from pnix-rs/)
      apps = forAll (pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system};
          # Run an installed binary's subcommand from the CURRENT source dir (the
          # full checks read the repo's source files, like a `cargo test`).
          srcCheck = { name, bin, needFile, hint, argv }: pkgs.writeShellApplication {
            inherit name;
            text = ''
              if [ ! -e "$PWD/${needFile}" ]; then
                echo "${name}: run this from the ${hint} source directory (needs ./${needFile})." >&2
                exit 2
              fi
              exec ${bin} ${argv} "$@"
            '';
          };
          # A runner pinned to a fixed subcommand of an installed binary, still
          # forwarding the user's `-- <args>` (e.g. `-f default.px`). This is how
          # one binary exposes several role-specific runners.
          sub = { name, bin, argv }: pkgs.writeShellApplication {
            inherit name;
            text = ''exec ${bin} ${argv} "$@"'';
          };
          rsMetaCheck = srcCheck {
            name = "rs-meta-check"; bin = "${p.rs-meta}/bin/bootstrap";
            needFile = "src/interp.rs"; hint = "rs-meta"; argv = "check";
          };
          pnixRsCheck = srcCheck {
            name = "pnix-rs-check"; bin = "${p.pnix-rs}/bin/pnix-rs";
            needFile = "src/px.rs"; hint = "pnix-rs"; argv = "check";
          };
          substrateCheck = srcCheck {
            name = "substrate-check"; bin = "${p.pnix-rs}/bin/pnix-rs";
            needFile = "src/px.rs"; hint = "pnix-rs"; argv = "substrate-check";
          };
          # This host's full gate, under the name every pnix host uses: the
          # rs-meta floor first, then the pnix-rs product, then the two-way
          # substrate proof between them.
          # Each check validates the source tree it belongs to, so the gate runs
          # every one from its own directory rather than from a single cwd.
          gateApp = pkgs.writeShellApplication {
            name = "pnix-rs-gate";
            text = ''
              root="$PWD"
              if [ ! -e "$root/rs-meta/src/interp.rs" ] || [ ! -e "$root/pnix-rs/src/px.rs" ]; then
                echo "pnix-rs gate: run from the pnix-rs repo root (needs ./rs-meta and ./pnix-rs)." >&2
                exit 2
              fi
              (cd "$root/rs-meta" && ${pkgs.lib.getExe rsMetaCheck})
              (cd "$root/pnix-rs" && ${pkgs.lib.getExe pnixRsCheck})
              (cd "$root/pnix-rs" && ${pkgs.lib.getExe substrateCheck})
              echo "pnix-rs gate: PASS"
            '';
          };
          # px compiler/evaluator (file or inline) and the two REPLs. All go
          # through the wrapped pnix-rs binary, so RS_META_BOOTSTRAP is wired.
          pnixPxEval = sub { name = "pnix-rs-px-eval"; bin = "${p.pnix-rs}/bin/pnix-rs"; argv = "px-eval"; };
          replPnix = sub { name = "pnix-rs-pnix"; bin = "${p.pnix-rs}/bin/pnix-rs"; argv = "px-repl"; };
          replRust = sub { name = "pnix-rs-rust"; bin = "${p.pnix-rs}/bin/pnix-rs"; argv = "rust-repl"; };
        in
        {
          # front-end + Rust engine
          default = { type = "app"; program = pkgs.lib.getExe p.pnix-rs; };
          pnix-rs = { type = "app"; program = pkgs.lib.getExe p.pnix-rs; };
          rs-meta = { type = "app"; program = "${p.rs-meta}/bin/bootstrap"; };
          # px compiler + the two REPLs (interpreter mode)
          pnix-rs-pnix = { type = "app"; program = pkgs.lib.getExe replPnix; };
          pnix-rs-rust = { type = "app"; program = pkgs.lib.getExe replRust; };
          pnix-rs-px-eval = { type = "app"; program = pkgs.lib.getExe pnixPxEval; };
          # checks
          gate = { type = "app"; program = pkgs.lib.getExe gateApp; };
          rs-meta-check = { type = "app"; program = pkgs.lib.getExe rsMetaCheck; };
          pnix-rs-check = { type = "app"; program = pkgs.lib.getExe pnixRsCheck; };
          substrate-check = { type = "app"; program = pkgs.lib.getExe substrateCheck; };
        });

      # ---- devShell ------------------------------------------------------
      # `nix develop` then, from a lane's source dir:
      #   (rs-meta/)  bootstrap check
      #   (pnix-rs/)  pnix-rs check          # RS_META_BOOTSTRAP already set
      #               pnix-rs px-eval -c 'let a = 1; b = a + 2; in a + b'
      #   or rebuild from source: cargo build --release
      devShells = forAll (pkgs:
        let
          p = self.packages.${pkgs.stdenv.hostPlatform.system};
        in
        {
          default = pkgs.mkShell {
            packages = [ p.rs-meta p.pnix-rs pkgs.cargo pkgs.rustc pkgs.stdenv.cc pkgs.git ];
            shellHook = ''
              export RS_META_BOOTSTRAP="${p.rs-meta}/bin/bootstrap"
              echo "rs-meta + pnix-rs devShell"
              echo "  bootstrap / pnix-rs on PATH; RS_META_BOOTSTRAP wired to the flake's rs-meta."
              echo "  examples:"
              echo "    bootstrap run -c 'fn main(){ println!(\"{}\", 6*7); }'  # Rust: interpret"
              echo "    bootstrap native-run -c 'fn main(){}'                   # Rust: compile via rustc"
              echo "    pnix-rs rust-repl                       # Rust REPL (drives rs-meta interp)"
              echo "    pnix-rs px-eval -c 'let a = 1; b = a + 2; in a + b'     # px: evaluate"
              echo "    pnix-rs px-repl                         # px REPL (interpreter mode)"
              echo "    pnix-rs mirror -c 'let x = 21; in x + x'"
              echo "    pnix-rs rust-mirror -c '{ a = 1; b = [ 2 3 ]; }'"
              echo "    (from pnix-rs/) pnix-rs check          # all_ready aggregate"
              echo "    (from rs-meta/) bootstrap check        # self-check"
              echo "    (from pnix-rs/) ./examples/run-all.sh  # the Rust<->pnix examples"
            '';
          };
        });
    };
}
