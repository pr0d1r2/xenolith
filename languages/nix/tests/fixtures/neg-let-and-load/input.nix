let
  script = ''
    echo defined but not yet used anywhere
  '';
in
{
  systemd.services.a.script = builtins.readFile ./a.sh;
  systemd.services.b.script = script;
}
