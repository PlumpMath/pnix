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
        in {
          default = pnixClr;
          pnix-clr = pnixClr;
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
