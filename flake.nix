{
  description = "Fast TUI app launcher and fuzzy finder for GNU/Linux and *BSD";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        buildInputs = with pkgs; [
          # Required for termion
          pkg-config
        ] ++ lib.optionals stdenv.isDarwin [
          # Darwin specific dependencies if needed
          darwin.apple_sdk.frameworks.Security
        ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

      in
      {
        packages = {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "fsel";
            version = "1.0.0-riceknife";

            src = ./.;

            cargoHash = "sha256-XwWH8uvmaD111wkCUJyQkuZSZUcFMwbGXk1MfQUn5oQ=";

            inherit buildInputs nativeBuildInputs;

            # Install man page
            postInstall = ''
              install -Dm644 fsel.1 $out/share/man/man1/fsel.1
            '';

            meta = with pkgs.lib; {
              description = "Fast TUI app launcher and fuzzy finder for GNU/Linux and *BSD";
              homepage = "https://github.com/Mjoyufull/fsel";
              license = licenses.bsd2;
              maintainers = with maintainers; [ "Mjoyufull" ];
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
            nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
              # Development tools
              cargo-watch
              rust-analyzer
            ]);

            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          };
        };
      }
    );
}
