# bboxy450-keychain-file

Passphrase-less, immutable key slots in an explicit private directory. The file
is the secret. There is no wrapping key, native-vault fallback or global queue.

`Store::open` creates a private directory or verifies its existing permissions;
it never repairs a broad directory. `put` publishes a complete slot atomically
and refuses replacement. `get` refuses links, non-regular files and values over
64 KiB. `remove` is explicit. Secret buffers erase themselves on drop and do
not print their contents through Debug.

Unix uses a retained directory descriptor, relative no-follow opens, 0700
directories and 0600 files. Windows uses the per-OS permission port in
`bboxy450-keychain-windows`: protected owner-only DACLs, opened-handle checks
and reparse-point refusal. Local disk paths only on Windows. The implementation
does not claim protection from another process running as the same user.

This kit stores bytes; account-root generation, grants, verified run labels and
startup selection belong to the identity owner. Tests generate their own keys
and directories. They never open the operator's account or native credential
store. Windows runtime behaviour needs native Windows tests; cross-compilation
does not prove it.
