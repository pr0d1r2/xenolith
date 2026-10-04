{ pkgs }:
pkgs.mkShell {
  packages = [ pkgs.cargo ];
  shellHook = ''
    export RUST_BACKTRACE=1
    echo "entered dev shell"
  '';
}
