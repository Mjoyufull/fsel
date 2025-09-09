{
  description = "Fast TUI launcher for GNU/Linux and *BSD";

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
            pname = "gyr";
            version = "0.2.6";

            src = ./.;

            cargoHash = "sha256-mjQTuHJ6jVSlDMgr7rDp+aIox2auhzIjIbK0kNHgjgU=";

            inherit buildInputs nativeBuildInputs;

            meta = with pkgs.lib; {
              description = "Fast TUI launcher for GNU/Linux and *BSD";
              homepage = "https://github.com/Mjoyufull/gyr";
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
