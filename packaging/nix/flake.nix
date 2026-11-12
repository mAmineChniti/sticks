# Nix packaging template.
#
# To use: copy `flake.nix` to the repository root (or vendor it into a nixpkgs
# overlay), then `nix build` / `nix run`.
#
# This builds sticks from source with rustPlatform — no release binaries are
# fetched, so checksums stay out of the picture entirely.

{
  description = "sticks — CLI for scaffolding and managing C/C++ projects";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        sticks = pkgs.rustPlatform.buildRustPackage {
          pname = "sticks";
          version = "0.4.1";
          src = ./.;
          cargoHash = "sha256-REPLACE_WITH_CARGO_VENDOR_HASH";
          meta = with pkgs.lib; {
            description = "CLI for scaffolding and managing C/C++ projects";
            homepage = "https://github.com/mAmineChniti/sticks";
            license = licenses.mit;
            mainProgram = "sticks";
          };
        };
      in {
        packages.default = sticks;
        apps.default = {
          type = "app";
          program = "${sticks}/bin/sticks";
        };
      });
}
