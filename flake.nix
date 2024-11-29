{
  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.4";
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
            ] ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [
              # needed to build Holochain on macos
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.bzip2
            ]);

            shellHook = ''
              export PS1='\[\033[1;34m\][holonix:\w]\$\[\033[0m\] '
            '';
          };
        };
    };
}
