{ pkgs }:
let
  launcher = ''
    #!/bin/sh
    exec app "$@"
  '';
in
{
  environment.etc."motd".text = ''
    Welcome. Run `make && make install` to begin.
  '';
  environment.etc."notes".text = ''
    first line
    #!/bin/sh
    echo not a shebang here
  '';
  environment.etc."tcl".text = ''
    #!/usr/bin/env tclsh
    puts hello
  '';
  environment.etc."runtime".text = ''
    #!${pkgs.runtimeShell}
    echo interpreter hidden in a hole
  '';
  wrapped = pkgs.writeText "w" launcher;
}
