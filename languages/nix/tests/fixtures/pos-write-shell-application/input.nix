{ pkgs }:
{
  packages.deploy = pkgs.writeShellApplication {
    name = "deploy";
    runtimeInputs = [ pkgs.rsync ];
    text = ''
      rsync -a ./public/ "$1"
    '';
  };
}
