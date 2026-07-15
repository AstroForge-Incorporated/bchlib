{
  description = "Dev shells for bchlib: Rust toolchain for building/testing, and Ruby for the Buildkite pipeline DSL";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # single source of truth for the compiler version: this reads
        # channel/profile/components from rust-toolchain.toml, so the nix
        # shell and any rustup-based local dev use the identical toolchain
        # instead of nix silently floating to whatever's newest.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        buildkiteBuilderGem = pkgs.bundlerEnv {
          name = "buildkite-builder-bchlib";
          gemdir = ./.;
        };
      in
      {
        devShells = {
          rust_stable = pkgs.mkShell {
            buildInputs = [
              rustToolchain
              pkgs.pkg-config
              pkgs.clang
            ];
            # populates LIBCLANG_PATH and BINDGEN_EXTRA_CLANG_ARGS (with the
            # correct -resource-dir already baked in) for bindgen, instead of
            # us reconstructing that ourselves.
            nativeBuildInputs = [ pkgs.rustPlatform.bindgenHook ];
          };

          buildkite = pkgs.mkShell {
            buildInputs = [
              buildkiteBuilderGem
              buildkiteBuilderGem.wrappedRuby
            ];
          };
        };
      });
}
