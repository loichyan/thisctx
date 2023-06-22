{
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    ra-flake.url = "github:loichyan/ra-flake";
  };

  outputs = { nixpkgs, flake-utils, fenix, ra-flake, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            fenix.overlays.default
            ra-flake.overlays.default
          ];
        };
        rust = with pkgs.fenix;
          with toolchainOf
            {
              channel = "1.56";
              sha256 = "sha256-MJyH6FPVI7diJql9d+pifu5aoqejvvXyJ+6WSJDWaIA=";
            };
          combine [
            defaultToolchain
            rust-src
          ];
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };
        # Old version of rust-analyzer is required.
        rust-analyzer =
          pkgs.ra-flake.make {
            cargo = rust;
            rustc = rust;
            version.rust = "1.56";
            sha256 = "sha256-vh7z8jupVxXPOko3sWUsOB7eji/7lKfwJ/CE3iw97Sw=";
          };
      in
      with pkgs; {
        devShells = {
          default = mkShell {
            nativeBuildInputs = [
              rust
              rust-analyzer
            ];
          };
        };
      }
    );
}
