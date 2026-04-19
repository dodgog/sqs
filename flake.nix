{
  description = "sqs - reorder lists from the terminal";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "aarch64-darwin";
    pkgs = import nixpkgs {inherit system;};
  in {
    packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
      pname = "sqs";
      version = "0.3.1";
      src = self;
      cargoLock.lockFile = ./Cargo.lock;
    };

    defaultPackage.${system} = self.packages.${system}.default;
  };
}
