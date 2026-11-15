{
  systemd.services.broken.script = ''
    if true; then
      echo broken
  '';
}
