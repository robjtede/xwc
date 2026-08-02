{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    x52 = {
      url = "github:x52dev/nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      perSystem =
        {
          pkgs,
          config,
          inputs',
          lib,
          ...
        }:
        {
          formatter = pkgs.nixfmt;

          devShells.default = pkgs.mkShell {
            packages = [
              config.formatter
              inputs'.x52.packages.x52-release-tools
              pkgs.cargo-machete
              pkgs.cargo-nextest
              pkgs.cargo-watch
              pkgs.fd
              pkgs.just
              pkgs.prettier
              pkgs.taplo
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.pkgsBuildHost.libiconv
            ];
          };

          devShells.release = pkgs.mkShell {
            packages = [ inputs'.x52.packages.x52-release-tools ];
          };
        };
    };
}
