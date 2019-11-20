//! RPM database access
//!
//! The database used is whichever one is configured as the `_dbpath` in the
//! in the global macro context. By default this is unset: you will need to
//! call `librpm::config::read_file(None)` to read the default "rpmrc"
//! configuration.
//!
//! # Example
//!
//! Finding the "rpm-devel" RPM in the database:
//!
//! ```
//! use librpm::Index;
//!
//! librpm::config::read_file(None).unwrap();
//!
//! let mut matches = Index::Name.find("rpm-devel");
//! let package = matches.next().unwrap();
//!
//! println!("package name: {}", package.name);
//! println!("package summary: {}", package.summary);
//! println!("package version: {}", package.version);
//! ```

use crate::error::{Error, ErrorKind};
use crate::internal::{iterator::MatchIterator, tag::Tag};
use crate::package::Package;
use streaming_iterator::StreamingIterator;

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

struct Db {}

struct DbBuilder<P>
where
    P: AsRef<Path>,
{
    config: Option<P>,
}

impl<P> Default for DbBuilder<P>
where
    P: AsRef<Path>,
{
    fn default() -> Self {
        Self { config: None }
    }
}

impl Db {
    fn open<P>() -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        DbBuilder::<&Path>::new().open()
    }

    fn open_with<P>() -> DbBuilder<P>
    where
        P: AsRef<Path>,
    {
        DbBuilder::default()
    }
}

impl<P> DbBuilder<P>
where
    P: AsRef<Path>,
{
    fn new() -> Self {
        Self::default()
    }

    fn with_config(&mut self, config: P) {
        self.config = Some(config);
    }
    
    fn open(self) -> Result<Db, Error> {
        let rc = match self.config {
            Some(ref path) => {
                if !path.as_ref().exists() {
                    fail!(
                        ErrorKind::Config,
                        "no such file: {}",
                        path.as_ref().display()
                    )
                }
                let cstr = CString::new(path.as_ref().as_os_str().as_bytes()).map_err(|e| {
                    format_err!(
                        ErrorKind::Config,
                        "invalid path: {} ({})",
                        path.as_ref().display(),
                        e
                    )
                })?;
                unsafe { librpm_sys::rpmReadConfigFiles(cstr.as_ptr(), ptr::null()) }
            }
            None => unsafe { librpm_sys::rpmReadConfigFiles(ptr::null(), ptr::null()) },
        };
        if rc != 0 {
            match self.config {
                Some(path) => fail!(
                    ErrorKind::Config,
                    "error reading RPM config from: {}",
                    path.as_ref().display()
                ),
                None => fail!(
                    ErrorKind::Config,
                    "error reading RPM config from default location"
                ),
            }
        }
        Err(Error::new(ErrorKind::Config, None))
    }
}

/// Iterator over the RPM database which returns `Package` structs.
pub struct Iter(MatchIterator);

impl Iterator for Iter {
    type Item = Package;

    /// Obtain the next header from the iterator.
    fn next(&mut self) -> Option<Package> {
        self.0.next().map(|h| h.to_package())
    }
}

/// Searchable fields in the RPM package headers.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Index {
    /// Search by package name.
    Name,

    /// Search by package version.
    Version,

    /// Search by package license.
    License,

    /// Search by package summary.
    Summary,

    /// Search by package description.
    Description,
}

impl Index {
    /// Find an exact match in the given index
    pub fn find<S: AsRef<str>>(self, key: S) -> Iter {
        Iter(MatchIterator::new(self.into(), Some(key.as_ref())))
    }
}

impl Into<Tag> for Index {
    fn into(self) -> Tag {
        match self {
            Index::Name => Tag::NAME,
            Index::Version => Tag::VERSION,
            Index::License => Tag::LICENSE,
            Index::Summary => Tag::SUMMARY,
            Index::Description => Tag::DESCRIPTION,
        }
    }
}

/// Find all packages installed on the local system.
pub fn installed_packages() -> Iter {
    Iter(MatchIterator::new(Tag::NAME, None))
}

/// Find installed packages with a search key that exactly matches the given tag.
///
/// Panics if the glob contains null bytes.
pub fn find<S: AsRef<str>>(index: Index, key: S) -> Iter {
    index.find(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn db_opens() {
        Db::open::<&Path>().unwrap();
    }
}
