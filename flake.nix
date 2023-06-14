{
  inputs = {
    holonix = {
      url = "github:holochain/holochain";
      inputs.versions.url = "github:holochain/holochain/?dir=versions/0_1";
      inputs.holochain.url = "github:holochain/holochain/holochain-0.2.0-beta-rc.6";
    };
    nixpkgs.follows = "holonix/nixpkgs";
  };

  outputs = inputs@{ holonix, ... }:
    holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      # provide a dev shell for all systems that the holonix flake supports
      systems = builtins.attrNames holonix.devShells;

      perSystem = { config, system, pkgs, ... }:
        {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ holonix.devShells.${system}.holonix ];
            packages = with pkgs; [
              # add further packages from nixpkgs
            ];
          };
        };
    };
}