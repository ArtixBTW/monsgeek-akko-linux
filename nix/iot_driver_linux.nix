{
  version,

  lib,
  rustPlatform,

  pkg-config,
  protobuf,

  libpulseaudio,
  openssl,
  pipewire,
  udev,
}:
rustPlatform.buildRustPackage (final: {
  pname = "iot_driver_linux";
  inherit version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = ../iot_driver_linux;
  };

  sourceRoot = "${final.src.name}/iot_driver_linux";

  cargoLock = {
    lockFile = ../iot_driver_linux/Cargo.lock;
  };

  nativeBuildInputs = [
    rustPlatform.bindgenHook
    rustPlatform.cargoSetupHook

    pkg-config
    protobuf
  ];

  buildInputs = [
    libpulseaudio
    openssl.dev
    pipewire
    udev
  ];

  meta = {
    mainProgram = "iot_driver";

    description = " Linux driver for MonsGeek/Akko magnetic keyboards (RongYuan RY5088 firmware)";
    homepage = "https://github.com/echtzeit-solutions/monsgeek-akko-linux";
    license = lib.licenses.gpl3;
    platforms = [
      "x86_64-linux"
    ];
    # maintainers = with lib.maintainers; [ ];
  };
})
