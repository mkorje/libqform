{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    liboptarith = {
      url = "github:mkorje/liboptarith";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      liboptarith,
    }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      optarith = liboptarith.packages.${system}.default;
    in
    {
      packages.${system} = rec {
        default = qform;
        qform = pkgs.callPackage ./qform.nix {
          inherit optarith;
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          autoconf
          automake
          libtool
          pkg-config
          rustPlatform.bindgenHook
        ];

        buildInputs = with pkgs; [
          rustc
          cargo
          clippy
          rustfmt
          llvmPackages_latest.libclang
          llvmPackages_latest.clang
          clang-tools
        ];
      };
    };
}
