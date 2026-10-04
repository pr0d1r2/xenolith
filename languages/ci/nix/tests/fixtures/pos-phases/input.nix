{ stdenv }:
stdenv.mkDerivation {
  pname = "demo";
  version = "1.0";
  src = ./.;
  phases = [
    "buildPhase"
    "installPhase"
  ];
  buildPhase = ''
    make all
  '';
  installPhase = ''
    install -Dm755 demo $out/bin/demo
  '';
  checkPhase = "make test";
}
