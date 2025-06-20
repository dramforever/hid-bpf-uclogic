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
            huion-switcher,
            hid-tools,
            rustPlatform,
            buildPackages,
          }:

          mkShell {
            nativeBuildInputs = [
              pkg-config
              cargo
              rustfmt
              rustc
              hid-tools
              buildPackages.llvmPackages.clang-unwrapped
            ];
            buildInputs = [
              zlib
              elfutils
              libbpf
              huion-switcher
            ];

            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          }
        ) { }
      );

      overlays.default = final: prev: {
        hid-bpf-uclogic = final.callPackage (
          {
            lib,
            rustPlatform,
            zlib,
            elfutils,
            libbpf,
            pkg-config,
            makeWrapper,
            buildPackages,
            huion-switcher,
          }:

          let
            cargoToml = with builtins; fromTOML (readFile ./Cargo.toml);

          in
          rustPlatform.buildRustPackage {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;

            src = ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [
              pkg-config
              makeWrapper
              buildPackages.llvmPackages.clang-unwrapped
            ];

            buildInputs = [
              zlib
              elfutils
              libbpf
            ];

            doCheck = false;

            postInstall = ''
              wrapProgram "$out/bin/"* \
                --set PATH ${lib.makeBinPath [ huion-switcher ]}
            '';

            meta.mainProgram = "hid-bpf-uclogic";
          }
        ) { };
      };

      packages = eachSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ self.overlays.default ];
          };
        in
        {
          inherit (pkgs) hid-bpf-uclogic;
          default = pkgs.hid-bpf-uclogic;
        }
      );
    };
}
