{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/8c64c8887fd3c24b97781b49cc8ef87b283fc3bd";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flutter-engine = {
      url = "github:flutter/engine/3.24.4";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flutter-engine,
    }:
    let
      inherit (nixpkgs) lib;

      systems = lib.intersectLists lib.systems.flakeExposed lib.platforms.linux;

      forAllSystems = lib.genAttrs systems;

      # just make it impossible to forget to sync these by reading the lockfile
      lockfile = (builtins.fromJSON (builtins.readFile "${self}/flake.lock")).nodes;
    in
    {
      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-rfc-style);

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          rust-bin = rust-overlay.lib.mkRustBin { } pkgs;
          rust-nightly-toolchain = rust-bin.selectLatestNightlyWith (
            toolchain:
            toolchain.default.override {
              extensions = [
                "rust-analyzer"
                "rust-src"
              ];
            }
          );

          engine = pkgs.flutterPackages-source.stable.engine;
        in
        {
          default = pkgs.mkShell {
            buildInputs = [
              rust-nightly-toolchain
              pkgs.bacon
              pkgs.pkg-config
              engine
            ];

            LIBCLANG_PATH = lib.makeLibraryPath [
              pkgs.libclang
            ];

            FLUTTER_ENGINE = "${engine.release}/out/host_release";

            shellHook =
              let
                color = color-code: str: "$(tput setaf ${toString color-code})${str}$(tput sgr0)";
              in
              ''
                cp -f ${flutter-engine}/shell/platform/embedder/embedder.h embedder.h
              ''
              + nixpkgs.lib.optionalString (pkgs.flutter.version != lockfile.flutter-engine.original.ref) ''
                echo
                echo "Flutter version in nixpkgs is at ${color 3 pkgs.flutter.version}"
                echo "But ${color 4 "./embedder.h"} is from ${color 5 lockfile.flutter-engine.original.ref}"
                echo "Update the flake input? Remember to also update the relevant bindings."
                echo
              '';
          };
        }
      );
    };
}
