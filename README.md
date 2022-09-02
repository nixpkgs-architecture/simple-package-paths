---
feature: auto-called-packages
start-date: 2022-09-02
author: Silvan Mosberger
co-authors: (find a buddy later to help out with the RFC)
shepherd-team: (names, to be nominated and accepted by RFC steering committee)
shepherd-leader: (name to be appointed by RFC steering committee)
related-issues: (will contain links to implementation PRs)
---

# Summary
[summary]: #summary

Make most attribute definitions in `pkgs/top-level/all-packages.nix` be auto-generated from a single flat directory instead, where the subdirectory corresponds to the attribute name. The ad-hoc category-based structure of packages will be gotten rid of.

# Motivation
[motivation]: #motivation

- (Especially new) package contributors are having a hard time figuring out which files to add and edit. These are very common questions:
  - Which directory should my package definition go in? What are all the categories and do they matter? What if the package has multiple matching categories?
  - Why can't I build my package after adding the package file? [introduced to all-packages.nix] Where in all-packages.nix should my package go?
- Figuring out where an attribute is defined is very tricky:
  - First one has to find the definition of it in all-packages.nix to see what file it refers to
    - Especially on GitHub this is even more problematic, as the `all-packages.nix` file is [too big to be displayed by GitHub](https://github.com/NixOS/nixpkgs/blob/nixos-22.05/pkgs/top-level/all-packages.nix)
  - Then go to that file's definition, which takes quite some time for navigation (unless you have a plugin that can jump to it directly)
- `all-packages.nix` frequently causes merge conflicts. It's a point of contention for all new packages

# Detailed design
[design]: #detailed-design

All attributes at the root of nixpkgs (`pkgs.<name>`) whose definition is of the form

```nix
{
  <name> = pkgs.callPackage ../some/dir { };
}
```

will be become eligible to be transformed as follows:
- Move `pkgs/some/dir` to `pkgs/auto/<name>`
- Fix any references to that folder (e.g. in update scripts) 
- Remove the original definition

These attributes will newly be defined by listing all directories in `pkgs/auto` using `builtins.readDir` and calling `pkgs.callPackage` on all of them, which then gets added to the `pkgs` scope.

Attributes whose definition aren't of the above form won't be changed, so e.g. the following definition in `all-packages.nix` won't be changed:
```nix
{
  syncplay-nogui = syncplay.override { enableGUI = false; };
}
```

However, if such definitions can be refactored into the above form they will become eligible for the transformation.

## Backwards compatibility symlinks
[symlinks]: #backwards-compatibility-symlinks

When moving `pkgs/some/dir` to the new `pkgs/auto/<name>`, a symlink will be created pointing from `pkgs/some/dir` to `pkgs/auto/<name>`. Reasoning:
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

- `pkgs.hello`: Move from `pkgs/applications/misc/hello` to `pkgs/auto/hello`
- `pkgs.gnumake`: Move from `pkgs/development/tools/build-managers/gnumake` to `pkgs/auto/gnumake`
- `pkgs.gnumake42`: Move from `pkgs/development/tools/build-managers/gnumake/4.2` to `pkgs/auto/gnumake42`
- `pkgs.buildEnv`: Move from `pkgs/build-support/buildenv` to `pkgs/auto/buildEnv`
- `pkgs.fetchFromGitHub`: Move from `pkgs/build-support/fetchgithub` to `pkgs/auto/fetchFromGitHub`

# Interactions
[interactions]: #interactions

- `nix edit` is unaffected, since it uses a packages `meta.position` to get the file to edit

# Drawbacks
[drawbacks]: #drawbacks

- The existing categorization of packages gets lost. Counter-arguments:
  - It was never that useful to begin with
  - There's other better ways of discovering similar packages, e.g. [Repology](https://repology.org/)
- GitHub's UI can't display more than 1000 items in a directory. Counter-arguments:
  - It's still possible to open a subdirectory using the "Go to file" button, or pressing the `T` key
- Creating [symlinks](#symlinks) for the old paths has the potential to create merge conflicts between the symlink and the changed original file there. If the symlink wasn't there, GitHub could perform auto-merging. Counter-arguments:
  - There's not too many such merge conflicts
- This breaks `builtins.unsafeGetAttrPos "hello" pkgs`. Counter-arguments:
  - This functionality is unsafe and therefore breakages can be expected
  - Support for this can be added to Nix (make `builtins.readDir` propagate file as a position)

# Alternatives
[alternatives]: #alternatives

- Create a prefix-based hierarchy of directories, e.g. `pkgs/auto/he/hello`, similar to `.git/objects`, so that no directory has more than 1000 entries, enabling GitHub to display the entries, therefore improving navigation on GitHub. Downsides are:
  - Slower evaluation, since a lot more directories need to be traversed
  - Increased end-user complexity:
    - Creating the package files often requires the creating of 2 directories, not just one
    - Referencing the files requires knowing the prefix schema
- Improve deprecation signalling by creating `.nix` files that act like a symlink, but with a warning. Something like this:
  ```nix
  builtins.trace "warning: Using deprecated path ${./.}, use pkgs/auto/<name> instead, this will be removed after NixOS 22.05"
    (import ../../pkgs/auto/name)
  ```
  The main downside of this is the increased complexity of implementation

# Unresolved questions
[unresolved]: #unresolved-questions

- Is it really okay to not be able to list the package directory in GitHub? Will the "Go to file" function be good enough?

# Future work
[future]: #future-work

- This RFC only addresses the top-level attribute namespace, aka packages in `pkgs.<name>`, it doesn't do anything about package sets like `pkgs.python3Packages.<name>`, `pkgs.haskell.packages.ghc942.<name>`, which could also benefit from a similar auto-calling
