{
  description = "ndisk development flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs = { self, nixpkgs, ... }@inputs:
  let

    # allows us to use the same devShells/package/etc definitions for multiple architectures
    # borrowed from https://ayats.org/blog/no-flake-utils
    forAllSystems = function:
      nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ] (system: function nixpkgs.legacyPackages.${system});

  in {

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          rustc
          cargo
          gcc
          rustfmt
          clippy
          rustPlatform.bindgenHook # required for libparted, sets LIBCLANG_PATH and friends
          rustPlatform.cargoSetupHook
        ];
        buildInputs = with pkgs; [
          parted # for libparted
          libclang # for libparted-sys
        ];

        # Certain Rust tools won't work without this
        # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
        # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      };
    });
  };
}
