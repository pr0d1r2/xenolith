{ pkgs }:
{
  docs = pkgs.runCommand "docs" { nativeBuildInputs = [ pkgs.pandoc ]; } ''
    mkdir -p $out
    pandoc README.md -o $out/index.html
  '';
  stamp = pkgs.runCommandLocal "stamp" { } "date > $out";
}
