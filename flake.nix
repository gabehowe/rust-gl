{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, fenix, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        target = "x86_64-unknown-linux-gnu";

        toolchain = with fenix.packages.${system}; combine [
          minimal.cargo
          minimal.rustc
          targets.${target}.latest.rust-std
        ];

        commonBuildInputs = with pkgs; [
          git
          git-lfs
          cmake
          libx11
          libxrandr
          libxinerama
          libxcursor
          libxi
		  libxkbcommon
		  wayland
		  libglvnd
        ];
      in
      {
        packages.default =
          (naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          }).buildPackage {
            src = ./.;

            nativeBuildInputs = [ toolchain ];
            buildInputs = commonBuildInputs;

            CARGO_BUILD_TARGET = target;
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER =
              let
                inherit (pkgs.pkgsCross.aarch64-multiplatform.stdenv) cc;
              in
              "${cc}/bin/${cc.targetPrefix}cc";
          };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ toolchain ];
          buildInputs = commonBuildInputs;
          LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
            libX11
            libXcursor
            libXrandr
            libXi
            libxkbcommon
            wayland
			libglvnd
          ];

        };
      });
}
