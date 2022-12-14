---
feature: simple-package-paths
start-date: 2022-09-02
author: Silvan Mosberger
co-authors: Nixpkgs Architecture Team
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
    - It also slows down or even deadlocks editors due to the file size
  - In some cases `nix edit` works, though that's not yet stable (it relies on Flakes being enabled) and comes with some problems ([doesn't yet open a writable file](https://github.com/NixOS/nix/issues/3347), doesn't work with packages that don't set `meta.position` correctly).
- `all-packages.nix` frequently causes merge conflicts. It's a point of contention for all new packages

# Detailed design
[design]: #detailed-design

Make a large part of `pkgs.<name>` definitions in `all-packages.nix` eligible to be moved to `pkgs/unit/<4-letter name>/<name>`.
The definition in `all-packages.nix` won't be necessary anymore, as all directories in `pkgs/unit/*/*` are automatically added to the `pkgs` set.

The criteria for `pkgs.<name>` becoming eligible are as follows:
1. <a id="criteria-1"/> Is defined in `pkgs/top-level/all-packages.nix`
  (necessary so that the overlay containing the automatically discovered packages can be ordered directly before the `all-packages.nix` overlay without changing any behavior)
2. <a id="criteria-2"/> Is defined to be equal to `pkgs.callPackage <path> { }`
3. <a id="criteria-3"/> All transitively referenced paths from the default Nix file of `<path>` are under the same directory as the default Nix file and can be moved around together without breaking any references in other Nix files (except the one reference in `pkgs/top-level/all-packages.nix`).
  This means that the Nix code should neither reference code outside, nor be referenced from outside.
  (This is necessary so that no Nix code needs to be updated in the transition below)
4. <a id="criteria-4"/> Evaluates to a derivation
  (necessary because using `pkg-fun.nix` for a non-package would be counter-intuitive)

If all criteria are satisfied, the package becomes eligible for the following changes:
- Move the default Nix file from `<path>` to `pkgs/unit/<4-prefix name>/<name>/pkg-fun.nix`
  - Where `<4-prefix name>` is the 4-letter prefix of `<name>`, equal to `substring 0 4 name`.
    If `<name>` has less than or exactly 4 characters, `<4-prefix name>` is equal to just `<name>`.
  - The directory `unit` [was chosen](https://github.com/nixpkgs-architecture/simple-package-paths/issues/16) for a future vision where it could be its own top-level directory, not only containing package definitions for software components, but also related NixOS modules, library components, etc.
- Move all paths transitively referenced by the default Nix file to `pkgs/unit/<4-prefix name>/<name>`
- Remove the definition of that attribute in `pkgs/top-level/all-packages.nix`

These attributes will newly be added to `pkgs` by automatically calling `pkgs.callPackage pkgs/unit/<4-prefix name>/<name>/pkg-fun.nix { }` on all entries in `pkgs/unit`. In order to ensure efficiency of this operation, `builtins.readDir` should be optimized as described [here](https://github.com/NixOS/nix/issues/7314).

## Transitioning

This RFC comes with [a reference tool](https://github.com/nixpkgs-architecture/simple-package-paths/pull/22) to make the above transition in an automated way.
If this RFC is accepted, the result of that tool will be PR'd to nixpkgs.
The tool itself will also be added to nixpkgs so that it can easily be ran again in the future.
For at least one release cycle, the legacy way of declaring packages should still be accepted, but the tool can be ran again at any point, thereby moving those new packages from the legacy paths to the new `pkgs/unit` paths.
A CI action may also be implemented to help with this if deemed necessary.

# Examples
[examples]: #examples

- `pkgs.hello` matches all criteria:
  The default Nix file [`pkgs/applications/misc/hello/default.nix`](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/applications/misc/hello/default.nix) only transitively [references `test.nix`](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/applications/misc/hello/default.nix#L31) in the same directory.
  Neither the `default.nix` nor `test.nix` is referenced by any other file in nixpkgs, so we can do the transformation:
  - Move `pkgs/applications/misc/hello/default.nix` to `pkgs/unit/hell/hello/pkg-fun.nix`
  - Move `pkgs/applications/misc/hello/test.nix` to `pkgs/unit/hell/hello/test.nix`
- `pkgs.gnumake` matches all criteria:
  The default Nix file [`pkgs/development/tools/build-managers/gnumake/default.nix`](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/development/tools/build-managers/gnumake/default.nix) transitively references only files in its own directory and no other files in nixpkgs reference `gnumake`'s files, so we can do the transformation by moving all the files from `pkgs/development/tools/build-managers/gnumake` to `pkgs/unit/gnum/gnumake`, the default Nix file ending up in `pkgs/unit/gnum/gnumake/pkg-fun.nix`.
- Similarly `pkgs.gnumake42` in [`pkgs/development/tools/build-managers/gnumake/4.2/default.nix`](https://github.com/NixOS/nixpkgs/tree/nixos-22.05/pkgs/development/tools/build-managers/gnumake/4.2/default.nix) fulfils all criteria, even though its directory is nested in `pkgs.gnumake`'s directory, they don't reference each others files.
- `pkgs.zsh` matches all criteria, its default Nix file is moved to `pkgs/unit/zsh/zsh/pkg-fun.nix`
- `pkgs.emptyFile` doesn't fulfil [criteria 1](#user-content-criteria-1) (it's defined in `pkgs/build-support/trivial-builders.nix`), so it can't be moved into `pkgs/unit`
- `pkgs.readline` doesn't fulfil [criteria 2](#user-content-criteria-2) (it's defined as an alias to `readline6`, which is itself defined as an alias to `readline63`)
- `pkgs.readline63` doesn't fulfil [criteria 3](#user-content-criteria-3) (it [transitively references](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/development/libraries/readline/6.3.nix#L23) `link-against-ncurses.patch`, which is [also referenced](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/development/libraries/readline/7.0.nix#L30) by the definition for `pkgs.readline70`)
- `pkgs.fetchFromGitHub` doesn't fulfil the [criteria 4](#user-content-criteria-4) (it evaluates to a function), so it can't be moved into `pkgs/unit`

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
   ├── sl
   │  └── sl
   ┊
   ├── slac
   │  ├── slack
   │  ├── slack-cli
   │  └── slack-term
   ┊
   ├── zsh
   │  ├── zsh
   │  ├── zsh-autocomplete
   │  ├── zsh-completions
   ┊  ┊
```

# Interactions
[interactions]: #interactions

- `nix edit` is unaffected, since it uses a packages `meta.position` to get the file to edit.
  Though with this RFC `nix edit` could be updated to not have to rely on that anymore for the packages in the new hierarchy in nixpkgs.

# Drawbacks
[drawbacks]: #drawbacks

- The existing categorization of packages gets lost. Counter-arguments:
  - It was never that useful to begin with
    - The categorization was always incomplete, because packages defined in the language package sets often don't get their own categorized file path.
    - It was an inconvenient user interface, requiring a checkout or browsing through GitHub
    - Many packages fit multiple categories, leading to multiple locations to search through instead of one
  - There's other better ways of discovering similar packages, e.g. [Repology](https://repology.org/)
- This breaks `builtins.unsafeGetAttrPos "hello" pkgs`. Counter-arguments:
  - This functionality is unsafe and therefore breakages can be expected
  - Support for this can be added to Nix (make `builtins.readDir` propagate file as a position)

# Alternatives
[alternatives]: #alternatives

- Use a flat directory `pkgs/unit/*/pkg-fun.nix` instead, arguments:
  - Good because it's simpler, both for the user and for the code
  - Good because it speeds up Nix evaluation since there's only a single directory to call `builtins.readDir` on instead of many
    - With an optimized `readDir` this isn't much of a problem
  - Bad because it causes GitHub to limit the rendering of that directory to 1'000 entries (and we have about 10'000 that benefit from this transition for a start)
  - Bad because it makes `git` slower ([TODO: By how much?](https://github.com/nixpkgs-architecture/simple-package-paths/issues/18))
  - Bad because directory listing slows down with many files

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
- Additionally have a backwards-compatibility layer for moved paths, such as a symlink pointing from the old to the new location, or for Nix files even a `builtins.trace "deprecated" (import ../new/path)`.
  We are not doing this because it would give precedent to file paths being a stable API interface, which definitely shouldn't be the case (bar some exceptions).
  It would also lead to worse merge conflicts as the transition is happening, since Git would have to resolve a merge conflict between a symlink and a changed file.
- Loosen [criteria 3](#user-content-criteria-3), allowing certain packages to be moved to the new structure even if it requires updating references to paths in Nix files.
  This isn't done because it [turns out](https://github.com/nixpkgs-architecture/simple-package-paths/issues/14) that this criteria indicates the file structure being used as an API interface.
  By manually refactoring the Nix code to not rely on this anymore, you can increase code quality/reusability/clarity and then do the transition described in the RFC.
- Use a different sharding scheme than `<4-prefix name>`.
  Discussions regarding this can be seen [here](https://github.com/nixpkgs-architecture/simple-package-paths/issues/1), [NAT meeting #18](https://github.com/nixpkgs-architecture/meetings/blob/6282b0c6bbc47b6f1becd155586c79728eddefc9/2022-11-21.md) and [here](https://github.com/nixpkgs-architecture/simple-package-paths/pull/20#discussion_r1029004083)

# Unresolved questions
[unresolved]: #unresolved-questions

# Future work
[future]: #future-work

All of these questions are in scope to be addressed in future discussions in the [Nixpkgs Architecture Team](https://nixos.org/community/teams/nixpkgs-architecture.html):

- This RFC only addresses the top-level attribute namespace, aka packages in `pkgs.<name>`, it doesn't do anything about package sets like `pkgs.python3Packages.<name>`, `pkgs.haskell.packages.ghc942.<name>`, which could also benefit from a similar auto-calling
- While this RFC doesn't address expressions where the second `callPackage` argument isn't `{}`, there is an easy way to transition to an argument of `{}`: For every attribute of the form `name = attrs.value;` in the argument, make sure `attrs` is in the arguments of the file, then add `name ? attrs.value` to the arguments. Then the expression in `all-packages.nix` can too be auto-called
  - Don't do this for `name = value` pairs though, that's an alias-like thing
- What to do with different versions, e.g. `wlroots = wlroots_0_14`? This goes into version resolution, a different problem to fix
- What to do about e.g. `libsForQt5.callPackage`? This goes into overrides, a different problem to fix
- What about aliases like `jami-daemon = jami.jami-daemon`?
- What about `recurseIntoAttrs`? Not single packages, package sets, another problem
