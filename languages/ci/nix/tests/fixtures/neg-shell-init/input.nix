{
  programs = {
    fish.interactiveShellInit = ''
      set -gx EDITOR vim
      fish_vi_key_bindings
    '';
    zsh.shellAliases.ll = "ls -l && echo done";
    bash.historyFile = "$HOME/.bash_history";
  };
  initExtra = ''
    echo a
    echo b
  '';
}
