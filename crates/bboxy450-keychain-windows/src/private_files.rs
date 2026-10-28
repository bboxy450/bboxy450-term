// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Owner-only file operations; independent of the native credential vault.
use std::{
    ffi::c_void,
    fs::File,
    io,
    mem::{size_of, zeroed},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr::{addr_of, null_mut},
};
use windows_sys::Win32::{
    Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetSecurityInfo, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetTokenInformation,
        OWNER_SECURITY_INFORMATION, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CREATE_NEW, CreateDirectoryW, CreateFileW,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

struct Local(*mut c_void);
impl Drop for Local {
    fn drop(&mut self) {
        // SAFETY: each pointer comes from a documented LocalAlloc API below.
        unsafe {
            LocalFree(self.0);
        }
    }
}
struct User(Vec<usize>);
impl User {
    fn current() -> io::Result<Self> {
        let mut handle = null_mut();
        // SAFETY: current process pseudo-handle and a valid output slot.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a new token handle is owned and closed exactly once.
        let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
        let mut length = 0;
        // SAFETY: the first call only obtains the required size.
        unsafe {
            GetTokenInformation(
                handle.as_raw_handle(),
                TokenUser,
                null_mut(),
                0,
                &mut length,
            );
        }
        if length < size_of::<TOKEN_USER>() as u32 || length > 64 * 1024 {
            return Err(denied());
        }
        let mut words = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        // SAFETY: usize storage is aligned for TOKEN_USER and covers length bytes.
        if unsafe {
            GetTokenInformation(
                handle.as_raw_handle(),
                TokenUser,
                words.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(words))
    }
    fn sid(&self) -> PSID {
        // SAFETY: successful GetTokenInformation initialized this aligned buffer;
        // the embedded SID remains in the same owned allocation.
        unsafe { (*(self.0.as_ptr().cast::<TOKEN_USER>())).User.Sid }
    }
    fn text(&self) -> io::Result<String> {
        let mut text = null_mut();
        // SAFETY: valid user SID and output pointer, freed with LocalFree.
        if unsafe { ConvertSidToStringSidW(self.sid(), &mut text) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let _owned = Local(text.cast());
        let mut length = 0;
        // SAFETY: Win32 returns a NUL-terminated SID; this additionally caps it.
        while length < 256 && unsafe { *text.add(length) } != 0 {
            length += 1;
        }
        if length == 256 {
            return Err(denied());
        }
        // SAFETY: length was measured in the allocated string above.
        String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) })
            .map_err(|_| denied())
    }
}
fn descriptor() -> io::Result<Local> {
    let user = User::current()?;
    let sid = user.text()?;
    let sddl: Vec<_> = format!("O:{sid}D:P(A;OICI;FA;;;{sid})")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = null_mut();
    // SAFETY: valid NUL-terminated SDDL and output slots; revision 1 is specified.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(Local(descriptor))
}
fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NUL in custody path",
        ));
    }
    wide.push(0);
    Ok(wide)
}
fn denied() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "owner-only file custody required",
    )
}
/// Check the opened object's owner and DACL; no permission is changed.
pub fn verify(file: &File) -> io::Result<()> {
    let user = User::current()?;
    let mut owner = null_mut();
    let mut dacl: *mut ACL = null_mut();
    let mut descriptor = null_mut();
    // SAFETY: live file handle, valid output slots and documented security flags.
    let error = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor,
        )
    };
    if error != 0 {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    let _owned = Local(descriptor);
    if owner.is_null() || dacl.is_null() || unsafe { EqualSid(owner, user.sid()) } == 0 {
        return Err(denied());
    }
    // SAFETY: GetSecurityInfo returned this kernel-validated ACL and owner SID.
    let count = unsafe { (*dacl).AceCount };
    if count == 0 || count > 128 {
        return Err(denied());
    }
    for index in 0..u32::from(count) {
        let mut ace = null_mut();
        // SAFETY: index is within the ACL count and ace is a valid output slot.
        if unsafe { GetAce(dacl, index, &mut ace) } == 0 || ace.is_null() {
            return Err(denied());
        }
        // SAFETY: the OS validates ACE sizes; inspect the header before the body.
        let header = unsafe { &*ace.cast::<ACE_HEADER>() };
        if header.AceType != 0 || usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>() {
            return Err(denied());
        }
        // SAFETY: the allowed-ACE type and size were checked, SID begins at SidStart.
        let sid = unsafe {
            addr_of!((*ace.cast::<ACCESS_ALLOWED_ACE>()).SidStart)
                .cast_mut()
                .cast()
        };
        if unsafe { EqualSid(sid, user.sid()) } == 0 {
            return Err(denied());
        }
    }
    Ok(())
}
fn open(path: &Path, create: bool, directory: bool) -> io::Result<File> {
    let path = wide(path)?;
    let descriptor = descriptor()?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // SAFETY: pointers are live through the call; OPEN_REPARSE_POINT refuses
    // following a link and the returned handle is adopted once below.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | if create { GENERIC_WRITE } else { 0 },
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &attributes,
            if create { CREATE_NEW } else { OPEN_EXISTING },
            FILE_ATTRIBUTE_NORMAL
                | FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful CreateFileW returned this new owned file handle.
    let file = unsafe { File::from_raw_handle(handle) };
    // SAFETY: File information is a plain output structure, initialized by Win32.
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
    {
        return Err(denied());
    }
    verify(&file)?;
    Ok(file)
}
/// Create an owner-only directory, or verify an existing one without repairing it.
pub fn directory(path: &Path, create: bool) -> io::Result<File> {
    if create {
        let wide = wide(path)?;
        let descriptor = descriptor()?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        // SAFETY: attributes and path remain owned through directory creation.
        if unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) } == 0 {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::AlreadyExists {
                return Err(error);
            }
        }
    }
    open(path, false, true)
}
/// Open a regular owner-only file without following a reparse point.
pub fn file(path: &Path, create: bool) -> io::Result<File> {
    open(path, create, false)
}

/// A path replacement cannot silently redirect an existing custody handle.
pub fn same_file(one: &File, two: &File) -> io::Result<bool> {
    // SAFETY: both POD outputs are initialized by their respective successful calls.
    let mut a: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    let mut b: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    if unsafe { GetFileInformationByHandle(one.as_raw_handle(), &mut a) } == 0
        || unsafe { GetFileInformationByHandle(two.as_raw_handle(), &mut b) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok((a.dwVolumeSerialNumber, a.nFileIndexHigh, a.nFileIndexLow)
        == (b.dwVolumeSerialNumber, b.nFileIndexHigh, b.nFileIndexLow))
}
