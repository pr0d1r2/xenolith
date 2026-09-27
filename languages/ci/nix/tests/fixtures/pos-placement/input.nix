{ pkgs, ... }:
{
  systemd.services.web-app.script = ''
    echo one
    echo two
  '';
  systemd.services.web.serviceConfig.ExecStartPre = "${pkgs.coreutils}/bin/true";
  packages.deploy = pkgs.writeShellApplication {
    name = "deploy";
    text = ''
      rsync -a ./public/ "$1"
    '';
  };
  environment.etc."X11/xinit/xinitrc".text = ''
    #!/bin/sh
    exec i3
  '';
  devShells.default = pkgs.mkShell {
    shellHook = ''
      export A=1
    '';
  };
  greet = pkgs.writeShellScript "greet" ''
    echo hi
  '';
}
