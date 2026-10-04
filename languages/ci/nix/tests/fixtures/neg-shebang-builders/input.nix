{ pkgs, extra }:
{
  motd = pkgs.writeText "motd" ''
    Welcome. Run `make && make install` to begin.
  '';
  tcl = pkgs.writeScript "tcl" ''
    #!/usr/bin/env tclsh
    puts hello
  '';
  runtime = pkgs.writeScript "runtime" ''
    #!${pkgs.runtimeShell}
    echo interpreter hidden in a hole
  '';
  joined = pkgs.writeText "joined" (
    ''
      #!/bin/sh
      echo only part of the file
    ''
    + extra
  );
  dir = pkgs.writeTextDir "bin/x" ''
    #!/bin/sh
    echo not one of the two builders
  '';
}
