{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, flake-utils, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ inputs.fenix.overlays.default ];
        };
        inherit (pkgs) fenix lib;

        # Rust toolchain
        rustToolchainFile = lib.importTOML ./rust-toolchain.toml;
        rustChannel = {
          channel = rustToolchainFile.toolchain.channel;
          sha256 = "sha256-MJyH6FPVI7diJql9d+pifu5aoqejvvXyJ+6WSJDWaIA=";
        };
        rustToolchain = fenix.toolchainOf rustChannel;

        # For development
        rust-dev = fenix.combine (
          with rustToolchain;
          [
            defaultToolchain
            rust-src
          ]
        );

      in
      {
        devShells.default = with pkgs; mkShell { packages = [ rust-dev ]; };
      }
    );
}
