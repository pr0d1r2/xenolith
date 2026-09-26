{
  pkgs,
  lib,
  extra,
}:
{
  shell = pkgs.mkShell {
    shellHook = ''
      [ -f .env ] || cp .env.example .env
    ''
    + extra
    + "export A=1";
  };
  greet = pkgs.writeShellScript "greet" (
    ''
      echo hi
    ''
    + lib.optionalString true "echo more"
  );
}
