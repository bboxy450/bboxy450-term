# Optional committed web references

If local Chromium is unavailable, the capture helper looks here for exact
`<component>/<state>-<density>-<theme>.png` files tracked by Git. It copies only
matching matrix cells, reads their PNG dimensions and reports every missing
reference as unproven. There are currently no committed complete matrix shots;
the unrelated `.bboxy/runs/ref-*.png` design inputs are not substitutes.
