let
  nixpkgs = fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/cf63ade6f74bbc9d2a017290f1b2e33e8fbfa70a.tar.gz";
    sha256 = "1aw4avc6mp3v1gwjlax6yn8984c92y56s4h7qrygxagpchkwq06j";
  };

  pkgs = import nixpkgs {
    overlays = [];
    config = {};
  };

  result = pkgs.rustPlatform.buildRustPackage {
    name = "nix-refs";
    src = pkgs.lib.cleanSource ./.;
    cargoLock.lockFile = ./Cargo.lock;
  };
in result
