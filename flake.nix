{
  description = "Fast TUI app launcher and fuzzy finder for GNU/Linux and *BSD - 4.0.0-nyamabeetle";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      naersk,
      fenix,
    }:
    # Nixpkgs 26.11 no longer supports x86_64-darwin.
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rustToolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "clippy"
          "rustc"
          "rustfmt"
        ];
        naersk' = pkgs.callPackage naersk {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

      in
      {
        packages = {
          default = naersk'.buildPackage {
            pname = "fsel";
            version = "4.0.0-nyamabeetle";
            src = ./.;

            nativeBuildInputs = with pkgs; [ pkg-config ];

            # install man page
            postInstall = ''
              install -Dm644 fsel.1 $out/share/man/man1/fsel.1
            '';

            meta = with pkgs.lib; {
              description = "Fast TUI app launcher and fuzzy finder for GNU/Linux and *BSD - 4.0.0-nyamabeetle";
              homepage = "https://github.com/Mjoyufull/fsel";
              license = licenses.bsd2;
              maintainers = [ ];
              platforms = platforms.linux ++ platforms.darwin;
              mainProgram = "fsel";
            };
          };
        };

        apps = {
          default = {
            type = "app";
            program = pkgs.lib.getExe self.packages.${system}.default;
            meta.description = self.packages.${system}.default.meta.description;
          };
        };

        formatter = pkgs.nixfmt;

        devShells = {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.default ];
            packages = with pkgs; [
              cargo-watch
              rust-analyzer
            ];
          };
        };
      }
    );
}
