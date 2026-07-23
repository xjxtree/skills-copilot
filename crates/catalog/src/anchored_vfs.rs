use std::{
    collections::HashMap,
    ffi::{c_char, c_int, c_void, CStr, CString},
    fs::File,
    ptr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
};

use rusqlite::ffi;
use rustix::{
    fs::{open, Mode, OFlags},
    process::fchdir,
};

static NEXT_VFS_ID: AtomicU64 = AtomicU64::new(1);
static CWD_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
static OPEN_FILES: OnceLock<Mutex<HashMap<usize, OpenFileState>>> = OnceLock::new();

#[derive(Clone, Copy)]
struct OpenFileState {
    vfs_state: usize,
    parent_methods: usize,
}

#[derive(Debug)]
struct AnchoredVfsState {
    parent: *mut ffi::sqlite3_vfs,
    owner: File,
}

#[derive(Debug)]
pub(crate) struct AnchoredVfsLease {
    vfs: Option<Box<ffi::sqlite3_vfs>>,
    state: Option<Box<AnchoredVfsState>>,
    name: Option<CString>,
}

// The lease owns stable heap allocations registered with SQLite. Moving the
// owning Catalog to another thread does not move those allocations; callback
// entry is serialized by SQLite and descriptor-relative cwd changes use the
// process-wide mutex below.
unsafe impl Send for AnchoredVfsLease {}

impl AnchoredVfsLease {
    pub(crate) fn register(owner: File) -> Result<Self, String> {
        let parent = unsafe { ffi::sqlite3_vfs_find(ptr::null()) };
        if parent.is_null() {
            return Err("SQLite default VFS is unavailable".to_string());
        }
        let id = NEXT_VFS_ID.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!("agent-copilot-dirfd-{}-{id}", std::process::id()))
            .map_err(|error| error.to_string())?;
        let mut state = Box::new(AnchoredVfsState { parent, owner });
        let parent_ref = unsafe { &*parent };
        let mut vfs = Box::new(ffi::sqlite3_vfs {
            iVersion: 2,
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
    if !name.is_null() && !valid_relative_name(name) {
        return ffi::SQLITE_CANTOPEN;
    }
    let Some((state, callback)) = state(vfs)
        .and_then(|state| unsafe { (*state.parent).xOpen }.map(|callback| (state, callback)))
    else {
        return ffi::SQLITE_CANTOPEN;
    };
    if file.is_null() {
        return ffi::SQLITE_CANTOPEN;
    }
    unsafe {
        (*file).pMethods = ptr::null();
    }
    let result = with_owner_cwd(state, ffi::SQLITE_CANTOPEN, || unsafe {
        callback(state.parent, name, file, flags, out_flags)
    });
    if result != ffi::SQLITE_OK {
        return result;
    }
    let parent_methods = unsafe { (*file).pMethods };
    if parent_methods.is_null() || unsafe { (*parent_methods).iVersion } < 3 {
        if let Some(close) = unsafe { parent_methods.as_ref() }.and_then(|methods| methods.xClose) {
            let _ = with_owner_cwd(state, ffi::SQLITE_IOERR_CLOSE, || unsafe { close(file) });
        }
        unsafe {
            (*file).pMethods = ptr::null();
        }
        return ffi::SQLITE_CANTOPEN;
    }
    let entry = OpenFileState {
        vfs_state: (state as *const AnchoredVfsState) as usize,
        parent_methods: parent_methods as usize,
    };
    let Ok(mut files) = OPEN_FILES.get_or_init(|| Mutex::new(HashMap::new())).lock() else {
        return ffi::SQLITE_CANTOPEN;
    };
    files.insert(file as usize, entry);
    unsafe {
        (*file).pMethods = &ANCHORED_IO_METHODS;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn anchored_delete(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    sync_dir: c_int,
) -> c_int {
    if name.is_null() || !valid_relative_name(name) {
        return ffi::SQLITE_IOERR_DELETE;
    }
    let Some((state, callback)) = state(vfs)
        .and_then(|state| unsafe { (*state.parent).xDelete }.map(|callback| (state, callback)))
    else {
        return ffi::SQLITE_IOERR_DELETE;
    };
    with_owner_cwd(state, ffi::SQLITE_IOERR_DELETE, || unsafe {
        callback(state.parent, name, sync_dir)
    })
}

unsafe extern "C" fn anchored_access(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    flags: c_int,
    result: *mut c_int,
) -> c_int {
    if name.is_null() || result.is_null() || !valid_relative_name(name) {
        return ffi::SQLITE_IOERR_ACCESS;
    }
    let Some((state, callback)) = state(vfs)
        .and_then(|state| unsafe { (*state.parent).xAccess }.map(|callback| (state, callback)))
    else {
        return ffi::SQLITE_IOERR_ACCESS;
    };
    with_owner_cwd(state, ffi::SQLITE_IOERR_ACCESS, || unsafe {
        callback(state.parent, name, flags, result)
    })
}

unsafe extern "C" fn anchored_full_pathname(
    _vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    output_len: c_int,
    output: *mut c_char,
) -> c_int {
    if name.is_null() || output.is_null() || output_len <= 0 || !valid_relative_name(name) {
        return ffi::SQLITE_CANTOPEN;
    }
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes_with_nul();
    if bytes.len() > output_len as usize {
        return ffi::SQLITE_CANTOPEN;
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), output, bytes.len());
    }
    ffi::SQLITE_OK
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

fn with_owner_cwd(
    state: &AnchoredVfsState,
    failure: c_int,
    callback: impl FnOnce() -> c_int,
) -> c_int {
    let Ok(_guard) = CWD_GUARD.get_or_init(|| Mutex::new(())).lock() else {
        return failure;
    };
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let Ok(saved) = open(".", flags, Mode::empty()) else {
        return failure;
    };
    if fchdir(&state.owner).is_err() {
        return failure;
    }
    let result = callback();
    if fchdir(&saved).is_err() {
        return failure;
    }
    result
}

fn valid_relative_name(name: *const c_char) -> bool {
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes();
    !bytes.is_empty()
        && bytes != b"."
        && bytes != b".."
        && !bytes
            .iter()
            .any(|byte| matches!(*byte, b'/' | b'\\' | b'\0'))
}

static ANCHORED_IO_METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    iVersion: 3,
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
    xShmMap: Some(anchored_io_shm_map),
    xShmLock: Some(anchored_io_shm_lock),
    xShmBarrier: Some(anchored_io_shm_barrier),
    xShmUnmap: Some(anchored_io_shm_unmap),
    xFetch: Some(anchored_io_fetch),
    xUnfetch: Some(anchored_io_unfetch),
};

unsafe extern "C" fn anchored_io_close(file: *mut ffi::sqlite3_file) -> c_int {
    let result = with_file_cwd(file, ffi::SQLITE_IOERR_CLOSE, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xClose)
        else {
            return ffi::SQLITE_IOERR_CLOSE;
        };
        unsafe { callback(file) }
    });
    if let Ok(mut files) = OPEN_FILES.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        files.remove(&(file as usize));
    }
    result
}

unsafe extern "C" fn anchored_io_read(
    file: *mut ffi::sqlite3_file,
    output: *mut c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_READ, |methods| {
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
    with_file_cwd(file, ffi::SQLITE_IOERR_WRITE, |methods| {
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
    with_file_cwd(file, ffi::SQLITE_IOERR_TRUNCATE, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xTruncate)
        else {
            return ffi::SQLITE_IOERR_TRUNCATE;
        };
        unsafe { callback(file, size) }
    })
}

unsafe extern "C" fn anchored_io_sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_FSYNC, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xSync) else {
            return ffi::SQLITE_IOERR_FSYNC;
        };
        unsafe { callback(file, flags) }
    })
}

unsafe extern "C" fn anchored_io_file_size(
    file: *mut ffi::sqlite3_file,
    size: *mut ffi::sqlite3_int64,
) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_FSTAT, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xFileSize)
        else {
            return ffi::SQLITE_IOERR_FSTAT;
        };
        unsafe { callback(file, size) }
    })
}

unsafe extern "C" fn anchored_io_lock(file: *mut ffi::sqlite3_file, lock: c_int) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_LOCK, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xLock) else {
            return ffi::SQLITE_IOERR_LOCK;
        };
        unsafe { callback(file, lock) }
    })
}

unsafe extern "C" fn anchored_io_unlock(file: *mut ffi::sqlite3_file, lock: c_int) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_UNLOCK, |methods| {
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
    with_file_cwd(file, ffi::SQLITE_IOERR_CHECKRESERVEDLOCK, |methods| {
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
    with_file_cwd(file, ffi::SQLITE_IOERR, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xFileControl)
        else {
            return ffi::SQLITE_NOTFOUND;
        };
        unsafe { callback(file, op, argument) }
    })
}

unsafe extern "C" fn anchored_io_sector_size(file: *mut ffi::sqlite3_file) -> c_int {
    with_file_cwd(file, 0, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xSectorSize)
        else {
            return 0;
        };
        unsafe { callback(file) }
    })
}

unsafe extern "C" fn anchored_io_device_characteristics(file: *mut ffi::sqlite3_file) -> c_int {
    with_file_cwd(file, 0, |methods| {
        let Some(callback) =
            (unsafe { methods.as_ref() }).and_then(|methods| methods.xDeviceCharacteristics)
        else {
            return 0;
        };
        unsafe { callback(file) }
    })
}

unsafe extern "C" fn anchored_io_shm_map(
    file: *mut ffi::sqlite3_file,
    page: c_int,
    page_size: c_int,
    extend: c_int,
    output: *mut *mut c_void,
) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_SHMMAP, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xShmMap)
        else {
            return ffi::SQLITE_IOERR_SHMMAP;
        };
        unsafe { callback(file, page, page_size, extend, output) }
    })
}

unsafe extern "C" fn anchored_io_shm_lock(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_SHMLOCK, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xShmLock)
        else {
            return ffi::SQLITE_IOERR_SHMLOCK;
        };
        unsafe { callback(file, offset, count, flags) }
    })
}

unsafe extern "C" fn anchored_io_shm_barrier(file: *mut ffi::sqlite3_file) {
    let _ = with_file_cwd(file, ffi::SQLITE_IOERR, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xShmBarrier)
        else {
            return ffi::SQLITE_OK;
        };
        unsafe { callback(file) };
        ffi::SQLITE_OK
    });
}

unsafe extern "C" fn anchored_io_shm_unmap(file: *mut ffi::sqlite3_file, delete: c_int) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR_SHMOPEN, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xShmUnmap)
        else {
            return ffi::SQLITE_IOERR_SHMOPEN;
        };
        unsafe { callback(file, delete) }
    })
}

unsafe extern "C" fn anchored_io_fetch(
    file: *mut ffi::sqlite3_file,
    offset: ffi::sqlite3_int64,
    amount: c_int,
    output: *mut *mut c_void,
) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xFetch)
        else {
            return ffi::SQLITE_OK;
        };
        unsafe { callback(file, offset, amount, output) }
    })
}

unsafe extern "C" fn anchored_io_unfetch(
    file: *mut ffi::sqlite3_file,
    offset: ffi::sqlite3_int64,
    pointer: *mut c_void,
) -> c_int {
    with_file_cwd(file, ffi::SQLITE_IOERR, |methods| {
        let Some(callback) = (unsafe { methods.as_ref() }).and_then(|methods| methods.xUnfetch)
        else {
            return ffi::SQLITE_OK;
        };
        unsafe { callback(file, offset, pointer) }
    })
}

fn with_file_cwd(
    file: *mut ffi::sqlite3_file,
    failure: c_int,
    callback: impl FnOnce(*const ffi::sqlite3_io_methods) -> c_int,
) -> c_int {
    let Some(entry) = OPEN_FILES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|files| files.get(&(file as usize)).copied())
    else {
        return failure;
    };
    let state = unsafe { (entry.vfs_state as *const AnchoredVfsState).as_ref() };
    let Some(state) = state else {
        return failure;
    };
    with_owner_cwd(state, failure, || {
        callback(entry.parent_methods as *const ffi::sqlite3_io_methods)
    })
}
