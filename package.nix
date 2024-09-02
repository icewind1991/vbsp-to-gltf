{
  stdenv,
  rust-bin,
  makeRustPlatform,
  meshoptimizer,
  vbsp-server-viewer,
  lib,
}: let
  toolchain = rust-bin.stable.latest.default;
  rustPlatform = makeRustPlatform {
    rustc = toolchain;
    cargo = toolchain;
  };
  inherit (lib.sources) sourceByRegex;
  src = sourceByRegex ./. ["Cargo.*" "(src|derive|benches|tests|examples.*)(/.*)?"];
in
  rustPlatform.buildRustPackage rec {
    pname = "vbsp-server";
    version = "0.1.0";

    GLTFPACK = "${meshoptimizer}/bin/gltfpack";

    inherit src;

    postPatch = ''
      cp -r ${vbsp-server-viewer} src/server/viewer/dist
    '';

    doCheck = false;

    cargoLock = {
      lockFile = ./Cargo.lock;
    };
    buildFeatures = [ "server" ];
  }
