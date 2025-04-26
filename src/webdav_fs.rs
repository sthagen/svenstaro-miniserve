//! Helper types and functions to allow configuring hidden files visibility
//! for WebDAV handlers

use dav_server::{
    davpath::DavPath,
    fs::{
        DavDirEntry, DavFile, DavFileSystem, DavMetaData, FsError as DavFsError,
        FsFuture as DavFsFuture, FsStream as DavFsStream, OpenOptions as DavOpenOptions,
        ReadDirMeta as DavReadDirMeta,
    },
    localfs::LocalFs,
};
use futures::{StreamExt, TryFutureExt, future::ready};
use std::path::{Component, Path};

/// A dav_server local filesystem backend that can be configured to deny access
/// to files and directories with names starting with a dot.
#[derive(Clone)]
pub struct RestrictedFs {
    local: Box<LocalFs>,
    show_hidden: bool,
}

impl RestrictedFs {
    /// Creates a new RestrictedFs serving the local path at "base".
    /// If "show_hidden" is false, access to hidden files is prevented.
    pub fn new<P: AsRef<Path>>(base: P, show_hidden: bool) -> Box<RestrictedFs> {
        let local = LocalFs::new(base, false, false, false);
        Box::new(RestrictedFs { local, show_hidden })
    }
}

/// true if any normal component of path either starts with dot or can't be turned into a str
fn path_has_hidden_components(path: &DavPath) -> bool {
    path.as_pathbuf().components().any(|c| match c {
        Component::Normal(name) => name.to_str().is_none_or(|s| s.starts_with('.')),
        _ => false,
    })
}

impl DavFileSystem for RestrictedFs {
    fn open<'a>(
        &'a self,
        path: &'a DavPath,
        options: DavOpenOptions,
    ) -> DavFsFuture<'a, Box<dyn DavFile>> {
        if !path_has_hidden_components(path) || self.show_hidden {
            self.local.open(path, options)
        } else {
            Box::pin(ready(Err(DavFsError::NotFound)))
        }
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        meta: DavReadDirMeta,
    ) -> DavFsFuture<'a, DavFsStream<Box<dyn DavDirEntry>>> {
        if self.show_hidden {
            self.local.read_dir(path, meta)
        } else if !path_has_hidden_components(path) {
            Box::pin(self.local.read_dir(path, meta).map_ok(|stream| {
                let dyn_stream: DavFsStream<Box<dyn DavDirEntry>> =
                    Box::pin(stream.filter(|entry| {
                        ready(match entry {
                            Ok(e) => !e.name().starts_with(b"."),
                            _ => false,
                        })
                    }));
                dyn_stream
            }))
        } else {
            Box::pin(ready(Err(DavFsError::NotFound)))
        }
    }

    fn metadata<'a>(&'a self, path: &'a DavPath) -> DavFsFuture<'a, Box<dyn DavMetaData>> {
        if !path_has_hidden_components(path) || self.show_hidden {
            self.local.metadata(path)
        } else {
            Box::pin(ready(Err(DavFsError::NotFound)))
        }
    }

    fn symlink_metadata<'a>(&'a self, path: &'a DavPath) -> DavFsFuture<'a, Box<dyn DavMetaData>> {
        if !path_has_hidden_components(path) || self.show_hidden {
            self.local.symlink_metadata(path)
        } else {
            Box::pin(ready(Err(DavFsError::NotFound)))
        }
    }
}
