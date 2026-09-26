{ pkgs }:
pkgs.writeShellScript "hello" ''
  echo hello
  echo world
''
