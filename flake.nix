{
  description = "Terminal user interface for SSH";
  
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
  flake-utils.lib.eachDefaultSystem (system:
  let
    version = builtins.substring 0 8 self.lastModifiedDate;
    pkgs = nixpkgs.legacyPackages.${system};
  in
    rec {
      defaultPackage = pkgs.buildGoModule {
        pname = "sshs";
        inherit version;
        src = ./.;
        vendorSha256 = "QWFz85bOrTnPGum5atccB5hKeATlZvDAt32by+DO/Fo=";
      };
    }
  );
}
