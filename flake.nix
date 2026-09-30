{
  description = "A basic flake with a shell";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };
  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.bashInteractive
            pkgs.rustup
          ];
          # rustup manages the actual toolchain + wasm32-wasip1 target, scoped
          # to this project directory so it stays out of ~/.rustup and is
          # reproducible without pinning a full toolchain via nix.
          shellHook = ''
            export RUSTUP_HOME="$PWD/.rustup"
            export CARGO_HOME="$PWD/.cargo"
            export PATH="$CARGO_HOME/bin:$PATH"
            rustup toolchain install stable --profile minimal --no-self-update 2>/dev/null
            rustup target add wasm32-wasip1 --toolchain stable 2>/dev/null
            rustup default stable >/dev/null
          '';
        };
      }
    );
}
