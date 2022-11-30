A simple tool based on [rnix](https://github.com/nix-community/rnix-parser) that takes a single nix file as an argument and prints all paths it references separated by newlines

```
$ nix-build
/nix/store/1clbfcm17x7awgfp7i1qqqri5mj60k49-nix-refs
$ result/bin/nix-refs ~/src/nixpkgs/default.nix
./lib/minver.nix
./nixos/doc/manual/release-notes
./pkgs/top-level/impure.nix
```
