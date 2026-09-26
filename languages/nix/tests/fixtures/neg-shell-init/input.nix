{
  programs.fish.interactiveShellInit = ''
    set -gx EDITOR vim
    fish_vi_key_bindings
  '';
  programs.zsh.shellAliases.ll = "ls -l && echo done";
  programs.bash.historyFile = "$HOME/.bash_history";
  initExtra = ''
    echo a
    echo b
  '';
}
