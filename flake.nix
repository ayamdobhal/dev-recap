{
  description = "dev-recap - AI-powered git commit summarizer for Demo Day presentations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
          };
        };

        buildInputs = with pkgs; [
          # Rust toolchain
          rustc
          cargo
          rust-analyzer
          clippy

          # Build dependencies
          pkg-config

          # Additional tools
          git
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          shellHook = ''
            echo "🦀 dev-recap development environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo run            - Run the application"
            echo "  cargo test           - Run tests"
            echo "  cargo watch -x run   - Watch and auto-rebuild"
            echo ""
          '';

          # Environment variables
          RUST_BACKTRACE = "1";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };

        # For building the final binary
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "dev-recap";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = buildInputs;

          meta = with pkgs.lib; {
            description = "AI-powered git commit summarizer for Demo Day presentations";
            homepage = "https://github.com/yourusername/dev-recap";
            license = licenses.mit;
            maintainers = [ ];
          };
        };
      }
    );
}
