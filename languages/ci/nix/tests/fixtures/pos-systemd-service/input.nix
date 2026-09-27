{ pkgs, ... }:
{
  systemd.services.backup = {
    description = "nightly backup of /home";
    preStart = ''
      mkdir -p /var/backup
    '';
    script = ''
      tar czf /var/backup/home.tgz /home
      ${pkgs.coreutils}/bin/sync
    '';
    postStart = "echo started";
  };
}
