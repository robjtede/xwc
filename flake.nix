{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-parts.url = "github:hercules-ci/flake-parts";
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
          lib,
          ...
        }:
        {
          formatter = pkgs.nixfmt-rfc-style;

          devShells.default = pkgs.mkShell {
            packages = [
              config.formatter
              pkgs.cargo-machete
              pkgs.cargo-nextest
              pkgs.cargo-watch
              pkgs.fd
              pkgs.just
              pkgs.nodePackages.prettier
              pkgs.taplo
            ]
            ++ lib.optional pkgs.stdenv.isDarwin [
              pkgs.pkgsBuildHost.libiconv
            ];
          };
        };
    };
}
