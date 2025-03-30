{
  buildNpmPackage,
  importNpmLock,
  nodejs_20,
  lib,
}: buildNpmPackage rec {
  pname = "vbsp-server-viewer";
  version = "0.1.0";
  src = ./src/server/viewer;

  npmDeps = importNpmLock {
      npmRoot = src;
    };

    npmConfigHook = importNpmLock.npmConfigHook;

  installPhase = "cp -r dist $out";
}
