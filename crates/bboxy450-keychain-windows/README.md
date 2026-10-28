# bboxy450-keychain-windows

DO RESEARCH BEFORE IMPLEMENTING the native vault. `open()` answers
`Unsupported { os: "windows", .. }`; it does not open Credential Manager.

Primary-source research retrieved 2026-10-24 before this placeholder:

- [CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw)
  reads the current logon token's credential set. A real backend must handle a
  missing set without prompting or falling back to another identity.
- [CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew)
  stores application-defined generic blobs.
  [CREDENTIALW persistence](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)
  distinguishes a logon-session lifetime from this user's subsequent logons on
  the same machine. This needs native persistence and isolation tests.

`private_files` is a separate implemented permission port for file custody.
It creates directories and files with a protected DACL granting only the
current token's user SID full access, refuses reparse points, and checks the
owner and every allow ACE on the opened handle. It never repairs an existing
broad DACL. These are the Windows equivalent of the file backend's Unix
0700 directories and 0600 files, with no encryption or wrapping key.

Mechanisms: [security descriptor conversion](https://learn.microsoft.com/en-us/windows/win32/api/sddl/nf-sddl-convertstringsecuritydescriptortosecuritydescriptorw),
[handle security information](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo),
and [ACE inspection](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getace).
Cross-compilation checks its API; native Windows behaviour requires a Windows
test run and is not proven by a macOS run.

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
