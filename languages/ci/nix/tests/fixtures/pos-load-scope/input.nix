{ pkgs, inputs, ... }:
let
  inherit (inputs) nix-shebang;
in
{
  systemd.services.backup.script = ''
    mkdir -p /var/backup
    tar czf /var/backup/home.tgz /home
  '';
  environment.systemPackages = [
    (nix-shebang.lib.toShellScript {
      inherit pkgs;
      name = "hello";
      src = ./hello.sh;
    })
  ];
  devShells.default = pkgs.mkShell {
    shellHook = ''
      echo entering
      export APP_ENV=dev
    '';
  };
}
