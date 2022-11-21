# Some useful evaluations to probe some RFC-relevant statistics about nixpkgs
{ nixpkgs ? fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/cf63ade6f74bbc9d2a017290f1b2e33e8fbfa70a.tar.gz";
    sha256 = "1aw4avc6mp3v1gwjlax6yn8984c92y56s4h7qrygxagpchkwq06j";
  }
}:
let
  pkgs = import nixpkgs {
    config = { allowAliases = false; };
    overlays = [];
  };
  inherit (pkgs) lib;

  attrsByFile = set:
    let
      attrsToFile = lib.mapAttrs (name: value:
        lib.mapNullable
        (lib.removePrefix (toString nixpkgs + "/"))
        ((builtins.unsafeGetAttrPos name set).file or null)
      ) set;
      filesToAttrs =
        lib.mapAttrs (name: map (value: value.name))
        (lib.groupBy (entry:
          if entry.value == null
          then "<unknown>"
          else entry.value)
        (lib.mapAttrsToList lib.nameValuePair attrsToFile));
      limit = 5;
      list = values:
        if lib.length values > limit
        then lib.generators.toPretty { multiline = false; } (lib.take limit values)
          + " and ${toString (lib.length values - limit)} more"
        else lib.generators.toPretty { multiline = false; } values;
      printed = lib.foldl'
        (acc: el: builtins.trace "${el.name}: ${list el.value}" acc)
        null (lib.mapAttrsToList lib.nameValuePair filesToAttrs);
    in builtins.trace "Which files define which attributes:" printed;

in {
  attrsByFile = attrsByFile pkgs;
}
