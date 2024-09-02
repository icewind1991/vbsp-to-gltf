{
  npmlock2nix,
  nodejs_20,
  lib,
}: let
  inherit (lib.sources) sourceByRegex;
in npmlock2nix.v2.build rec {
  pname = "vbsp-server-viewer";
  version = "0.1.0";

  src = ./src/server/viewer;
  nodejs = nodejs_20;

  installPhase = "cp -r dist $out";
  buildCommands = ["npm run build"];
}
