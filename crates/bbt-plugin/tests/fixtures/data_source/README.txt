data_source 1.0 conformance data

conformance.json contains portable request/result schema examples for all eight methods.
Its cursors and digest are illustrative; a running provider supplies its actual values.
seed/ is the shared real two-record root for bundled and later stdio conformance:
- resolve #7, brief, its ID, and tasks/brief.md to the same reference;
- fetch preserves frontmatter/body and returns images/pixel.png as a confined file link;
- search "fixture" with limit 1 has next; list type=task returns records 7 and 8;
- links go from record 7 to 8 with relation depends and weight 2;
- graph pages preserve those node/edge identities;
- creating a third record through write allocates number 9, with the Index's writer;
- after the host's existing scan/publish/ack cycle, changes_since includes that new reference.
Use temporary copies for writes. Never mutate this checked-in seed or compare a live
reference to the illustrative all-a digest. images/pixel.png is a generated 1x1 RGBA
PNG test fixture, not a user image or copied third-party artwork.
