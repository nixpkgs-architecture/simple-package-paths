---
feature: simple-package-paths
start-date: 2022-09-02
author: Silvan Mosberger
co-authors: (find a buddy later to help out with the RFC)
shepherd-team: (names, to be nominated and accepted by RFC steering committee)
shepherd-leader: (name to be appointed by RFC steering committee)
related-issues: (will contain links to implementation PRs)
---

# Summary
[summary]: #summary

Make trivial top-level attribute definitions in `pkgs/top-level/all-packages.nix` be auto-generated from a predictable attribute-based file hierarchy.
This makes it much easier to contribute new packages packages, since there's no more guessing needed as to where the package should go, both in the ad-hoc directory categories and in `pkgs/top-level/all-packages.nix`.


# Motivation
[motivation]: #motivation

- (Especially new) package contributors are having a hard time figuring out which files to add or edit. These are very common questions:
  - Which directory should my package definition go in?
  - What are all the categories and do they matter?
  - What if the package has multiple matching categories?
  - Why can't I build my package after adding the package file?
  - Where in all-packages.nix should my package go?
- Figuring out where an attribute is defined is a bit tricky:
  - First one has to find the definition of it in all-packages.nix to see what file it refers to
    - On GitHub this is even more problematic, as the `all-packages.nix` file is [too big to be displayed by GitHub](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/top-level/all-packages.nix)
  - Then go to that file's definition, which takes quite some time for navigation (unless you have a plugin that can jump to it directly)
- `all-packages.nix` frequently causes merge conflicts. It's a point of contention for all new packages

# Detailed design
[design]: #detailed-design

For all attributes at the root of nixpkgs `pkgs.<name>` which:
1. Are defined in `pkgs/top-level/all-packages.nix`
  (necessary so that the overlay containing the automatically discovered packages can be ordered directly before the `all-packages.nix` overlay without changing any behavior)
2. Are defined to be equal to `pkgs.callPackage <path> { }` such that all transitively referenced paths from the default Nix file of `<path>`:
  1. Are under the directory of `<path>`
    (necessary so that moving these files to a new directory doesn't break references in this package)
  2. Are not referenced from any paths outside of these transitive references
    (necessary so that moving these files to a new directory doesn't break references in other packages)
3. Evaluate to a derivation
  (necessary because using `pkg-fun.nix` for a non-package would be counter-intuitive)

These will be become eligible to be transformed as follows:
- Move the default Nix file from `<path>` to `pkgs/unit/<4-prefix name>/<name>/pkg-fun.nix` ([TODO: Justify why `unit`](https://github.com/nixpkgs-architecture/simple-package-paths/issues/16))
  - Where `<4-prefix name>` is the 4-letter prefix of `<name>`.
    If `<name>` has less than 4 characters, append `-` to the `<name>` until it's 4 characters long and use that as `<4-prefix name>`
- Additionally also move all paths transitively referenced by the default Nix file to `pkgs/unit/<4-prefix name>/<name>/???` [TODO](https://github.com/nixpkgs-architecture/simple-package-paths/issues/19)
- Remove the definition of that attribute in `pkgs/top-level/all-packages.nix`
- For each moved path, create a compatibility layer from the old to the new path, potentially using a symlink. See [compatibility] for more details

These attributes will newly be added to `pkgs` by automatically calling `pkgs.callPackage pkgs/unit/<4-prefix name>/<name>/pkg-fun.nix { }` on all entries in `pkgs/unit`. In order to make this more efficient, `builtins.readDir` should be optimized as described [here](https://github.com/NixOS/nix/issues/7314).

## Compatibility layer
[compatibility]: #compatibility-layer

TODO: Nix files should use `import` to act like a symlink while also giving a warning with `builtins.trace`. Something like

```nix
builtins.trace "warning: Using deprecated path ${./.}, use pkgs/unit/<name> instead, this will be removed after NixOS 22.05"
  (import ../../pkgs/unit/name)
```


When moving `pkgs/some/dir/default.nix` to the new `pkgs/unit/<4-prefix name>/<name>/pkg-fun.nix`, a symlink will be created pointing from the old to the new location. Reasoning:
- Current community discussions referencing old files from the `master` branch are still valid for some time. While GitHub doesn't provide an easy way to navigate to a symlink, seeing the path to where the file has moved is better than getting an error.
- It provides an opportunity for code referencing old paths to be updated. While it's not possible to give a deprecation warning with symlinks, users will at least be able to read it in the NixOS release notes. This doesn't occur often in practice.

These symlinks need to be present for at least one NixOS release.

## Transitioning

This RFC makes no requirement as to how the transition should happen, but here are some possible ways:
- Rip-off-the-bandaid approach: Do a big transition, causing a lot of merge conflicts with existing PR's creating and updating packages. Inform PR authors on how to progress
- Incremental CI approach: Introduce a CI action that complains when new packages (and maybe updated packages as well) use the old package paths
- Transparent and sneaky approach: Transition only packages that can be done without causing merge conflicts for all existing PRs. Repeat every once in a while until all packages are done

# Examples
[examples]: #examples

- `pkgs.hello` matches all criteria:
  The default Nix file [`pkgs/applications/misc/hello/default.nix`](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/applications/misc/hello/default.nix) only transitively [references `test.nix`](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/applications/misc/hello/default.nix#L31) in the same directory.
  Neither the `default.nix` nor `test.nix` is referenced by any other file in nixpkgs, so we can do the transformation:
  - Move `pkgs/applications/misc/hello/default.nix` to `pkgs/unit/hell/hello/pkg-fun.nix`
  - Move `pkgs/applications/misc/hello/test.nix` to `pkgs/unit/hell/hello/???` [TODO](https://github.com/nixpkgs-architecture/simple-package-paths/issues/19)
- `pkgs.gnumake` matches all criteria:
  The default Nix file [`pkgs/development/tools/build-managers/gnumake/default.nix`](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/development/tools/build-managers/gnumake/default.nix) transitively references only files in its own directory and no other files in nixpkgs reference `gnumake`'s files, so we can do the transformation by moving all the files from `pkgs/development/tools/build-managers/gnumake` to `pkgs/unit/gnum/gnumake`, the default Nix file ending up in `pkgs/unit/gnum/gnumake/pkg-fun.nix`.
- Similarly `pkgs.gnumake42` in [`pkgs/development/tools/build-managers/gnumake/4.2/default.nix`](https://github.com/NixOS/nixpkgs/tree/nixos-22.05/pkgs/development/tools/build-managers/gnumake/4.2/default.nix) fulfils all criteria, even though its directory is nested in `pkgs.gnumake`'s directory, they don't reference each others files.
- `pkgs.zsh` matches all criteria, its default Nix file is moved to `pkgs/unit/zsh-/zsh/pkg-fun.nix`
- `pkgs.emptyFile` doesn't fulfil criteria 1 (it's defined in `pkgs/build-support/trivial-builders.nix`) and 2 (it's defined inline), so it can't be moved into `pkgs/unit`
- `pkgs.fetchFromGitHub` doesn't fulfil the criteria 3 (it evaluates to a function), so it can't be moved into `pkgs/unit`

Here's how such a `pkgs/unit` directory structure would look like, note how all attribute names have the same level of nesting:
```
pkgs
└── unit
   ├── acpi
   │  ├── acpi
   │  ├── acpica-tools
   │  ├── acpid
   │  ┊
   ┊
   ├── auto
   │  ├── autossh
   │  ├── automirror
   │  ├── autosuspend
   │  ┊
   ┊
   ├── sl--
   │  └── sl
   ┊
   ├── slac
   │  ├── slack
   │  ├── slack-cli
   │  └── slack-term
   ┊
   ├── zsh-
   │  ├── zsh
   │  ├── zsh-autocomplete
   │  ├── zsh-completions
   ┊  ┊
```

# Interactions
[interactions]: #interactions

- `nix edit` is unaffected, since it uses a packages `meta.position` to get the file to edit

# Drawbacks
[drawbacks]: #drawbacks

- The existing categorization of packages gets lost. Counter-arguments:
  - It was never that useful to begin with
    - The categorization was always incomplete, because packages defined in the language package sets often don't get their own categorized file path.
    - It was an inconvenient user interface, requiring a checkout or browsing through GitHub
    - Many packages fit multiple categories, leading to multiple locations to search through instead of one
  - There's other better ways of discovering similar packages, e.g. [Repology](https://repology.org/)
- Creating [symlinks](#symlinks) for the old paths has the potential to create merge conflicts between the symlink and the changed original file there. If the symlink wasn't there, GitHub could perform auto-merging. Counter-arguments:
  - There's not too many such merge conflicts
- This breaks `builtins.unsafeGetAttrPos "hello" pkgs`. Counter-arguments:
  - This functionality is unsafe and therefore breakages can be expected
  - Support for this can be added to Nix (make `builtins.readDir` propagate file as a position)

# Alternatives
[alternatives]: #alternatives

- Use a flat directory `pkgs/unit/*/pkg-fun.nix` instead, arguments:
  - Good because it speeds up Nix evaluation since there's only a single directory to call `builtins.readDir` on instead of many
    - With an optimized `readDir` this isn't much of a problem
  - Good because it's simpler, both for the user and for the code
  - Bad because it causes GitHub to limit the rendering of that directory to 1'000 entries (and we have about 10'000 that benefit from this transition for a start)
  - Bad because it makes `git` slower ([TODO: By how much?](https://github.com/nixpkgs-architecture/simple-package-paths/issues/18))

- Don't use `pkg-fun.nix` but another file name:
  - `package.nix`/`pkg.nix`: Makes the migration to a non-function form of overridable packages harder in the future.
  - `default.nix`:
    - Doesn't have its main benefits in this case:
      - Removing the need to specify the file name in expressions, but this does not apply because this file will be imported automatically by the code that replaces definitions from `all-packages.nix`.
      - Removing the need to specify the file name on the command line, but this does not apply because a package function must be imported into an expression before it can be used, making `nix build -f pkgs/unit/hell/hello` equally broken regardless of file name.
    - Not using `default.nix` frees up `default.nix` for a short expression that is actually buildable, e.g. `(import ../..).hello`.
    - Choosing `default.nix` would bias the purpose of the `unit` directory to serve only as package definitions, whereas we could make the tree more human friendly by grouping files together by "topic" rather than by technical delineations.
      For instance, having a package definition, changelog, package-specific config generator and perhaps even NixOS module in one directory makes work on the package in a broad sense easier.
      This is not a goal of this RFC, but a motivation to make this a future possibility.

 - Use `unit` (at the nixpkgs root) instead of `pkgs/unit`.
   This is future proof in case we want to make the directory structure more general purpose, but this is out of scope

# Unresolved questions
[unresolved]: #unresolved-questions

# Future work
[future]: #future-work

- This RFC only addresses the top-level attribute namespace, aka packages in `pkgs.<name>`, it doesn't do anything about package sets like `pkgs.python3Packages.<name>`, `pkgs.haskell.packages.ghc942.<name>`, which could also benefit from a similar auto-calling
- While this RFC doesn't address expressions where the second `callPackage` argument isn't `{}`, there is an easy way to transition to an argument of `{}`: For every attribute of the form `name = attrs.value;` in the argument, make sure `attrs` is in the arguments of the file, then add `name ? attrs.value` to the arguments. Then the expression in `all-packages.nix` can too be auto-called
  - Don't do this for `name = value` pairs though, that's an alias-like thing
- What to do with different versions, e.g. `wlroots = wlroots_0_14`? This goes into version resolution, a different problem to fix
- What to do about e.g. `libsForQt5.callPackage`? This goes into overrides, a different problem to fix
- What about aliases like `jami-daemon = jami.jami-daemon`?
- What about `recurseIntoAttrs`? Not single packages, package sets, another problem
