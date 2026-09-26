{ lib, cond, ... }:
{
  services.app.description = lib.mkForce ''
    Runs `make && make install` on boot.
  '';
  environment.etc."xinitrc".text = lib.mkDefault ''
    #!/bin/sh
    exec i3
  '';
  devShells.default.shellHook = lib.mkIf cond ''
    [ -f .env ] || cp .env.example .env
  '';
  systemd.services.app.preStart = lib.mkForce (
    lib.mkBefore ''
      mkdir -p /var/lib/app && chown app /var/lib/app
    ''
  );
}
