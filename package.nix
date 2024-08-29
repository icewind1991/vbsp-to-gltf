{
  stdenv,
  rust-bin,
  makeRustPlatform,
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

    inherit src;

    cargoLock = {
      lockFile = ./Cargo.lock;
    };
    buildFeatures = [ "server" ];
  }
