{
  pkgs,
  cfg,
  nix-shebang,
  ...
}:
{
  systemd.services.a.script =
    builtins.replaceStrings [ "__HELLO_BIN__" "__DIR__" ] [ "${pkgs.hello}/bin/hello" "${cfg.dir}" ]
      (builtins.readFile ./a/a-script.sh);
  systemd.services.b.script = builtins.replaceStrings [ "__X__" ] [ "${cfg.x}" ] (
    nix-shebang.lib.readWithoutStrict ./a/b-script.sh
  );
  hook = pkgs.writeShellScript "hook" (
    builtins.replaceStrings [ "__X__" ] [ "${cfg.x}" ] (builtins.readFile ./a/hook.sh)
  );
  other.text = builtins.replaceStrings [ "x" ] [ "${cfg.x}" ] (builtins.readFile ./a/other.sh);
  uneven.text = builtins.replaceStrings [ "__X__" "__Y__" ] [ "${cfg.x}" ] (
    builtins.readFile ./a/uneven.sh
  );
  escaped.text = builtins.replaceStrings [ "__X__" ] [ "${cfg.x}\n" ] (
    builtins.readFile ./a/escaped.sh
  );
}
