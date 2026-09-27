{ pkgs }:
{
  config = pkgs.writeText "app.conf" ''
    listen 8080
    for x in a b; do echo $x; done
  '';
  partial = pkgs.writeShellScript "only-the-name";
  app = pkgs.writeShellApplication {
    name = "app";
    runtimeInputs = [ ];
  };
  wrapped = pkgs.writeShellApplication (lib.id { text = "not the builder's own argument set"; });
}
