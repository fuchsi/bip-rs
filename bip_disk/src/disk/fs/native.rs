use std::path::{Path, PathBuf};
use std::io::{self, Write, Read, Seek, SeekFrom};
use std::fs::{self, File, OpenOptions};
use std::borrow::Cow;

use bip_util::bt::InfoHash;
use bip_util::sha::ShaHashBuilder;
use bip_util::convert;

use disk::fs::FileSystem;

// TODO: This should be sanitizing paths passed into it so they don't escape the base directory!!!

/// File that exists on disk.
pub struct NativeFile {
    file: File,
    path: PathBuf,
}

impl NativeFile {
    /// Create a new NativeFile.
    fn new(file: File, path: PathBuf) -> NativeFile {
        NativeFile{ file: file, path: path }
    }
}

/// File system that maps to the OS file system.
pub struct NativeFileSystem {
    current_dir: PathBuf
}

impl NativeFileSystem {
    /// Initialize a new NativeFileSystem with the default directory set.
    pub fn with_directory<P>(default: P) -> NativeFileSystem
        where P: AsRef<Path> {
        NativeFileSystem{ current_dir: default.as_ref().to_path_buf() }
    }
}

/// Create a hex representation of the given InfoHash.
fn info_hash_to_hex(hash: InfoHash) -> String {
    let hex_len = hash.as_ref().len() * 2;

    hash.as_ref()
        .iter()
        .map(|b| format!("{:02X}", b))
        .fold(String::with_capacity(hex_len), |mut acc, nex| {
            acc.push_str(&nex);
            acc
        })
}

impl FileSystem for NativeFileSystem {
    type File = NativeFile;

    fn open_file<P>(&self, path: P) -> io::Result<Self::File>
        where P: AsRef<Path> + Send + 'static {
        let combine_path = combine_user_path(&path, &self.current_dir);
        let file = try!(create_new_file(&combine_path));

        Ok(NativeFile::new(file, combine_path.into_owned()))
    }

    fn file_size(&self, file: &NativeFile) -> io::Result<u64> {
        file.file.metadata().map(|metadata| metadata.len())
    }

    fn remove_file(&self, file: NativeFile) -> io::Result<()> {
        fs::remove_file(&file.path)
    }

    fn read_file(&self, file: &mut NativeFile, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        try!(file.file.seek(SeekFrom::Start(offset)));

        file.file.read(buffer)
    }

    fn write_file(&self, file: &mut NativeFile, offset: u64, buffer: &[u8]) -> io::Result<usize> {
        try!(file.file.seek(SeekFrom::Start(offset)));

        file.file.write(buffer)
    }
}

/// Create a new file with read and write options.
///
/// Intermediate directories will be created if they do not exist.
fn create_new_file<P>(path: P) -> io::Result<File>
    where P: AsRef<Path> {
    match path.as_ref().parent() {
        Some(parent_dir) => {
            try!(fs::create_dir_all(parent_dir));

            OpenOptions::new().read(true).write(true).create(true).open(&path)
        },
        None => {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "File Path Has No Parent"))
        }
    }
}

/// Create a path from the user path and current directory.
fn combine_user_path<'a, P>(user_path: &'a P, current_dir: &Path) -> Cow<'a, Path>
    where P: AsRef<Path> {
    let ref_user_path = user_path.as_ref();

    if ref_user_path.is_absolute() {
        Cow::Borrowed(ref_user_path)
    } else {
        let mut combine_user_path = current_dir.to_path_buf();

        for user_path_piece in ref_user_path.iter() {
            combine_user_path.push(user_path_piece);
        }
        
        Cow::Owned(combine_user_path)
    }
}