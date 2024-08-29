{
  stdenv,
  rust-bin,
  makeRustPlatform,
  meshoptimizer,
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

    doCheck = false;

    cargoLock = {
      lockFile = ./Cargo.lock;
    };
    buildFeatures = [ "server" ];
  }
