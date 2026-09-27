{ pkgs, ... }:
{
  systemd.services.web.serviceConfig = {
    ExecStart = "${pkgs.nginx}/bin/nginx -g 'daemon off;'";
    ExecStartPre = [
      "${pkgs.coreutils}/bin/mkdir -p /run/web"
      "${pkgs.nginx}/bin/nginx -t"
    ];
    ExecStartPost = "${pkgs.coreutils}/bin/true";
    ExecStartSec = "not a systemd key";
    Restart = "always";
  };
}
