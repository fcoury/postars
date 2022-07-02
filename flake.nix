{
  description = "Rust library for email management.";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    naersk.url = "github:nix-community/naersk";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, naersk, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          name = "himalaya-lib";
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs { inherit system overlays; };
        in
        rec {
          # nix build
          defaultPackage = packages.${name};
          packages = {
            ${name} = naersk.lib.${system}.buildPackage {
              pname = name;
              root = ./.;
              nativeBuildInputs = with pkgs; [ openssl.dev pkgconfig ];
            };
          };

          # nix run
          defaultApp = apps.${name};
          apps.${name} = utils.lib.mkApp {
            inherit name;
            drv = packages.${name};
          };

          # nix develop
          devShell = pkgs.mkShell {
            inputsFrom = builtins.attrValues self.packages.${system};
            nativeBuildInputs = with pkgs; [
              # Nix LSP + formatter
              rnix-lsp
              nixpkgs-fmt

              # Rust env
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              cargo-watch
              rust-analyzer

              # Notmuch
              notmuch
            ];
          };
        }
      );
}
