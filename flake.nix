{
  description = "Terminal user interface for SSH";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        version = "3.2.0" + "-" + (self.shortRev or "dirty");
        pkgs = nixpkgs.legacyPackages.${system};
      in rec {
        packages.default = pkgs.buildGoModule {
          pname = "sshs";
          inherit version;
          src = ./.;
          ldflags = ["-s" "-w" "-X github.com/quantumsheep/sshs/cmd.Version=${version}"];
          vendorSha256 = "OCh37wjSs40Q0VQmoc1nXQ4nWddnoUCrI5xgxpxR/Ec=";
        };
      }
    );
}
