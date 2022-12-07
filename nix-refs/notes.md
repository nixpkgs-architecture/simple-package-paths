root
- common
  - a
    - b
      - c
  - x
    - y         (current directory)
      - z.nix   "../../a/b/c"


Rename/Move allowed?
- root: Yes/Yes
- common: Yes/Yes
- a: No/No
- b: No/No
- c: No/No
- x: Yes/No
- y: Yes/No
- z.nix: Yes/No

An arbitrary relative reference has
- A common ancestor directory (common) that's safe to rename/move
- An upwards path (z.nix, y, x) that's only safe to rename
- A downwards path (a, b, c) that's neither safe to move nor rename

Function to determine this needs to:
- Set `dir` to the directory of the file with the reference
- Mark the file with the reference as unsafe to move
- Process each reference path component:
  - If it's a `..` component
    - Mark `dir` as unsafe to move
    - Set `dir` to its parent
  - Otherwise:
    - Set `dir` to `<dir>/<component>`
    - Mark `<dir>` as unsafe to move and rename


When we try to move a path, it should either work, or give an error like this:
> Cannot perform this move because it would break the reference "../../foo/bar" at <file>:<line>:<column>

To do this, we need a function that takes a move operation and tells us whether it's valid, like
```
fn check_move(from: Path, to: Path) -> Result<[Reference], ()>
fn check_rename(path: Path) -> Result<[Reference], ()>
fn check_move(path: Path) -> Result<[Reference], ()>
```

`check_move(./foo/bar, ./foo/baz)` should do:
- Determine whether it's a rename or a move
  - Call `check_rename(./foo/bar)` if it's a rename
  - Call `check_move(./foo/bar)` if it's a move

`check_{rename,move}` can just check their respective trees


At the beginning, create an empty tree from all files in the given root directory. Rose tree

```
data Tree = Tree [Reference] (Map String Tree)
```





