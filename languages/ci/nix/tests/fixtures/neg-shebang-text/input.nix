{ pkgs }:
let
  launcher = ''
    #!/bin/sh
    exec app "$@"
  '';
in
{
  environment.etc = {
    "motd".text = ''
      Welcome. Run `make && make install` to begin.
    '';
    "notes".text = ''
      first line
      #!/bin/sh
      echo not a shebang here
    '';
    "tcl".text = ''
      #!/usr/bin/env tclsh
      puts hello
    '';
    "runtime".text = ''
      #!${pkgs.runtimeShell}
      echo interpreter hidden in a hole
    '';
  };
  wrapped = pkgs.writeText "w" launcher;
}
