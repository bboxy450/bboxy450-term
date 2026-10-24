# bboxy450-keychain

Explicit identity custody selection, independent of the terminal, window,
protocol and account implementation. `open(Backend::File { directory })` returns
the `Store` port: `get`, immutable `put`, explicit `remove`, and `acl`.

The file-first U-0 cut exposes only file selection. The application's existing
native Keychain path remains separate until its facade extraction and the
one-shot re-grant action (#280). This cut does not enroll a production
automation device or unlock the operator's Keychain.

The caller owns IO scheduling. No selected backend error opens a different
store or generates a replacement identity.
