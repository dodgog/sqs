{
  description = "sqs - reorder lists from the terminal";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    forAllSystems = nixpkgs.lib.genAttrs [
      "aarch64-darwin"
      "x86_64-linux"
      "aarch64-linux"
    ];
    pkgsFor = system: import nixpkgs {inherit system;};
  in {
    packages = forAllSystems (system: {
      default = (pkgsFor system).rustPlatform.buildRustPackage {
        pname = "sqs";
        version = "0.3.3";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
      };
    });

    defaultPackage = forAllSystems (system: self.packages.${system}.default);
  };
}
