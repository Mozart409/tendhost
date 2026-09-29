{
  description = "Development environment for tendhost project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
        overlays = [rust-overlay.overlays.default];
      };
      rust = pkgs.rust-bin.stable."1.96.1".default;
    in {
      # to use other shells, run:
      # nix develop . --command fish
      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # keep-sorted start
          rust
          lazydocker
          bacon
          cargo-deny
          lefthook
          cocogitto
          just
          cargo-workspaces
          opentofu
          postgresql_18
          tailwindcss_4
          podman
          podman-compose
          sqlx-cli
          websocat
          scc
          cargo-outdated
          cargo-edit
          cargo-deny
          cargo-machete
          keep-sorted
          # keep-sorted end
        ];
        shellHook = ''
          lefthook install
        '';
      };
    });
}
