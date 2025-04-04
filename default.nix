{
  pkgs ? import <nixpkgs> { }
}:

pkgs.rustPlatform.buildRustPackage {
  name = "timer";
  version = "1.0.0";
  buildInputs = with pkgs; [ SDL2 SDL2_ttf ];
  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "sdl2-0.35.2" = "sha256-zhzJNgHpzFREsO59hvdr7GoNVbmhcoOCsHS5eL5Qam8=";
    };
  };
  src = pkgs.lib.cleanSource ./.;
}
