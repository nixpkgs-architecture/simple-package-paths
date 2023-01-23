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

This RFC establishes the convention of `pkgs/unit/${toLower (substring 0 4 name)}/${name}` "unit" directories for the definitions of the Nix packages `pkgs.${name}` in nixpkgs.
The `pkg-fun.nix` files in all unit directories are automatically discovered, called using `pkgs.callPackage` and added to the `pkgs` set.

These requirements will be checked using CI:
1. The `pkgs/unit` directory must only contain unit directories, and only in subdirectories of the form `${substring 0 4 name}/${name}`.
2. <a id="req-ref-out"/> Files inside a unit directory must not reference files outside that unit directory.
3. <a id="req-ref-in"/> Files outside a unit directory must not reference files inside a unit directory, except for definitions of attributes in `all-packages.nix` and the auto-calling logic.
4. The definition of a package in the unit directory is the one `pkgs.<name>` points to.
5. To avoid problems with merges, if a package attribute is defined by a unit directory, an attribute of the same name in `pkgs/top-level/all-packages.nix` (or `pkgs/top-level/aliases.nix`) must not redefine it in terms of something other than the unit.

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

- `nix edit` and search.nixos.org are unaffected, since they rely on `meta.position` to get the file to edit, which still works
- `git blame` locally and on GitHub is unaffected, since it follows file renames properly.
- A commonly recommended way of building package directories in nixpkgs is to use `nix-build -E 'with import <nixpkgs> {}; callPackage pkgs/applications/misc/hello {}'`.
  Since the path changes `pkg-fun.nix` is now used, this becomes like `nix-build -E 'with import <nixpkgs> {}; callPackage pkgs/unit/he/hello/pkg-fun.nix {}'`, which is harder for users.
  However, calling a path like this is an anti-pattern anyways, because it doesn't use the correct nixpkgs version and it doesn't use the correct argument overrides.
  The correct way of doing it was to add the package to `pkgs/top-level/all-packages.nix`, then calling `nix-build -A hello`.
  This `nix-build -E` workaround is partially motivated by the difficulty of knowing the mapping from attributes to package paths, which is what this RFC improves upon.
  By teaching users that `pkgs/unit/*/<name>` corresponds to `nix-build -A <name>`, the need for such `nix-build -E` workarounds should disappear.

# Drawbacks
[drawbacks]: #drawbacks

- The existing categorization of packages gets lost. Counter-arguments:
  - It was never that useful to begin with
    - The categorization was always incomplete, because packages defined in the language package sets often don't get their own categorized file path.
    - It was an inconvenient user interface, requiring a checkout or browsing through GitHub
    - Many packages fit multiple categories, leading to multiple locations to search through instead of one
  - There's other better ways of discovering similar packages, e.g. [Repology](https://repology.org/)
- This breaks `builtins.unsafeGetAttrPos "hello" pkgs`. Counter-arguments:
  - We have to draw a line as to what constitutes the public interface of Nixpkgs. We have decided that making attribute position information part of that is not productive. For context, this information is already accepted to be unreliable at the language level, noting the `unsafe` part of the name.
  - Support for this could be added to Nix (make `builtins.readDir` propagate file as a position)

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
  - Not using `default.nix` frees up `default.nix` for a short expression that is actually buildable, e.g. `(import ../..).hello`, although at that point it might better be auto-generated or implicit in the CLI
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

## Restrict design to try delay issues like "package variants" {#no-variants}

We perceived some uncertainty around [package variants](#def-package-variant) that led us to scope these out at first, but we did not identify a real problem that would arise from allowing non-auto-called attributes to reference `pkgs/unit` files. However, imposing unnecessary restrictions would be counterproductive because:

 - The contributor experience would suffer, because it won't be obvious to everyone whether their package is allowed to go into `pkgs/unit`. This means that we'd fail to solve the goal "Which directory should my package definition go in?", leading to unnecessary requests for changes in pull requests.

 - Changes in dependencies can require dependents to add an override, causing packages to be moved back and forth between unit directories and the general `pkgs` tree, worsening the problem as people have to decide categories *again*.

 - When lifting the restriction, the reviewers have to adapt, again leading to unnecessary requests for changes in pull requests.
 
 - We'd be protracting the migration by unnecessary gatekeeping or discovering some problem late.

That said, we did identify risks:

 - We might get something wrong, and while we plan to incrementally migrate Nixpkgs to this new system anyway, starting with fewer units is good.
    - Mitigation: only automate the renames of simple (`callPackage path { }`) calls, to keep the initial change small
 
 - We might not focus enough on the foundation, while we could more easily relax requirements later.
    - After more discussion, we feel confident that the manual `callPackage` calls are unlikely to cause issues that we wouldn't otherwise have.

# Recommend a `callPackage` pattern with default arguments

> - While this RFC doesn't address expressions where the second `callPackage` argument isn't `{}`, there is an easy way to transition to an argument of `{}`: For every attribute of the form `name = attrs.value;` in the argument, make sure `attrs` is in the arguments of the file, then add `name ? attrs.value` to the arguments. Then the expression in `all-packages.nix` can too be auto-called
>   - Don't do this for `name = value` pairs though, that's an alias-like thing

`callPackage` does not favor the default argument when both a default argument and a value in `pkgs` exist. Changing the semantics of `callPackage` is out of scope.

# Allow `callPackage` arguments to be specified in `<unit>/args.nix`

The idea was to expand the auto-calling logic according to:

Unit directories are automatically discovered and transformed to a definition of the form
```
# If args.nix doesn't exist
pkgs.${name} = pkgs.callPackage ${unitDir}/pkg-fun.nix {}
# If args.nix does exists
pkgs.${name} = pkgs.callPackage ${unitDir}/pkg-fun.nix (import ${unitDir}/args.nix pkgs);
```

Pro:
 - It makes another class of packages uniform, by picking a solution with restricted expressive power.

Con:
 - It does not solve the contributor experience problem of having to many rules.
 - `args.nix` is another pattern that contributors need to learn how to use, as we have seen that it is not immediately obvious to everyone how it works.
 - A CI check can mitigate the possible lack of uniformity, and we see a simple implementation strategy for it.
 - This keeps the contents of the unit directories simple and a bit more uniform than with optional `args.nix` files.

# Unresolved questions
[unresolved]: #unresolved-questions

# Future work
[future]: #future-work

All of these questions are in scope to be addressed in future discussions in the [Nixpkgs Architecture Team](https://nixos.org/community/teams/nixpkgs-architecture.html):

- Making the filetree more human-friendly by grouping files together by "topic" rather than technical delineations.
  For instance, having a package definition, changelog, package-specific config generator and perhaps even NixOS module in one directory makes work on the package in a broad sense easier.
- This RFC only addresses the top-level attribute namespace, aka packages in `pkgs.<name>`, it doesn't do anything about package sets like `pkgs.python3Packages.<name>`, `pkgs.haskell.packages.ghc942.<name>`, which may or may not also benefit from a similar auto-calling
- Improve the semantics of `callPackage` and/or apply a better solution, such as a module-like solution
- What to do with different versions, e.g. `wlroots = wlroots_0_14`? This goes into version resolution, a different problem to fix
- What to do about e.g. `libsForQt5.callPackage`? This goes into overrides, a different problem to fix
- What about aliases like `jami-daemon = jami.jami-daemon`?
- What about `recurseIntoAttrs`? Not single packages, package sets, another problem

# Definitions

 - <a id="def-variant-attribute"/> *variant attribute*: an attribute that defines a package by invoking it with non-default arguments, for example:
   ```
     graphviz_nox = callPackage ../tools/graphics/graphviz { withXorg = false; };
   ```
