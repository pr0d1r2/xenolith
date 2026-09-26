{
  pkgs,
  lib,
  debug,
}:
{
  systemd.services.hello.script = ''
    ${pkgs.hello}/bin/hello --greeting "''${GREETING:-hi}"
    ${lib.optionalString debug "set -x"}
  '';
}
