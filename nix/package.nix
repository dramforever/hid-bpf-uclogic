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
  cargoToml = with builtins; fromTOML (readFile ../hid-bpf-uclogic/Cargo.toml);

in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = ../.;
  buildAndTestSubdir = "hid-bpf-uclogic";

  cargoLock.lockFile = ../Cargo.lock;

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
