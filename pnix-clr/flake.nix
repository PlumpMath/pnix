{
  description = "Experimental PNIX ClojureCLR/.NET host bootstrap";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      sourceTree = "${self}";
      sourceKey = builtins.substring 0 16
        (builtins.hashString "sha256" "${sourceTree}");
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          sourceRunner = {
            name,
            script,
            extraRuntimeInputs ? [ ],
            prepare ? "",
          }: pkgs.writeShellApplication {
            inherit name;
            runtimeInputs = [
              pkgs.dotnet-sdk_10
              pkgs.bash
              pkgs.coreutils
              pkgs.jq
              pkgs.ripgrep
            ] ++ extraRuntimeInputs;
            text = ''
              if [ -x "$PWD/${script}" ] && [ -d "$PWD/pnix-clr" ]; then
                source_root="$PWD"
              else
                cache_base="''${XDG_CACHE_HOME:-$HOME/.cache}/pnix-clr"
                cache_root="$cache_base/${sourceKey}"
                if [ ! -f "$cache_root/.ready" ]; then
                  mkdir -p "$cache_base"
                  temp_root="$(mktemp -d "$cache_base/.${sourceKey}.XXXXXX")"
                  trap 'rm -rf "$temp_root"' EXIT
                  cp -R ${sourceTree} "$temp_root/pnix-clr"
                  # Store paths are read-only.
                  chmod -R u+w "$temp_root"
                  touch "$temp_root/.ready"
                  if mv "$temp_root" "$cache_root" 2>/dev/null; then
                    trap - EXIT
                  else
                    rm -rf "$temp_root"
                    trap - EXIT
                  fi
                fi
                source_root="$cache_root/pnix-clr"
              fi
              if [ ! -x "$source_root/${script}" ]; then
                echo "${name}: source closure is missing ${script}" >&2
                exit 2
              fi
              ${prepare}
              exec "$source_root/${script}" "$@"
            '';
          };
          pnixClrRepl = sourceRunner {
            name = "pnix-clr-pnix";
            script = "bin/pnix-clr";
            prepare = ''
              if [ ! -f "$source_root/pnix-clr/target/runtime-artifact/manifest.json" ]; then
                "$source_root/bin/build-pnix-clr-artifact" >/dev/null
              fi
              set -- --repl "$@"
            '';
          };
          pnixClr = sourceRunner {
            name = "pnix-clr";
            script = "bin/pnix-clr";
            prepare = ''
              if [ ! -f "$source_root/pnix-clr/target/runtime-artifact/manifest.json" ]; then
                "$source_root/bin/build-pnix-clr-artifact" >/dev/null
              fi
            '';
          };
          clrMeta = sourceRunner {
            name = "clr-meta";
            script = "bin/clr-meta";
          };
          gate = sourceRunner {
            name = "pnix-clr-gate";
            script = "bin/pnix-clr-gate";
            extraRuntimeInputs = [ pkgs.nix ];
          };
          # Materialize host library (guest AOT + Pnix.Clr managed + MSBuild).
          # Impure: builds AOT/NuGet into cache or checkout; prints PNIX_CLR_LIBRARY=.
          pnixClrLibrary = sourceRunner {
            name = "pnix-clr-library";
            script = "bin/export-pnix-clr-library";
            prepare = ''
              if [ ! -f "$source_root/pnix-clr/target/runtime-artifact/manifest.json" ]; then
                "$source_root/bin/build-pnix-clr-artifact" >/dev/null
              fi
              # Default export into the cache/checkout target tree unless OUT is given.
              if [ "$#" -eq 0 ]; then
                set -- "$source_root/pnix-clr/target/pnix-clr-library"
              fi
            '';
          };
          # Bare-name host substrate facade (clojure-clr → focused clr-meta -e/file).
          clojureClr = sourceRunner {
            name = "clojure-clr";
            script = "bin/clojure-clr";
          };
          # Print Reference / env paths after ensuring the library export exists.
          pnixClrRefs = sourceRunner {
            name = "pnix-clr-refs";
            script = "bin/export-pnix-clr-library";
            prepare = ''
              lib_out="$source_root/pnix-clr/target/pnix-clr-library"
              if [ ! -f "$lib_out/manifest.json" ]; then
                "$source_root/bin/export-pnix-clr-library" "$lib_out" >/dev/null
              fi
              echo "PNIX_CLR_ROOT=$source_root"
              echo "PNIX_CLR_LIBRARY=$lib_out"
              echo "PNIX_CLR_ARTIFACT=$lib_out/lib/net10.0/runtime-artifact"
              if [ -f "$lib_out/share/pnix-clr/refs.env" ]; then
                echo "# --- refs.env ---"
                cat "$lib_out/share/pnix-clr/refs.env"
              fi
              echo "# Guest AOT DLLs:"
              find "$lib_out/lib/net10.0/runtime-artifact" -maxdepth 1 -name '*.dll' -print | sort
              echo "# Managed Pnix.Clr:"
              find "$lib_out/lib" -name 'Pnix.Clr.dll' -print | sort
              exit 0
            '';
          };
        in {
          default = pnixClr;
          pnix-clr = pnixClr;
          pnix-clr-pnix = pnixClrRepl;
          pnix-clr-library = pnixClrLibrary;
          pnix-clr-refs = pnixClrRefs;
          clojure-clr = clojureClr;
          clr-meta = clrMeta;
          inherit gate;
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.pnix-clr}/bin/pnix-clr";
          meta.description = "Run the experimental PNIX ClojureCLR host";
        };
        pnix-clr = {
          type = "app";
          program = "${self.packages.${system}.pnix-clr}/bin/pnix-clr";
          meta.description = "Run the experimental PNIX ClojureCLR host";
        };
        pnix-clr-pnix = {
          type = "app";
          program = "${self.packages.${system}.pnix-clr-pnix}/bin/pnix-clr-pnix";
          meta.description = "Interactive PNIX REPL on the ClojureCLR host";
        };
        pnix-clr-library = {
          type = "app";
          program = "${self.packages.${system}.pnix-clr-library}/bin/pnix-clr-library";
          meta.description = "Export C# + guest AOT library layout (PNIX_CLR_LIBRARY)";
        };
        pnix-clr-refs = {
          type = "app";
          program = "${self.packages.${system}.pnix-clr-refs}/bin/pnix-clr-refs";
          meta.description = "Print DLL/Reference paths for the exported CLR library";
        };
        clojure-clr = {
          type = "app";
          program = "${self.packages.${system}.clojure-clr}/bin/clojure-clr";
          meta.description = "Focused ClojureCLR -e / single-file facade (clr-meta)";
        };
        clr-meta = {
          type = "app";
          program = "${self.packages.${system}.clr-meta}/bin/clr-meta";
          meta.description = "Run the PNIX-agnostic ClojureCLR meta bootstrap";
        };
        gate = {
          type = "app";
          program = "${self.packages.${system}.gate}/bin/pnix-clr-gate";
          meta.description = "Run the focused PNIX CLR bootstrap gate";
        };
      });

      devShells = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; };
        in {
          default = pkgs.mkShell {
            packages = [
              pkgs.dotnet-sdk_10
              pkgs.git
              pkgs.jq
              pkgs.ripgrep
            ];
            shellHook = ''
              echo "pnix-clr: ./bin/build-clr && ./bin/pnix-clr-gate"
            '';
          };
        });

      checks = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; };
        in {
          identity = pkgs.runCommand "pnix-clr-identity" {
            nativeBuildInputs = [ pkgs.bash pkgs.ripgrep ];
            src = sourceTree;
          } ''
            cp -R "$src" source
            chmod -R u+w source
            cd source
            ./bin/pnix-clr-identity-gate
            touch "$out"
          '';
        });
    };
}
