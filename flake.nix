{
  description = "Browse AI models, benchmarks, and coding agents from the terminal";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        naersk-lib = naersk.lib.${system};
      in {
        packages.default = naersk-lib.buildPackage {
          pname = "modelsdev";
          src = ./.;
          meta = with pkgs.lib; {
            description = "Browse AI models, benchmarks, and coding agents from the terminal";
            homepage = "https://github.com/arimxyer/models";
            license = licenses.mit;
            mainProgram = "models";
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo rustc rustfmt clippy rust-analyzer
          ];
        };
      }
    );
}
