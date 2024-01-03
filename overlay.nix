prev: final: {
  vbsp-server = final.callPackage ./package.nix {};
  vbsp-server-assets = final.callPackage ./assets.nix {};
}
