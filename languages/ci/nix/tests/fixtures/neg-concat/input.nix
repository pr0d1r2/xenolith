{ lib, extra }:
let
  shellHook = ''
    [ -f .env ] || cp .env.example .env
  ''
  + extra;
in
{
  description = "set -e; " + "make && make install";
  hooks = shellHook + "";
  flag = lib.optionalString extra ("make && " + "make install");
}
