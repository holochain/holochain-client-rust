{
  inputs = {
    holonix.url = "github:holochain/holonix?ref=main";
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
            ] ++ (pkgs.lib.optionals pkgs.stdenv.isLinux [
              pkgs.libclang
              pkgs.pkg-config
              pkgs.rustPlatform.bindgenHook
            ]) ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.bzip2
            ]);

            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          };
        };
    };
}
