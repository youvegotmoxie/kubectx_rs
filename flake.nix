{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = inputs: {
    packages =
      builtins.mapAttrs (system: pkgs: {
        hello = pkgs.kubectl;

        default = inputs.self.packages.${system}.hello;
      })
      inputs.nixpkgs.legacyPackages;
  };
}
