{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rust = (pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" ];
        });

        shellInputs = with pkgs; [
          rust
          clang
          mold
          trunk
          butler
          binaryen
        ];
        appNativeBuildInputs = with pkgs; [
          pkg-config
        ];
        appBuildInputs = appRuntimeInputs ++ (with pkgs; [
          udev
          alsaLib
          xorg.libX11
          vulkan-tools
          vulkan-headers
          vulkan-validation-layers
        ]);
        appRuntimeInputs = with pkgs; [
          udev
          alsaLib
          vulkan-loader
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];
      in
      with pkgs;
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = appNativeBuildInputs;
          buildInputs = shellInputs ++ appBuildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath appRuntimeInputs}"
            ln -fsT ${rust} ./.direnv/rust
          '';
        };
      }
    );
}
