{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      eachSystem = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
      pkgs = eachSystem (
        system:
        import nixpkgs {
          inherit system;
          overlays = [ self.overlays.default ];
        }
      );
    in
    {
      devShells = eachSystem (system: {
        default = nixpkgs.legacyPackages.${system}.callPackage ./nix/dev.nix { };
      });

      overlays.default = final: prev: {
        hid-bpf-uclogic = final.callPackage ./nix/package.nix { };
      };

      packages = eachSystem (system: {
        default = pkgs.${system}.hid-bpf-uclogic;
      });

      checks = eachSystem (system: {
        hid-bpf-uclogic = self.packages.${system}.default;
        hid-bpf-uclogic-test = self.packages.${system}.default.overrideAttrs {
          pname = "hid-bpf-uclogic-test";
          dontBuild = true;
          installPhase = "touch $out";
          buildAndTestSubdir = "hid-bpf-uclogic-test";
        };
      });
    };
}
