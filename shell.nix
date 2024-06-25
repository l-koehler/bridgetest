{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup
    glib
    openssl
  ];
  nativeBuildInputs = with pkgs; [
    pkg-config
    openssl
  ];
  shellHook = ''
  export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath[
    pkgs.openssl
  ]};
  '';
}
