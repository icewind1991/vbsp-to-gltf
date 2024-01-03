{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-23.11";
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.inputs.flake-utils.follows = "utils";
    cross-naersk.url = "github:icewind1991/cross-naersk";
    cross-naersk.inputs.nixpkgs.follows = "nixpkgs";
    cross-naersk.inputs.naersk.follows = "naersk";
    steam-fetcher = {
      #      url = "github:nix-community/steam-fetcher";
      url = "github:icewind1991/steam-fetcher/filelist";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    naersk,
    rust-overlay,
    cross-naersk,
    steam-fetcher,
  }:
    utils.lib.eachDefaultSystem (system: let
      overlays = [
        steam-fetcher.overlays.default
        (import rust-overlay)
        (import ./overlay.nix)
      ];
      pkgs = (import nixpkgs) {
        inherit system overlays;
      };
      inherit (pkgs) lib callPackage rust-bin mkShell;
      inherit (lib.sources) sourceByRegex;

      msrv = (fromTOML (readFile ./Cargo.toml)).package.rust-version;
      inherit (builtins) fromTOML readFile;
      toolchain = rust-bin.stable.latest.default;
      msrvToolchain = rust-bin.stable."${msrv}".default;

      hostTarget = pkgs.hostPlatform.config;
      targets = [
        "x86_64-unknown-linux-musl"
        "x86_64-pc-windows-gnu"
        hostTarget
      ];
      releaseTargets = lib.lists.remove hostTarget targets;

      naersk' = callPackage naersk {
        rustc = toolchain;
        cargo = toolchain;
      };
      msrvNaersk = callPackage naersk {
        rustc = msrvToolchain;
        cargo = msrvToolchain;
      };
      cross-naersk' = pkgs.callPackage cross-naersk {inherit naersk;};

      buildMatrix = targets: {
        include =
          builtins.map (target: {
            inherit target;
            artifactSuffix = cross-naersk'.execSufficForTarget target;
          })
          targets;
      };

      src = sourceByRegex ./. ["Cargo.*" "(src|derive|benches|tests|examples.*)(/.*)?"];
      nearskOpt = {
        pname = "vbsp-to-gltf";
        root = src;
      };
    in rec {
      packages =
        lib.attrsets.genAttrs targets (target:
          (cross-naersk'.buildPackage target) nearskOpt)
        // rec {
          vbsp-to-gltf = packages.${hostTarget};
          check = naersk'.buildPackage (nearskOpt
            // {
              mode = "check";
            });
          clippy = naersk'.buildPackage (nearskOpt
            // {
              mode = "clippy";
            });
          test = naersk'.buildPackage (nearskOpt
            // {
              release = false;
              mode = "test";
            });
          msrv = msrvNaersk.buildPackage (nearskOpt
            // {
              mode = "check";
            });
          server = naersk'.buildPackage (nearskOpt
            // {
              pname = "vbsp-server";
              preConfigure = ''
                cargo_build_options="--features server $cargo_build_options"
              '';
              GLTFPACK = "${pkgs.meshoptimizer}/bin/gltfpack";
            });
          assets = pkgs.vbsp-server-assets;
          server-with-assets = server.overrideAttrs (old: {
            buildInputs = server.buildInputs ++ [pkgs.makeWrapper];
            postInstall = ''
              wrapProgram "$out/bin/vbsp-server" --set TF_DIR "${assets}"
            '';
          });
          vbsp-server = pkgs.vbsp-server;
          default = vbsp-to-gltf;
        };

      matrix = buildMatrix targets;
      releaseMatrix = buildMatrix releaseTargets;

      devShells = let
        tools = with pkgs; [
          bacon
          cargo-edit
          cargo-outdated
          cargo-audit
          cargo-msrv
          meshoptimizer
        ];
      in {
        default = mkShell {
          nativeBuildInputs = [toolchain] ++ tools;
        };
        msrv = mkShell {
          nativeBuildInputs = [msrvToolchain] ++ tools;
        };
      };
    })
    // {
      overlays.default = import ./overlay.nix;
    };
}
