{
  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.3";
    nixpkgs.follows = "holonix/nixpkgs";
  };

  outputs = inputs@{ holonix, ... }:
    holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      # provide a dev shell for all systems that the holonix flake supports
      systems = builtins.attrNames holonix.devShells;

      perSystem = { inputs', config, system, pkgs, ... }:
        {
          devShells.default = pkgs.mkShell {
            packages = [
              inputs'.holonix.packages.holochain
              inputs'.holonix.packages.lair-keystore
              inputs'.holonix.packages.rust
              pkgs.go
              pkgs.openssl
            ] ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [
              # needed to build Holochain on macos
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ]);
          };
        };
    };
}