{
  meta = {
    description = "run a script";
    longDescription = ''
      set -e
      for f in *; do echo "$f"; done
    '';
  };
  environment.etc."motd".text = ''
    echo this is a message of the day, not a script
  '';
  name = "script";
  "script".enable = true;
}
