{
  description = "Fast TUI app launcher and fuzzy finder for GNU/Linux and *BSD - v3.1.0-kiwicrab";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk' = pkgs.callPackage naersk {};

        buildInputs = with pkgs; [
          pkg-config
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
        ];

      in
      {
        packages = {
          default = naersk'.buildPackage {
            pname = "fsel";
            version = "3.1.0-kiwicrab";
            src = ./.;

            nativeBuildInputs = with pkgs; [ pkg-config ];
            inherit buildInputs;

            # install man page
            postInstall = ''
              install -Dm644 fsel.1 $out/share/man/man1/fsel.1
            '';

            meta = with pkgs.lib; {
              description = "Fast TUI app launcher and fuzzy finder for GNU/Linux and *BSD - v3.1.0-kiwicrab";
              homepage = "https://github.com/Mjoyufull/fsel";
              license = licenses.bsd2;
              maintainers = [ ];
              platforms = platforms.linux ++ platforms.darwin;
            };
          };
        };

        apps = {
          default = flake-utils.lib.mkApp {
            drv = self.packages.${system}.default;
          };
        };

        devShells = {
          default = pkgs.mkShell {
            inherit buildInputs;
            nativeBuildInputs = with pkgs; [
              rustc
              cargo
              pkg-config
              cargo-watch
              rust-analyzer
            ];
          };
        };
      }
    );
}
