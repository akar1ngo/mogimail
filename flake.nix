{
  description = "Embedded SMTP server in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.mkLib pkgs;

        # Common arguments can be set here to avoid repeating them later
        # Note: changes here will rebuild all dependency crates
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          buildInputs =
            [
              # Add additional build inputs here
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
            ];
        };

        # Build just the cargo dependencies, so we can reuse all of that work
        # (e.g. via cachix) when running in CI.
        cargoArtifacts = craneLib.buildDepsOnly (
          commonArgs
          // {
            pname = "mogimail";
          }
        );

        # Run clippy on the crate source, reusing dependency artifacts from
        # above. This is a separate derivation, so it doesn't affect building
        # the crate itself.
        mogimail-clippy = craneLib.cargoClippy (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          }
        );

        # Build the actual crate, reusing dependency artifacts from above.
        mogimail = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--all-features --locked";
            # Additional environment variables or build phases/hooks can be set
            # here *without* rebuilding all dependency crates
            # MY_CUSTOM_VAR = "some value";
          }
        );
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience.
          inherit mogimail;
          inherit mogimail-clippy;
        };

        packages.default = mogimail;

        apps.default = flake-utils.lib.mkApp {
          drv = mogimail;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            pkgs.clippy
            pkgs.nixd
            pkgs.rust-analyzer
          ];
        };
      }
    );
}
