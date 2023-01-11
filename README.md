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

Auto-generate trivial top-level attribute definitions in `pkgs/top-level/all-packages.nix`  from a sharded directory that matches the attribute name.
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
  - `nix edit -f . package-attr` works, though that's not yet stable (it relies on the `nix-command` feature being enabled) and doesn't work with packages that don't set `meta.position` correctly).
- `all-packages.nix` frequently causes merge conflicts. It's a point of contention for all new packages

# Detailed design
[design]: #detailed-design

This RFC establishes the convention of `pkgs/unit/${substring 0 4 name}/${name}` "unit" directories for the definitions of the Nix packages `pkgs.${name}` in nixpkgs.
The `pkg-fun.nix` files in all unit directories are automatically discovered, called using `pkgs.callPackage` and added to the `pkgs` set.

These requirements will be checked using CI:
1. The `pkgs/unit` directory must only contain unit directories, and only in subdirectories of the form `${substring 0 4 name}/${name}`.
2. <a id="req-ref"/> Files outside a unit directory must not reference files inside that unit directory, and the other way around.
4. The definition of a package in the unit directory is the one `pkgs.<name>` points to.
5. To avoid problems with merges, if a package attribute is defined by a unit directory, it must not be defined in `pkgs/top-level/all-packages.nix` or `pkgs/top-level/aliases.nix`.

This convention is optional, but it will be applied to all existing packages where possible. Nixpkgs reviewers may encourage contributors to use this convention without enforcing it.

## Examples
[examples]: #examples

To add a new package `pkgs.foobar` to nixpkgs, one only needs to create the file `pkgs/unit/foob/foobar/pkg-fun.nix`.
No need to find an appropriate category nor to modify `pkgs/top-level/all-packages.nix` anymore.

With many packages, the `pkgs/unit` directory may look like this:

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

## Alternate `pkgs/unit` structure

- Use a flat directory, e.g. `pkgs.hello` would be in `pkgs/unit/hello`.
  - Good because it's simpler, both for the user and for the code
  - Good because it speeds up Nix evaluation since there's only a single directory to call `builtins.readDir` on instead of many
    - With an optimized `readDir` this isn't much of a problem
  - Bad because the GitHub web interface only renders the first 1'000 entries (and we have about 10'000 that benefit from this transition, even given the restrictions)
  - Bad because it makes `git` slower ([TODO: By how much?](https://github.com/nixpkgs-architecture/simple-package-paths/issues/18))
  - Bad because directory listing slows down with many files
- Use only the 1-, 2- or 3-prefix instead of the 4-prefix name. This was not done because it still leads to directories in `pkgs/unit` containing more than 1'000 entries, leading to the same problems.
- Use multi-level structure, like a 2-level 2-prefix structure where `hello` is in `pkgs/unit/he/ll/hello`,
  if packages are less than 4 characters long, we will it out with `-`, e.g. `z` is in `pkgs/unit/z-/--/z`.
  This is not great because it's more complicated and it would improve git performance only marginally.
- Use a dynamic structure where directories are rebalanced when they have too many entries.
  E.g. `pkgs.foobar` could be in `pkgs/unit/f/foobar` initially.
  But when there's more than 1'000 packages starting with `f`, all packages starting with `f` are distributed under 2-letter prefixes, moving `foobar` to `pkgs/unit/fo/foobar`.
  This is not great because it's very complex to determine which directory to put a package in, making it bad for contributors.

## Alternate `pkg-fun.nix` filename

- `default.nix`: Bad because:
  - Doesn't have its main benefits in this case:
    - Removing the need to specify the file name in expressions, but this does not apply because this file will be imported automatically by the code that replaces definitions from `all-packages.nix`.
    - Removing the need to specify the file name on the command line, but this does not apply because a package function must be imported into an expression before it can be used, making `nix build -f pkgs/unit/hell/hello` equally broken regardless of file name.
  - Not using `default.nix` frees up `default.nix` for a short expression that is actually buildable, e.g. `(import ../..).hello`.
  - Choosing `default.nix` would bias the purpose of the `unit` directory to serve only as package definitions, whereas we could make the tree more human friendly by grouping files together by "topic" rather than by technical delineations.
    For instance, having a package definition, changelog, package-specific config generator and perhaps even NixOS module in one directory makes work on the package in a broad sense easier.
    This is not a goal of this RFC, but a motivation to make this a future possibility.
- `package.nix`/`pkg.nix`: Bad, because it makes the migration to a non-function form of overridable packages harder in the future.

## Alternate `pkgs/unit` location

- Use `unit` (at the nixpkgs root) instead of `pkgs/unit`.
  This is future proof in case we want to make the directory structure more general purpose, but this is out of scope
- Other name proposals were deemed worse: `pkg`, `component`, `part`, `mod`, `comp`

## Filepath backwards-compatibility

Additionally have a backwards-compatibility layer for moved paths, such as a symlink pointing from the old to the new location, or for Nix files even a `builtins.trace "deprecated" (import ../new/path)`.
- We are not doing this because it would give precedent to file paths being a stable API interface, which definitely shouldn't be the case (bar some exceptions).
- It would also lead to worse merge conflicts as the transition is happening, since Git would have to resolve a merge conflict between a symlink and a changed file.

## Not having the [reference requirement](#user-content-req-ref)

The reference requirement could be removed, which would allow unit directories to reference files outside themselves, and the other way around. This is not great because it encourages the use of file paths as an API, rather than explicitly exposing functionality from Nix expressions.

## Relax design to try to attack issues like "package variants" up front

An issue with restrictions like the above one is that they don't work well for when we package a number of variants of package, e.g. different versions of the same package that share some infra. We do presume we would have to have *some* notion of "private details shared between multiple units" or "multiple entry points to unit" to handle these cases.

We've chosen to explicitly ignore these tough cases, and emphasize uniform structure of units over being able to migrate over as many packages as possible from the get go. The rationale for this decision is basically:

1. It is (a bit) easier to relax requirements later than tighten them later.
2. We plan on incrementally migrating Nixpkgs to this new system anyways, for caution's sake, so starting with fewer units is not only fine but *good*.
3. Explicitly marking use-cases out of scope allows us to have a more focused and thorough investigation of the use-cases that remain, to build a solid foundation.

# Unresolved questions
[unresolved]: #unresolved-questions

# Future work
[future]: #future-work

All of these questions are in scope to be addressed in future discussions in the [Nixpkgs Architecture Team](https://nixos.org/community/teams/nixpkgs-architecture.html):

- This RFC only addresses the top-level attribute namespace, aka packages in `pkgs.<name>`, it doesn't do anything about package sets like `pkgs.python3Packages.<name>`, `pkgs.haskell.packages.ghc942.<name>`, which may or may not also benefit from a similar auto-calling
- While this RFC doesn't address expressions where the second `callPackage` argument isn't `{}`, there is an easy way to transition to an argument of `{}`: For every attribute of the form `name = attrs.value;` in the argument, make sure `attrs` is in the arguments of the file, then add `name ? attrs.value` to the arguments. Then the expression in `all-packages.nix` can too be auto-called
  - Don't do this for `name = value` pairs though, that's an alias-like thing
- What to do with different versions, e.g. `wlroots = wlroots_0_14`? This goes into version resolution, a different problem to fix
- What to do about e.g. `libsForQt5.callPackage`? This goes into overrides, a different problem to fix
- What about aliases like `jami-daemon = jami.jami-daemon`?
- What about `recurseIntoAttrs`? Not single packages, package sets, another problem
