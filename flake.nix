{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    {
      devShell = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
        system:
        nixpkgs.legacyPackages.${system}.callPackage (
          {
            mkShell,
            zlib,
            elfutils,
            libbpf,
            pkg-config,
            cargo,
            rustfmt,
            rustc,
            rustPlatform,
          }:

          mkShell {
            nativeBuildInputs = [
              pkg-config
              cargo
              rustfmt
              rustc
            ];
            buildInputs = [
              zlib
              elfutils
              libbpf
            ];

            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          }
        ) { }
      );
    };
}
