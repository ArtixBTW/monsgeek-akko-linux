{
  description = "Linux driver for MonsGeek/Akko magnetic keyboards (RongYuan RY5088 firmware)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { ... }@inputs:
    let
      inherit (inputs.nixpkgs) lib;

      systems = [ "x86_64-linux" ];
      eachSystem = lib.genAttrs systems;

      pkgsFor = eachSystem (
        system:
        import inputs.nixpkgs {
          inherit system;
        }
      );
    in
    {
      formatter = eachSystem (system: inputs.nixpkgs.legacyPackages.${system}.nixfmt-tree);

      devShells = eachSystem (
        system:
        let
          pkgs = pkgsFor.${system};
        in
        {
          default = pkgs.mkShell {
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

            packages = with pkgs; [
              ## Build
              openssl
              pkg-config
              protobuf
              udev
              # Screen capture/sync (PipeWire)
              clang
              libpulseaudio
              pipewire
              # Rust
              rustc
              cargo

              # For scripts
              nodejs_latest
              node-gyp
            ];
          };
        }
      );

      packages = eachSystem (
        system:
        let
          pkgs = pkgsFor.${system};
        in
        {
          iot_driver_linux = pkgs.callPackage ./nix/iot_driver_linux.nix {
            version = inputs.self.shortRev or inputs.self.dirtyShortRev or "unknown";
          };
        }
      );
    };
}
