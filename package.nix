{
  stdenv,
  rustPlatform,
  lib,
}: let
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
