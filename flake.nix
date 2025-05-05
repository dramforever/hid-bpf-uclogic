{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      eachSystem = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    {
      devShell = eachSystem (
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
