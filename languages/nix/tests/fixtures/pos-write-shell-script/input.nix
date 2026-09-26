{ pkgs }:
{
  greet = pkgs.writeShellScript "greet" ''
    echo "hello $1"
  '';
  tool = with pkgs; writeShellScriptBin "tool" "exec ${hello}/bin/hello";
}
