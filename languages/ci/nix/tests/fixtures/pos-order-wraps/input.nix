{ lib, ... }:
{
  programs.zsh.initContent = lib.mkOrder 550 ''
    [ -r "$HOME/.zshrc.local" ] && source "$HOME/.zshrc.local"
  '';
  systemd.services.app.preStart = lib.mkBefore ''
    mkdir -p /var/lib/app && chown app /var/lib/app
  '';
  devShells.default.shellHook = lib.mkForce ''
    [ -f .env ] || cp .env.example .env
  '';
}
