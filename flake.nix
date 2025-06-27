{
  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.5";
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
              inputs'.holonix.packages.hc
              inputs'.holonix.packages.rust
            ] ++ (pkgs.lib.optionals pkgs.stdenv.isLinux [
                pkgs.perl
                pkgs.cmake
                pkgs.clang
                pkgs.llvmPackages_18.libunwind
            ]) ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.bzip2
            ]);

            LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${pkgs.stdenv.cc.cc.lib}/lib";
            LIBCLANG_PATH = "${pkgs.llvmPackages_18.libclang.lib}/lib";
          };
        };
    };
}
