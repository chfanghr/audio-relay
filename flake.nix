{
  description = "Description for the project";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-flake.url = "github:juspay/rust-flake";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
        inputs.git-hooks-nix.flakeModule
      ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem = {
        config,
        self',
        pkgs,
        ...
      }: {
        pre-commit = {
          check.enable = true;
          settings.hooks = {
            alejandra.enable = true;
            deadnix.enable = true;
            rustfmt.enable = true;
            taplo.enable = true;
          };
        };

        rust-project.crates.audio-relay.crane.args = {
          buildInputs = [
            pkgs.rustPlatform.bindgenHook
            pkgs.pipewire
          ];
          nativeBuildInputs = [pkgs.pkg-config];
        };

        devShells.default = self'.devShells.rust.overrideAttrs (_: prevAttrs: {
          shellHook = ''
            ${prevAttrs.shellHook or ""}
            ${config.pre-commit.installationScript}
          '';
          buildInputs =
            (prevAttrs.buildInputs or [])
            ++ [
              config.pre-commit.settings.package
            ];
        });
      };
    };
}
