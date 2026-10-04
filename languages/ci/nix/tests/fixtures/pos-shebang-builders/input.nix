{ pkgs }:
{
  hook = pkgs.writeScript "post-merge" ''
    #!/bin/sh
    git diff --quiet HEAD@{1} -- flake.lock || nix flake check
  '';
  report = pkgs.writeText "report.py" ''
    #!/usr/bin/env python3
    import sys
    print(sys.argv)
  '';
}
