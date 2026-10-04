{ stdenv }:
{
  demo = stdenv.mkDerivation {
    pname = "demo";
    version = "1.0";
    src = ./.;
    preCheck = ''
      export HOME=$TMPDIR
      mkdir -p "$HOME/.cache"
      patchShebangs tests
    '';
    postInstall = "installManPage demo.1";
  };
  systemd.services.demo.postStop = ''
    rm -f /run/demo.pid
  '';
}
