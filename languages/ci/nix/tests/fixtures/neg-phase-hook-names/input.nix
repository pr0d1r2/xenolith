{ stdenv }:
stdenv.mkDerivation {
  pname = "demo";
  version = "1.0";
  src = ./.;
  prefix = "make && make install";
  preferLocalBuild = true;
  prePhases = "setupPhase && extraPhase";
  postgresql = "run a && run b";
}
