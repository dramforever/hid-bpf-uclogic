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
