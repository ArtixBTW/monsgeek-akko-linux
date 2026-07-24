{
  lib,
  fetchPnpmDeps,
  fetchFromGitHub,
  nodejs,
  pnpm_10,
  pnpmConfigHook,
  makeWrapper,
  stdenv,
}:
let
  # Make sure that the same nodejs version is referenced in nativeBuildInputs
  pnpm = pnpm_10;
in
stdenv.mkDerivation (finalAttrs: {
  pname = "webcrack";
  version = "2.16.0";

  src = fetchFromGitHub {
    owner = "j4k0xb";
    repo = "webcrack";
    tag = "v${finalAttrs.version}";
    hash = "sha256-IalU/wio1cNCr6K7Pa1neHVpnO2md4Ey6YmtReE3r+8=";
  };

  pnpmRoot = "packages/webcrack";

  pnpmDeps = fetchPnpmDeps {
    inherit (finalAttrs) pname version src;
    inherit pnpm;
    fetcherVersion = 4;
    hash = "sha256-VMP5Iwbd+lDvohyzosUdBlu2VLb6NkrGKi9UEHLJd0I=";
  };

  nativeBuildInputs = [
    nodejs # in case scripts are run outside of a pnpm call
    pnpmConfigHook
    pnpm # At least required by pnpmConfigHook, if not other (custom) phases
    makeWrapper
  ];

  # reference: https://discourse.nixos.org/t/how-to-package-a-pnpm-project/74529/10
  buildPhase = ''
    runHook preBuild

    pnpm build

    runHook postBuild
  '';

  installPhase =
    let
      installDir = "$out/lib/webcrack";
    in
    # bash
    ''
      runHook preInstall

      mkdir -p ${installDir}

      pnpm install --frozen-lockfile #--prod

      cp -r node_modules ${installDir}/node_modules
      cp -r apps ${installDir}/apps
      cp -r packages ${installDir}/packages

      makeWrapper ${lib.getExe nodejs} $out/bin/webcrack --add-flag ${installDir}/packages/webcrack/src/cli-wrapper.js

      runHook postInstall
    '';

  meta = {
    description = "Deobfuscate obfuscator.io, unminify and unpack bundled javascript";
    homepage = "https://github.com/j4k0xb/webcrack";
    license = lib.licenses.mit;
    # maintainers = with lib.maintainers; [];
    platforms = [ "x86_64-linux" ];
  };
})
