{
  pkgs,
  nix-shebang,
  inputs,
  ...
}:
let
  inherit (builtins) readFile;
in
{
  systemd.services.a.script = builtins.readFile ./a/a-script.sh;
  systemd.services.b.script = nix-shebang.lib.readWithoutStrict ./a/b-script.bash;
  programs.zsh.initContent = inputs.nix-shebang.lib.readWithoutStrict ../zsh/init.zsh;
  hook = pkgs.writeShellScript "hook" (builtins.readFile ./a/hook.sh);
  # builtins.readFile ./a/in-a-comment.sh
  environment.etc = {
    "app.conf".text = builtins.readFile ./a/app.conf;
    bare.text = readFile ./a/bare.sh;
    abs.text = builtins.readFile /etc/abs.sh;
    interp.text = builtins.readFile ./a/${pkgs.system}.sh;
    store.source = "${./a/store.sh}";
    other.text = pkgs.lib.readWithoutStrict ./a/other.sh;
  };
}
