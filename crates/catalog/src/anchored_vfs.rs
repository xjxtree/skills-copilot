use std::{
    collections::{hash_map::Entry, HashMap},
    ffi::{c_char, c_int, c_void, CStr, CString},
    fs::File,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::Path,
    ptr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
};

use rusqlite::ffi;
use rustix::fs::{
    accessat, fchmod, fstat, fsync, openat, statat, unlinkat, Access, AtFlags, FileType, Mode,
    OFlags, Stat,
};

static NEXT_VFS_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);
static FILE_REGISTRY: OnceLock<Mutex<FileRegistry>> = OnceLock::new();
static CATALOG_TARGETS: OnceLock<Mutex<HashMap<CatalogTarget, CatalogTargetState>>> =
    OnceLock::new();

#[derive(Default)]
struct FileRegistry {
    open: HashMap<usize, OpenFileState>,
    // Closing any descriptor for an inode can release this process's POSIX
    // record locks on that inode. Descriptor shims therefore stay open until
    // every wrapped file and every path-open Catalog connection has closed.
    retired: Vec<File>,
    // Path-open Catalog connections use the parent VFS directly. Retired shim
    // descriptors cannot close while one of those connections may hold a
    // process-scoped lock, even when no wrapped file remains.
    raw_catalog_opens: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CatalogTarget {
    directory_device: u64,
    directory_inode: u64,
    child_name: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CatalogOpenStyle {
    Anchored,
    Path,
}

#[derive(Default)]
struct CatalogTargetState {
    anchored: usize,
    path: usize,
}

#[derive(Debug)]
pub(crate) struct CatalogOpenSafetyLease {
    target: CatalogTarget,
    style: CatalogOpenStyle,
    raw_counted: bool,
}

impl CatalogOpenSafetyLease {
    pub(crate) fn for_anchored_owner(owner: &File) -> Result<Self, String> {
        Self::reserve(
            catalog_target(owner, b"catalog.sqlite")?,
            CatalogOpenStyle::Anchored,
        )
    }

    pub(crate) fn for_path(path: &Path) -> Result<Self, String> {
        let child_name = path
            .file_name()
            .map(|name| name.as_bytes())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| "catalog path has no child filename".to_string())?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let owner = File::open(parent)
            .map_err(|error| format!("opening catalog parent directory failed: {error}"))?;
        Self::reserve(catalog_target(&owner, child_name)?, CatalogOpenStyle::Path)
    }

    fn reserve(target: CatalogTarget, style: CatalogOpenStyle) -> Result<Self, String> {
        let mut targets = CATALOG_TARGETS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| "catalog target registry is unavailable".to_string())?;
        let target_state = targets.entry(target.clone()).or_default();
        let conflicts = match style {
            CatalogOpenStyle::Anchored => target_state.path != 0,
            CatalogOpenStyle::Path => target_state.anchored != 0,
        };
        if conflicts {
            return Err(
                "path-open and descriptor-anchored catalog connections cannot target the same owner child"
                    .to_string(),
            );
        }
        match style {
            CatalogOpenStyle::Anchored => target_state.anchored += 1,
            CatalogOpenStyle::Path => target_state.path += 1,
        }
        drop(targets);

        let mut lease = Self {
            target,
            style,
            raw_counted: false,
        };
        if style == CatalogOpenStyle::Path {
            let mut registry = FILE_REGISTRY
                .get_or_init(|| Mutex::new(FileRegistry::default()))
                .lock()
                .map_err(|_| "SQLite file registry is unavailable".to_string())?;
            registry.raw_catalog_opens = registry
                .raw_catalog_opens
                .checked_add(1)
                .ok_or_else(|| "path-open catalog count overflowed".to_string())?;
            lease.raw_counted = true;
        }
        Ok(lease)
    }
}

impl Drop for CatalogOpenSafetyLease {
    fn drop(&mut self) {
        if let Ok(mut targets) = CATALOG_TARGETS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
        {
            if let Entry::Occupied(mut target) = targets.entry(self.target.clone()) {
                let target_state = target.get_mut();
                match self.style {
                    CatalogOpenStyle::Anchored => {
                        target_state.anchored = target_state.anchored.saturating_sub(1);
                    }
                    CatalogOpenStyle::Path => {
                        target_state.path = target_state.path.saturating_sub(1);
                    }
                }
                if target_state.anchored == 0 && target_state.path == 0 {
                    target.remove();
                }
            }
        }
        if self.raw_counted {
            release_raw_catalog_open();
        }
    }
}

fn catalog_target(owner: &File, child_name: &[u8]) -> Result<CatalogTarget, String> {
    let metadata = owner
        .metadata()
        .map_err(|error| format!("reading catalog owner metadata failed: {error}"))?;
    if !metadata.is_dir() {
        return Err("catalog owner is not a directory".to_string());
    }
    Ok(CatalogTarget {
        directory_device: metadata.dev(),
        directory_inode: metadata.ino(),
        child_name: child_name.to_vec(),
    })
}

fn release_raw_catalog_open() {
    let retired = FILE_REGISTRY
        .get_or_init(|| Mutex::new(FileRegistry::default()))
        .lock()
        .ok()
        .map(|mut registry| {
            registry.raw_catalog_opens = registry.raw_catalog_opens.saturating_sub(1);
            take_retired_if_idle(&mut registry)
        })
        .unwrap_or_default();
    drop(retired);
}

fn take_retired_if_idle(registry: &mut FileRegistry) -> Vec<File> {
    if registry.open.is_empty() && registry.raw_catalog_opens == 0 {
        std::mem::take(&mut registry.retired)
    } else {
        Vec::new()
    }
}

struct OpenFileState {
    vfs_state: usize,
    parent_methods: usize,
    parent_filename: usize,
    sync_owner_on_first_sync: bool,
}

impl Drop for OpenFileState {
    fn drop(&mut self) {
        if self.parent_filename != 0 {
            unsafe {
                ffi::sqlite3_free_filename(self.parent_filename as ffi::sqlite3_filename);
            }
        }
    }
}

#[derive(Debug)]
struct AnchoredVfsState {
    parent: *mut ffi::sqlite3_vfs,
    owner: File,
    namespace: CString,
}

#[derive(Debug)]
pub(crate) struct AnchoredVfsLease {
    vfs: Option<Box<ffi::sqlite3_vfs>>,
    state: Option<Box<AnchoredVfsState>>,
    name: Option<CString>,
}

// The lease owns stable heap allocations registered with SQLite. Moving the
// owning Catalog to another thread does not move those allocations. File
// descriptors and paths remain descriptor-relative and never change cwd.
unsafe impl Send for AnchoredVfsLease {}

impl AnchoredVfsLease {
    pub(crate) fn register(owner: File) -> Result<Self, String> {
        let parent = unsafe { ffi::sqlite3_vfs_find(ptr::null()) };
        if parent.is_null() {
            return Err("SQLite default VFS is unavailable".to_string());
        }
        let id = NEXT_VFS_ID.fetch_add(1, Ordering::Relaxed);
        let identity = format!("agent-copilot-dirfd-{}-{id}", std::process::id());
        let name = CString::new(identity.as_str()).map_err(|error| error.to_string())?;
        if !owner
            .metadata()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            return Err("descriptor-anchored SQLite owner is not a directory".to_string());
        }
        let namespace = CString::new(format!("/{identity}/")).map_err(|error| error.to_string())?;
        let mut state = Box::new(AnchoredVfsState {
            parent,
            owner,
            namespace,
        });
        let parent_ref = unsafe { &*parent };
        let mut vfs = Box::new(ffi::sqlite3_vfs {
            iVersion: if parent_ref.iVersion >= 2 && parent_ref.xCurrentTimeInt64.is_some() {
                2
            } else {
                1
            },
            szOsFile: parent_ref.szOsFile,
            mxPathname: parent_ref.mxPathname.min(1024),
            pNext: ptr::null_mut(),
            zName: name.as_ptr(),
            pAppData: (&mut *state as *mut AnchoredVfsState).cast::<c_void>(),
            xOpen: Some(anchored_open),
            xDelete: Some(anchored_delete),
            xAccess: Some(anchored_access),
            xFullPathname: Some(anchored_full_pathname),
            xDlOpen: None,
            xDlError: None,
            xDlSym: None,
            xDlClose: None,
            xRandomness: Some(delegated_randomness),
            xSleep: Some(delegated_sleep),
            xCurrentTime: Some(delegated_current_time),
            xGetLastError: Some(delegated_get_last_error),
            xCurrentTimeInt64: Some(delegated_current_time_int64),
            xSetSystemCall: None,
            xGetSystemCall: None,
            xNextSystemCall: None,
        });
        let result = unsafe { ffi::sqlite3_vfs_register(&mut *vfs, 0) };
        if result != ffi::SQLITE_OK {
            return Err(format!(
                "registering descriptor-anchored SQLite VFS failed with code {result}"
            ));
        }
        Ok(Self {
            vfs: Some(vfs),
            state: Some(state),
            name: Some(name),
        })
    }

    pub(crate) fn name(&self) -> &str {
        self.name
            .as_deref()
            .expect("registered anchored VFS retains its name")
            .to_str()
            .expect("generated anchored VFS name is UTF-8")
    }
}

impl Drop for AnchoredVfsLease {
    fn drop(&mut self) {
        let Some(mut vfs) = self.vfs.take() else {
            return;
        };
        let result = unsafe { ffi::sqlite3_vfs_unregister(&mut *vfs) };
        if result != ffi::SQLITE_OK {
            // SQLite may still hold the VFS pointer. Leaking is safer than
            // releasing callback state that a live connection could access.
            std::mem::forget(vfs);
            if let Some(state) = self.state.take() {
                std::mem::forget(state);
            }
            if let Some(name) = self.name.take() {
                std::mem::forget(name);
            }
        }
    }
}

unsafe extern "C" fn anchored_open(
    vfs: *mut ffi::sqlite3_vfs,
    name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    out_flags: *mut c_int,
) -> c_int {
    if file.is_null() {
        return ffi::SQLITE_CANTOPEN;
    }
    // SQLite permits a failed xOpen to leave either a valid method table or a
    // null one. Initialize it before every branch that can fail.
    unsafe {
        (*file).pMethods = ptr::null();
    }
    let Some((state, callback)) = state(vfs)
        .and_then(|state| unsafe { (*state.parent).xOpen }.map(|callback| (state, callback)))
    else {
        return ffi::SQLITE_CANTOPEN;
    };

    // This VFS intentionally supports rollback journals only. SQLite shared
    // memory bypasses xOpen in the parent VFS, so accepting WAL here would
    // forfeit descriptor-relative path ownership.
    if flags & ffi::SQLITE_OPEN_WAL != 0 {
        return ffi::SQLITE_CANTOPEN;
    }

    let preopened = match preopen_sqlite_child(state, name, flags) {
        Ok(preopened) => preopened,
        Err(code) => return code,
    };
    let (parent_filename, parent_name) =
        match create_parent_filename(&preopened.file, preopened.effective_flags) {
            Ok(filename) => filename,
            Err(code) => {
                cleanup_created_child(state, &preopened);
                retire_descriptor(preopened.file);
                return code;
            }
        };
    let parent_flags = (preopened.effective_flags
        & !(ffi::SQLITE_OPEN_CREATE
            | ffi::SQLITE_OPEN_EXCLUSIVE
            | ffi::SQLITE_OPEN_DELETEONCLOSE
            | ffi::SQLITE_OPEN_AUTOPROXY))
        | ffi::SQLITE_OPEN_NOFOLLOW;
    let mut parent_out_flags = 0;
    let result = unsafe {
        callback(
            state.parent,
            parent_name,
            file,
            parent_flags,
            &mut parent_out_flags,
        )
    };
    if result != ffi::SQLITE_OK {
        close_parent_after_failed_open(file);
        unsafe {
            ffi::sqlite3_free_filename(parent_filename);
        }
        cleanup_created_child(state, &preopened);
        retire_descriptor(preopened.file);
        return result;
    }
    let parent_methods = unsafe { (*file).pMethods };
    if !valid_parent_methods(parent_methods) {
        close_parent_after_failed_open(file);
        unsafe {
            ffi::sqlite3_free_filename(parent_filename);
        }
        cleanup_created_child(state, &preopened);
        retire_descriptor(preopened.file);
        return ffi::SQLITE_CANTOPEN;
    }
    if preopened.effective_flags & ffi::SQLITE_OPEN_DELETEONCLOSE != 0
        && unlink_opened_child(state, &preopened).is_err()
    {
        close_parent_after_failed_open(file);
        unsafe {
            ffi::sqlite3_free_filename(parent_filename);
        }
        cleanup_created_child(state, &preopened);
        retire_descriptor(preopened.file);
        return ffi::SQLITE_IOERR_DELETE;
    }
    let entry = OpenFileState {
        vfs_state: (state as *const AnchoredVfsState) as usize,
        parent_methods: parent_methods as usize,
        parent_filename: parent_filename as usize,
        sync_owner_on_first_sync: preopened.created
            && preopened.effective_flags & ffi::SQLITE_OPEN_DELETEONCLOSE == 0,
    };
    let Ok(mut registry) = FILE_REGISTRY
        .get_or_init(|| Mutex::new(FileRegistry::default()))
        .lock()
    else {
        close_parent_after_failed_open(file);
        unsafe {
            ffi::sqlite3_free_filename(parent_filename);
        }
        cleanup_created_child(state, &preopened);
        std::mem::forget(preopened.file);
        return ffi::SQLITE_CANTOPEN;
    };
    match registry.open.entry(file as usize) {
        Entry::Vacant(slot) => {
            slot.insert(entry);
            registry.retired.push(preopened.file);
        }
        Entry::Occupied(_) => {
            registry.retired.push(preopened.file);
            drop(registry);
            close_parent_after_failed_open(file);
            drop(entry);
            return ffi::SQLITE_CANTOPEN;
        }
    }
    unsafe {
        (*file).pMethods = &ANCHORED_IO_METHODS;
        if !out_flags.is_null() {
            *out_flags = (preopened.effective_flags
                & !(ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_READWRITE))
                | (parent_out_flags & (ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_READWRITE));
        }
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn anchored_delete(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    sync_dir: c_int,
) -> c_int {
    if name.is_null() {
        return ffi::SQLITE_IOERR_DELETE;
    }
    let Some(state) = state(vfs) else {
        return ffi::SQLITE_IOERR_DELETE;
    };
    let Some(name) = resolve_child_name(state, name) else {
        return ffi::SQLITE_IOERR_DELETE;
    };
    match safe_child_metadata(state, name.as_c_str()) {
        Ok(_) => {}
        Err(rustix::io::Errno::NOENT) => return ffi::SQLITE_IOERR_DELETE_NOENT,
        Err(_) => return ffi::SQLITE_IOERR_DELETE,
    }
    if unlinkat(&state.owner, name.as_c_str(), AtFlags::empty()).is_err() {
        return ffi::SQLITE_IOERR_DELETE;
    }
    if sync_dir & 1 != 0 && fsync(&state.owner).is_err() {
        return ffi::SQLITE_IOERR_DIR_FSYNC;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn anchored_access(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    flags: c_int,
    result: *mut c_int,
) -> c_int {
    if result.is_null() {
        return ffi::SQLITE_IOERR_ACCESS;
    }
    unsafe {
        *result = 0;
    }
    if name.is_null() {
        return ffi::SQLITE_IOERR_ACCESS;
    }
    let Some(state) = state(vfs) else {
        return ffi::SQLITE_IOERR_ACCESS;
    };
    let Some(name) = resolve_child_name(state, name) else {
        return ffi::SQLITE_IOERR_ACCESS;
    };
    match safe_child_metadata(state, name.as_c_str()) {
        Ok(_) => {}
        Err(rustix::io::Errno::NOENT) => return ffi::SQLITE_OK,
        Err(_) => return ffi::SQLITE_IOERR_ACCESS,
    };
    let accessible = match flags {
        ffi::SQLITE_ACCESS_EXISTS => true,
        ffi::SQLITE_ACCESS_READWRITE => accessat(
            &state.owner,
            name.as_c_str(),
            Access::READ_OK | Access::WRITE_OK,
            AtFlags::EACCESS | AtFlags::SYMLINK_NOFOLLOW,
        )
        .is_ok(),
        ffi::SQLITE_ACCESS_READ => accessat(
            &state.owner,
            name.as_c_str(),
            Access::READ_OK,
            AtFlags::EACCESS | AtFlags::SYMLINK_NOFOLLOW,
        )
        .is_ok(),
        _ => return ffi::SQLITE_IOERR_ACCESS,
    };
    unsafe {
        *result = c_int::from(accessible);
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn anchored_full_pathname(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    output_len: c_int,
    output: *mut c_char,
) -> c_int {
    if name.is_null() || output.is_null() || output_len <= 0 {
        return ffi::SQLITE_CANTOPEN;
    }
    let Some(state) = state(vfs) else {
        return ffi::SQLITE_CANTOPEN;
    };
    let Some(child) = resolve_child_name(state, name) else {
        return ffi::SQLITE_CANTOPEN;
    };
    let namespace = state.namespace.as_bytes();
    let child = child.as_bytes();
    let required = namespace
        .len()
        .checked_add(child.len())
        .and_then(|len| len.checked_add(1));
    let Some(required) = required else {
        return ffi::SQLITE_CANTOPEN;
    };
    if required > output_len as usize {
        return ffi::SQLITE_CANTOPEN;
    }
    unsafe {
        ptr::copy_nonoverlapping(namespace.as_ptr().cast::<c_char>(), output, namespace.len());
        ptr::copy_nonoverlapping(
            child.as_ptr().cast::<c_char>(),
            output.add(namespace.len()),
            child.len(),
        );
        *output.add(namespace.len() + child.len()) = 0;
    }
    ffi::SQLITE_OK
}

struct PreopenedChild {
    file: File,
    name: CString,
    created: bool,
    effective_flags: c_int,
}

fn preopen_sqlite_child(
    state: &AnchoredVfsState,
    name: ffi::sqlite3_filename,
    flags: c_int,
) -> Result<PreopenedChild, c_int> {
    if flags & ffi::SQLITE_OPEN_MEMORY != 0 {
        return Err(ffi::SQLITE_CANTOPEN);
    }
    if name.is_null() {
        if flags & ffi::SQLITE_OPEN_DELETEONCLOSE == 0 {
            return Err(ffi::SQLITE_CANTOPEN);
        }
        for _ in 0..32 {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let name = CString::new(format!(
                ".agent-copilot-sqlite-temp-{}-{id}",
                std::process::id()
            ))
            .map_err(|_| ffi::SQLITE_CANTOPEN)?;
            match open_child_create_new(state, name.as_c_str(), flags) {
                Ok(child) => return Ok(child),
                Err(rustix::io::Errno::EXIST) => continue,
                Err(_) => return Err(ffi::SQLITE_CANTOPEN),
            }
        }
        return Err(ffi::SQLITE_CANTOPEN);
    }

    let name = resolve_child_name(state, name).ok_or(ffi::SQLITE_CANTOPEN)?;
    open_named_child(state, name, flags).map_err(|_| ffi::SQLITE_CANTOPEN)
}

fn open_named_child(
    state: &AnchoredVfsState,
    name: CString,
    flags: c_int,
) -> Result<PreopenedChild, rustix::io::Errno> {
    let wants_read_write = flags & ffi::SQLITE_OPEN_READWRITE != 0;
    let wants_read_only = flags & ffi::SQLITE_OPEN_READONLY != 0;
    let wants_create = flags & ffi::SQLITE_OPEN_CREATE != 0;
    let wants_exclusive = flags & ffi::SQLITE_OPEN_EXCLUSIVE != 0;
    if wants_read_write == wants_read_only
        || (wants_create && !wants_read_write)
        || (wants_exclusive && !wants_create)
    {
        return Err(rustix::io::Errno::INVAL);
    }

    if wants_exclusive {
        return open_child_create_new(state, name.as_c_str(), flags);
    }

    let access = if wants_read_write {
        OFlags::RDWR
    } else {
        OFlags::RDONLY
    };
    match open_child_existing(state, name.as_c_str(), access) {
        Ok(file) => validated_preopened_child(state, file, name, false, flags),
        Err(rustix::io::Errno::NOENT) if wants_create => {
            match open_child_create_new(state, name.as_c_str(), flags) {
                Ok(child) => Ok(child),
                Err(rustix::io::Errno::EXIST) => {
                    let file = open_child_existing(state, name.as_c_str(), access)?;
                    validated_preopened_child(state, file, name, false, flags)
                }
                Err(error) => Err(error),
            }
        }
        Err(_) if wants_read_write => {
            let file = open_child_existing(state, name.as_c_str(), OFlags::RDONLY)?;
            let effective_flags = (flags
                & !(ffi::SQLITE_OPEN_READWRITE
                    | ffi::SQLITE_OPEN_CREATE
                    | ffi::SQLITE_OPEN_EXCLUSIVE))
                | ffi::SQLITE_OPEN_READONLY;
            validated_preopened_child(state, file, name, false, effective_flags)
        }
        Err(error) => Err(error),
    }
}

fn open_child_existing(
    state: &AnchoredVfsState,
    name: &CStr,
    access: OFlags,
) -> Result<File, rustix::io::Errno> {
    openat(
        &state.owner,
        name,
        access | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map(File::from)
}

fn open_child_create_new(
    state: &AnchoredVfsState,
    name: &CStr,
    flags: c_int,
) -> Result<PreopenedChild, rustix::io::Errno> {
    let mode = Mode::from_bits_truncate(0o600);
    let descriptor = openat(
        &state.owner,
        name,
        OFlags::RDWR
            | OFlags::CREATE
            | OFlags::EXCL
            | OFlags::NOFOLLOW
            | OFlags::NONBLOCK
            | OFlags::NOCTTY
            | OFlags::CLOEXEC,
        mode,
    )?;
    let file = File::from(descriptor);
    if let Err(error) = fchmod(&file, mode) {
        let _ = unlink_opened_child_parts(state, &file, name);
        retire_descriptor(file);
        return Err(error);
    }
    validated_preopened_child(
        state,
        file,
        name.to_owned(),
        true,
        flags | ffi::SQLITE_OPEN_READWRITE,
    )
}

fn validated_preopened_child(
    state: &AnchoredVfsState,
    file: File,
    name: CString,
    created: bool,
    effective_flags: c_int,
) -> Result<PreopenedChild, rustix::io::Errno> {
    let validation = (|| {
        let opened = fstat(&file)?;
        validate_child_stat(state, &opened)?;
        let linked = statat(&state.owner, name.as_c_str(), AtFlags::SYMLINK_NOFOLLOW)?;
        validate_child_stat(state, &linked)?;
        if opened.st_dev != linked.st_dev || opened.st_ino != linked.st_ino {
            return Err(rustix::io::Errno::STALE);
        }
        Ok(())
    })();
    if let Err(error) = validation {
        if created {
            let _ = unlink_opened_child_parts(state, &file, name.as_c_str());
        }
        retire_descriptor(file);
        return Err(error);
    }
    Ok(PreopenedChild {
        file,
        name,
        created,
        effective_flags,
    })
}

fn validate_child_stat(state: &AnchoredVfsState, metadata: &Stat) -> Result<(), rustix::io::Errno> {
    let owner = fstat(&state.owner)?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile
        || metadata.st_uid != owner.st_uid
        || metadata.st_nlink != 1
    {
        return Err(rustix::io::Errno::PERM);
    }
    Ok(())
}

fn safe_child_metadata(state: &AnchoredVfsState, name: &CStr) -> Result<Stat, rustix::io::Errno> {
    let metadata = statat(&state.owner, name, AtFlags::SYMLINK_NOFOLLOW)?;
    validate_child_stat(state, &metadata)?;
    Ok(metadata)
}

fn resolve_child_name(state: &AnchoredVfsState, name: *const c_char) -> Option<CString> {
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes();
    let namespace = state.namespace.as_bytes();
    let child = if let Some(child) = bytes.strip_prefix(namespace) {
        child
    } else if bytes.first() == Some(&b'/') {
        return None;
    } else {
        bytes
    };
    valid_child_name(child)
        .then(|| CString::new(child).ok())
        .flatten()
}

fn valid_child_name(name: &[u8]) -> bool {
    !name.is_empty()
        && name != b"."
        && name != b".."
        && !name
            .iter()
            .any(|byte| matches!(*byte, b'/' | b'\\' | b'\0'))
}

fn create_parent_filename(
    file: &File,
    flags: c_int,
) -> Result<(ffi::sqlite3_filename, ffi::sqlite3_filename), c_int> {
    let path =
        CString::new(format!("/dev/fd/{}", file.as_raw_fd())).map_err(|_| ffi::SQLITE_CANTOPEN)?;
    let filename = unsafe {
        ffi::sqlite3_create_filename(
            path.as_ptr(),
            path.as_ptr(),
            path.as_ptr(),
            0,
            ptr::null_mut(),
        )
    };
    if filename.is_null() {
        return Err(ffi::SQLITE_NOMEM);
    }
    let file_type = flags & 0x000f_ff00;
    let selected = unsafe {
        match file_type {
            ffi::SQLITE_OPEN_MAIN_JOURNAL => ffi::sqlite3_filename_journal(filename),
            ffi::SQLITE_OPEN_WAL => ffi::sqlite3_filename_wal(filename),
            _ => filename,
        }
    };
    if selected.is_null() {
        unsafe {
            ffi::sqlite3_free_filename(filename);
        }
        return Err(ffi::SQLITE_CANTOPEN);
    }
    Ok((filename, selected))
}

fn valid_parent_methods(methods: *const ffi::sqlite3_io_methods) -> bool {
    let Some(methods) = (unsafe { methods.as_ref() }) else {
        return false;
    };
    methods.iVersion >= 1
        && methods.xClose.is_some()
        && methods.xRead.is_some()
        && methods.xWrite.is_some()
        && methods.xTruncate.is_some()
        && methods.xSync.is_some()
        && methods.xFileSize.is_some()
        && methods.xLock.is_some()
        && methods.xUnlock.is_some()
        && methods.xCheckReservedLock.is_some()
        && methods.xFileControl.is_some()
        && methods.xSectorSize.is_some()
        && methods.xDeviceCharacteristics.is_some()
}

fn close_parent_after_failed_open(file: *mut ffi::sqlite3_file) {
    let methods = unsafe { file.as_ref() }.map(|file| file.pMethods);
    if let Some(close) = methods
        .and_then(|methods| unsafe { methods.as_ref() })
        .and_then(|methods| methods.xClose)
    {
        let _ = unsafe { close(file) };
    }
    if let Some(file) = unsafe { file.as_mut() } {
        file.pMethods = ptr::null();
    }
}

fn cleanup_created_child(state: &AnchoredVfsState, child: &PreopenedChild) {
    if child.created {
        let _ = unlink_opened_child(state, child);
    }
}

fn unlink_opened_child(
    state: &AnchoredVfsState,
    child: &PreopenedChild,
) -> Result<(), rustix::io::Errno> {
    unlink_opened_child_parts(state, &child.file, child.name.as_c_str())
}

fn unlink_opened_child_parts(
    state: &AnchoredVfsState,
    file: &File,
    name: &CStr,
) -> Result<(), rustix::io::Errno> {
    let opened = fstat(file)?;
    let linked = safe_child_metadata(state, name)?;
    if opened.st_dev != linked.st_dev || opened.st_ino != linked.st_ino {
        return Err(rustix::io::Errno::STALE);
    }
    unlinkat(&state.owner, name, AtFlags::empty())?;
    fsync(&state.owner)?;
    Ok(())
}

fn retire_descriptor(file: File) {
    let Ok(mut registry) = FILE_REGISTRY
        .get_or_init(|| Mutex::new(FileRegistry::default()))
        .lock()
    else {
        // A poisoned registry cannot prove that closing this descriptor is
        // safe for process-scoped POSIX locks.
        std::mem::forget(file);
        return;
    };
    if registry.open.is_empty() && registry.raw_catalog_opens == 0 {
        drop(registry);
        drop(file);
    } else {
        registry.retired.push(file);
    }
}

unsafe extern "C" fn delegated_randomness(
    vfs: *mut ffi::sqlite3_vfs,
    len: c_int,
    output: *mut c_char,
) -> c_int {
    let Some((parent, callback)) = parent_callback(vfs, |parent| parent.xRandomness) else {
        return 0;
    };
    unsafe { callback(parent, len, output) }
}

unsafe extern "C" fn delegated_sleep(vfs: *mut ffi::sqlite3_vfs, micros: c_int) -> c_int {
    let Some((parent, callback)) = parent_callback(vfs, |parent| parent.xSleep) else {
        return 0;
    };
    unsafe { callback(parent, micros) }
}

unsafe extern "C" fn delegated_current_time(vfs: *mut ffi::sqlite3_vfs, output: *mut f64) -> c_int {
    let Some((parent, callback)) = parent_callback(vfs, |parent| parent.xCurrentTime) else {
        return ffi::SQLITE_ERROR;
    };
    unsafe { callback(parent, output) }
}

unsafe extern "C" fn delegated_current_time_int64(
    vfs: *mut ffi::sqlite3_vfs,
    output: *mut ffi::sqlite3_int64,
) -> c_int {
    let Some((parent, callback)) = parent_callback(vfs, |parent| parent.xCurrentTimeInt64) else {
        return ffi::SQLITE_ERROR;
    };
    unsafe { callback(parent, output) }
}

unsafe extern "C" fn delegated_get_last_error(
    vfs: *mut ffi::sqlite3_vfs,
    len: c_int,
    output: *mut c_char,
) -> c_int {
    let Some((parent, callback)) = parent_callback(vfs, |parent| parent.xGetLastError) else {
        return 0;
    };
    unsafe { callback(parent, len, output) }
}

fn parent_callback<T: Copy>(
    vfs: *mut ffi::sqlite3_vfs,
    select: impl FnOnce(&ffi::sqlite3_vfs) -> Option<T>,
) -> Option<(*mut ffi::sqlite3_vfs, T)> {
    let state = state(vfs)?;
    let parent = unsafe { state.parent.as_ref() }?;
    select(parent).map(|callback| (state.parent, callback))
}

fn state<'a>(vfs: *mut ffi::sqlite3_vfs) -> Option<&'a AnchoredVfsState> {
    let app_data = unsafe { vfs.as_ref() }?.pAppData.cast::<AnchoredVfsState>();
    unsafe { app_data.as_ref() }
}

static ANCHORED_IO_METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    // Version 1 deliberately disables WAL shared memory and mmap fetches.
    // Those parent callbacks derive paths outside this wrapper's openat-owned
    // namespace and would weaken the descriptor anchor.
    iVersion: 1,
    xClose: Some(anchored_io_close),
    xRead: Some(anchored_io_read),
    xWrite: Some(anchored_io_write),
    xTruncate: Some(anchored_io_truncate),
    xSync: Some(anchored_io_sync),
    xFileSize: Some(anchored_io_file_size),
    xLock: Some(anchored_io_lock),
    xUnlock: Some(anchored_io_unlock),
    xCheckReservedLock: Some(anchored_io_check_reserved_lock),
    xFileControl: Some(anchored_io_file_control),
    xSectorSize: Some(anchored_io_sector_size),
    xDeviceCharacteristics: Some(anchored_io_device_characteristics),
    xShmMap: None,
    xShmLock: None,
    xShmBarrier: None,
    xShmUnmap: None,
    xFetch: None,
    xUnfetch: None,
};

unsafe extern "C" fn anchored_io_close(file: *mut ffi::sqlite3_file) -> c_int {
    let result = with_file_methods(file, ffi::SQLITE_IOERR_CLOSE, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xClose)
        else {
            return ffi::SQLITE_IOERR_CLOSE;
        };
        unsafe { callback(file) }
    });
    let retired = FILE_REGISTRY
        .get_or_init(|| Mutex::new(FileRegistry::default()))
        .lock()
        .ok()
        .map(|mut registry| {
            registry.open.remove(&(file as usize));
            take_retired_if_idle(&mut registry)
        })
        .unwrap_or_default();
    if let Some(file) = unsafe { file.as_mut() } {
        file.pMethods = ptr::null();
    }
    // Drop only after the registry lock is released. The idle check proves no
    // wrapped file or path-open Catalog remains, so closing descriptor shims
    // cannot clear one of their process-scoped POSIX locks.
    drop(retired);
    result
}

unsafe extern "C" fn anchored_io_read(
    file: *mut ffi::sqlite3_file,
    output: *mut c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    with_file_methods(file, ffi::SQLITE_IOERR_READ, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xRead) else {
            return ffi::SQLITE_IOERR_READ;
        };
        unsafe { callback(file, output, amount, offset) }
    })
}

unsafe extern "C" fn anchored_io_write(
    file: *mut ffi::sqlite3_file,
    input: *const c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    with_file_methods(file, ffi::SQLITE_IOERR_WRITE, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xWrite)
        else {
            return ffi::SQLITE_IOERR_WRITE;
        };
        unsafe { callback(file, input, amount, offset) }
    })
}

unsafe extern "C" fn anchored_io_truncate(
    file: *mut ffi::sqlite3_file,
    size: ffi::sqlite3_int64,
) -> c_int {
    with_file_methods(file, ffi::SQLITE_IOERR_TRUNCATE, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xTruncate)
        else {
            return ffi::SQLITE_IOERR_TRUNCATE;
        };
        unsafe { callback(file, size) }
    })
}

unsafe extern "C" fn anchored_io_sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    let result = with_file_methods(file, ffi::SQLITE_IOERR_FSYNC, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xSync) else {
            return ffi::SQLITE_IOERR_FSYNC;
        };
        unsafe { callback(file, flags) }
    });
    if result == ffi::SQLITE_OK {
        sync_owner_directory_if_pending(file)
    } else {
        result
    }
}

unsafe extern "C" fn anchored_io_file_size(
    file: *mut ffi::sqlite3_file,
    size: *mut ffi::sqlite3_int64,
) -> c_int {
    with_file_methods(file, ffi::SQLITE_IOERR_FSTAT, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xFileSize)
        else {
            return ffi::SQLITE_IOERR_FSTAT;
        };
        unsafe { callback(file, size) }
    })
}

unsafe extern "C" fn anchored_io_lock(file: *mut ffi::sqlite3_file, lock: c_int) -> c_int {
    with_file_methods(file, ffi::SQLITE_IOERR_LOCK, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xLock) else {
            return ffi::SQLITE_IOERR_LOCK;
        };
        unsafe { callback(file, lock) }
    })
}

unsafe extern "C" fn anchored_io_unlock(file: *mut ffi::sqlite3_file, lock: c_int) -> c_int {
    with_file_methods(file, ffi::SQLITE_IOERR_UNLOCK, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xUnlock)
        else {
            return ffi::SQLITE_IOERR_UNLOCK;
        };
        unsafe { callback(file, lock) }
    })
}

unsafe extern "C" fn anchored_io_check_reserved_lock(
    file: *mut ffi::sqlite3_file,
    result: *mut c_int,
) -> c_int {
    with_file_methods(file, ffi::SQLITE_IOERR_CHECKRESERVEDLOCK, |methods| {
        let Some(callback) =
            (unsafe { methods.as_ref() }).and_then(|methods| methods.xCheckReservedLock)
        else {
            return ffi::SQLITE_IOERR_CHECKRESERVEDLOCK;
        };
        unsafe { callback(file, result) }
    })
}

unsafe extern "C" fn anchored_io_file_control(
    file: *mut ffi::sqlite3_file,
    op: c_int,
    argument: *mut c_void,
) -> c_int {
    if op == ffi::SQLITE_FCNTL_HAS_MOVED {
        let Some(result) = (argument as *mut c_int).as_mut() else {
            return ffi::SQLITE_IOERR;
        };
        // The synthetic path is intentionally stable for the lifetime of the
        // owner directory descriptor, even if its original pathname is
        // renamed or replaced.
        *result = 0;
        return ffi::SQLITE_OK;
    }
    if matches!(
        op,
        ffi::SQLITE_FCNTL_GET_LOCKPROXYFILE | ffi::SQLITE_FCNTL_SET_LOCKPROXYFILE
    ) {
        return ffi::SQLITE_NOTFOUND;
    }
    with_file_methods(file, ffi::SQLITE_IOERR, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xFileControl)
        else {
            return ffi::SQLITE_NOTFOUND;
        };
        unsafe { callback(file, op, argument) }
    })
}

unsafe extern "C" fn anchored_io_sector_size(file: *mut ffi::sqlite3_file) -> c_int {
    with_file_methods(file, 0, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xSectorSize)
        else {
            return 0;
        };
        unsafe { callback(file) }
    })
}

unsafe extern "C" fn anchored_io_device_characteristics(file: *mut ffi::sqlite3_file) -> c_int {
    with_file_methods(file, 0, |methods| {
        let Some(callback) =
            (unsafe { methods.as_ref() }).and_then(|methods| methods.xDeviceCharacteristics)
        else {
            return 0;
        };
        unsafe { callback(file) }
    })
}

fn with_file_methods(
    file: *mut ffi::sqlite3_file,
    failure: c_int,
    callback: impl FnOnce(*const ffi::sqlite3_io_methods) -> c_int,
) -> c_int {
    let Some(parent_methods) = FILE_REGISTRY
        .get_or_init(|| Mutex::new(FileRegistry::default()))
        .lock()
        .ok()
        .and_then(|registry| {
            registry
                .open
                .get(&(file as usize))
                .map(|entry| entry.parent_methods)
        })
    else {
        return failure;
    };
    callback(parent_methods as *const ffi::sqlite3_io_methods)
}

fn sync_owner_directory_if_pending(file: *mut ffi::sqlite3_file) -> c_int {
    let Ok(mut registry) = FILE_REGISTRY
        .get_or_init(|| Mutex::new(FileRegistry::default()))
        .lock()
    else {
        return ffi::SQLITE_IOERR_DIR_FSYNC;
    };
    let Some(entry) = registry.open.get_mut(&(file as usize)) else {
        return ffi::SQLITE_IOERR_DIR_FSYNC;
    };
    if !entry.sync_owner_on_first_sync {
        return ffi::SQLITE_OK;
    }
    let Some(state) = (unsafe { (entry.vfs_state as *const AnchoredVfsState).as_ref() }) else {
        return ffi::SQLITE_IOERR_DIR_FSYNC;
    };
    if fsync(&state.owner).is_err() {
        return ffi::SQLITE_IOERR_DIR_FSYNC;
    }
    entry.sync_owner_on_first_sync = false;
    ffi::SQLITE_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn create(label: &str) -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "agent-copilot-vfs-{label}-{}-{id}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create test owner");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn wrapper_advertises_only_descriptor_safe_io_methods() {
        assert_eq!(ANCHORED_IO_METHODS.iVersion, 1);
        assert!(ANCHORED_IO_METHODS.xShmMap.is_none());
        assert!(ANCHORED_IO_METHODS.xShmLock.is_none());
        assert!(ANCHORED_IO_METHODS.xShmBarrier.is_none());
        assert!(ANCHORED_IO_METHODS.xShmUnmap.is_none());
        assert!(ANCHORED_IO_METHODS.xFetch.is_none());
        assert!(ANCHORED_IO_METHODS.xUnfetch.is_none());
    }

    #[test]
    fn failed_open_clears_the_method_table() {
        let mut file = ffi::sqlite3_file {
            pMethods: &ANCHORED_IO_METHODS,
        };

        let result = unsafe {
            anchored_open(
                ptr::null_mut(),
                ptr::null(),
                &mut file,
                ffi::SQLITE_OPEN_READONLY,
                ptr::null_mut(),
            )
        };

        assert_eq!(result, ffi::SQLITE_CANTOPEN);
        assert!(file.pMethods.is_null());
    }

    #[test]
    fn full_path_rejects_parent_and_absolute_names() {
        let owner_path = TestDirectory::create("reject-paths");
        let owner = File::open(&owner_path.0).expect("owner descriptor");
        let namespace = CString::new("/test-anchor/").expect("namespace");
        let mut state = AnchoredVfsState {
            parent: ptr::null_mut(),
            owner,
            namespace,
        };
        let mut vfs: ffi::sqlite3_vfs = unsafe { std::mem::zeroed() };
        vfs.pAppData = (&mut state as *mut AnchoredVfsState).cast::<c_void>();
        let mut output = [0i8; 128];
        let parent = CString::new("../outside.sqlite").expect("parent path");
        let absolute = CString::new("/outside.sqlite").expect("absolute path");

        assert_eq!(
            unsafe {
                anchored_full_pathname(
                    &mut vfs,
                    parent.as_ptr(),
                    output.len() as c_int,
                    output.as_mut_ptr(),
                )
            },
            ffi::SQLITE_CANTOPEN
        );
        assert_eq!(
            unsafe {
                anchored_full_pathname(
                    &mut vfs,
                    absolute.as_ptr(),
                    output.len() as c_int,
                    output.as_mut_ptr(),
                )
            },
            ffi::SQLITE_CANTOPEN
        );
    }

    #[test]
    fn full_path_returns_stable_synthetic_namespace() {
        let owner_path = TestDirectory::create("full-path");
        let owner = File::open(&owner_path.0).expect("owner descriptor");
        let namespace = CString::new("/test-anchor/").expect("namespace");
        let mut state = AnchoredVfsState {
            parent: ptr::null_mut(),
            owner,
            namespace,
        };
        let mut vfs: ffi::sqlite3_vfs = unsafe { std::mem::zeroed() };
        vfs.pAppData = (&mut state as *mut AnchoredVfsState).cast::<c_void>();
        let name = CString::new("catalog.sqlite").expect("name");
        let mut output = [0i8; 128];

        let result = unsafe {
            anchored_full_pathname(
                &mut vfs,
                name.as_ptr(),
                output.len() as c_int,
                output.as_mut_ptr(),
            )
        };

        assert_eq!(result, ffi::SQLITE_OK);
        assert_eq!(
            unsafe { CStr::from_ptr(output.as_ptr()) }.to_bytes(),
            b"/test-anchor/catalog.sqlite"
        );
    }
}
