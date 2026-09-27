{
  pkgs,
  cfg,
  ...
}:
{
  systemd.services.a.script = ''
    ${pkgs.hello}/bin/hello --name "${cfg.name}"
    ${pkgs.hello}/bin/hello again
    if [ -e ${cfg.dir}/flag ]; then
      echo "''${HOME}" > ${cfg.dir}/out
    fi
  '';
  systemd.services.a.postStart = "mkdir -p ${cfg.dir} && echo \"${cfg.dir}\"";
  shellHook = ''
    cat <<EOF
      ${cfg.banner}
    EOF
    echo '${cfg.name}'
  '';
}
