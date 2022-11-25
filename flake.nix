{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, fenix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };
      in
      with pkgs; {
        devShells = {
          default = mkShell {
            nativeBuildInputs = [
              (pkgs.fenix.stable.defaultToolchain)
            ];
          };
          msrv = mkShell {
            nativeBuildInputs = [
              (pkgs.fenix.toolchainOf {
                channel = "1.33";
                sha256 = "sha256-CzEKnrTx8LAVk1fLRtLPQFYH1RoU11owRkBdfhhINjI=";
              }).minimalToolchain
            ];
          };
        };
      }
    );
}

