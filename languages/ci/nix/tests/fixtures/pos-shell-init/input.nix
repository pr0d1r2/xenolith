{ lib, config, ... }:
{
  programs.zsh = {
    enable = true;
    initContent = ''
      bindkey -e
      autoload -U compinit && compinit
    '';
  };
  programs.bash = lib.mkIf config.programs.bash.enable {
    bashrcExtra = ''
      export PATH="$HOME/.local/bin:$PATH"
    '';
  };
}
