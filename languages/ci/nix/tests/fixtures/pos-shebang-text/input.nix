{ pkgs }:
{
  environment.etc."X11/xinit/xinitrc".text = ''
    #!/bin/sh
    xrdb -merge "$HOME/.Xresources"
    xsetroot -cursor_name left_ptr
    setxkbmap -option ctrl:nocaps
    xset r rate 200 40
    exec i3
  '';
  hooks = pkgs.writeTextFile {
    name = "hook";
    executable = true;
    text = "#!${pkgs.bash}/bin/bash\nexec true\n";
  };
}
