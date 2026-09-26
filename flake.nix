{
  # xenolith's pinned toolchain. The flake lives at the repo ROOT because a
  # flake's source root is its own directory: one in `nix/` could not see
  # `Cargo.toml` or `src/` and so could offer no package at all. The root file
  # stays a wiring diagram -- inputs, systems, and one import per output --
  # while everything with a body lives in `nix/*.nix` (`nix` §G).
  description = "xenolith -- one language per file: find embeds, extract them, verify the loads";

  # hk is built by `nix-hk` and pushed to this cache. Without the substituter
  # every entry into the dev shell BUILDS hk from source, which is the cost
  # this whole follows-wiring exists to avoid. Declared here rather than in
  # each user's nix.conf so the cache travels with the flake.
  nixConfig = {
    extra-substituters = [ "https://pr0d1r2.cachix.org" ];
    extra-trusted-public-keys = [
      "pr0d1r2.cachix.org-1:NfWjbhgAj41byXhCKiaE+av3Vnphm1fTezHXEGsiQIM="
    ];
  };

  # FIVE declared inputs, ONE nixpkgs (`nix:C6`, `nix:V17`). `nixpkgs-lock` is
  # the fleet's sole nixpkgs authority and every other input follows it, so
  # this repo names no revision of its own and the agreement is a property of
  # the graph rather than of matching strings in five files.
  #
  # The last three are the SPEC toolchain (`scripts:C21`): microlith formats
  # and checks every `SPEC.md`, itok counts its tokens against
  # `.context-limits`, sherd validates the federation. They are dev-shell and
  # guardrail only -- `packages.default` must not carry them (`nix:V29`).
  inputs = {
    nixpkgs-lock.url = "github:pr0d1r2/nixpkgs-lock";
    nixpkgs.follows = "nixpkgs-lock/nixpkgs";

    nix-hk = {
      url = "github:pr0d1r2/nix-hk";
      inputs.nixpkgs-lock.follows = "nixpkgs-lock";
    };

    # itok names its hk input `hk`, microlith and sherd name theirs `nix-hk`.
    # The attribute name is theirs; what matters is that all three resolve to
    # the SAME node here, or the shell would hold three hk builds and two of
    # them would miss the cache.
    itok = {
      url = "github:pr0d1r2/itok";
      inputs = {
        nixpkgs-lock.follows = "nixpkgs-lock";
        hk.follows = "nix-hk";
      };
    };

    # microlith's own itok edge follows ours (`nix:C6`, tool-to-tool edges).
    # itok's microlith edge is deliberately LEFT ALONE: it pins a release tag,
    # which is what terminates the graph -- pointing it back at our microlith
    # while microlith points at our itok is a cycle, and a lock file cannot be
    # computed from one.
    microlith = {
      url = "github:pr0d1r2/microlith";
      inputs = {
        nixpkgs-lock.follows = "nixpkgs-lock";
        nix-hk.follows = "nix-hk";
        itok.follows = "itok";
      };
    };

    sherd = {
      url = "github:pr0d1r2/sherd";
      inputs = {
        nixpkgs-lock.follows = "nixpkgs-lock";
        nix-hk.follows = "nix-hk";
      };
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      nix-hk,
      itok,
      microlith,
      sherd,
      ...
    }:
    let
      # Four systems are DECLARED, three are tier-1 (`nix:C7`). Today the list
      # is the tier-1 three; `x86_64-darwin` joins when something builds for
      # it, because a system nobody builds is a claim nobody checks.
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];

      # The overlay is what makes `pkgs.hk` mean nix-hk's hk rather than
      # nixpkgs' -- nixpkgs trails the releases `hk.pkl` is written against.
      # Applied as an overlay so there is exactly ONE `pkgs` per system: a
      # second lookup path is a second place to forget.
      forAll =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f {
            inherit system;
            pkgs = nixpkgs.legacyPackages.${system}.extend nix-hk.overlays.default;
          }
        );

      # The spec toolchain, resolved per system. Passed as a list rather than
      # looked up inside `nix/devshell.nix` so that file needs no flake inputs
      # of its own and stays a package list.
      specTools =
        system:
        map (flake: flake.packages.${system}.default) [
          itok
          microlith
          sherd
        ];
    in
    {
      # The `xnl` binary, from the committed `Cargo.lock` (`nix` §I, `nix:T26`).
      packages = forAll (
        { pkgs, ... }:
        {
          default = import ./nix/package.nix { inherit pkgs; };
        }
      );

      # test and clippy over the package's own source set, and dogfood: the
      # packaged `xnl check` over the whole tree (`nix/checks.nix`).
      checks = forAll (
        { pkgs, system }:
        import ./nix/checks.nix {
          inherit pkgs;
          package = self.packages.${system}.default;
        }
      );

      devShells = forAll (
        { pkgs, system }:
        {
          default = import ./nix/devshell.nix {
            inherit pkgs;
            specTools = specTools system;
          };
        }
      );
    };
}
