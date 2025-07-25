use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::sync::Arc;

use polars_core::error::{PolarsResult, feature_gated};
use polars_io::cloud::CloudOptions;
#[cfg(feature = "cloud")]
use polars_io::file_cache::FileCacheEntry;
#[cfg(feature = "cloud")]
use polars_io::utils::byte_source::{DynByteSource, DynByteSourceBuilder};
use polars_io::{expand_paths, expand_paths_hive, expanded_from_single_directory};
use polars_utils::mmap::MemSlice;
use polars_utils::pl_str::PlSmallStr;
use polars_utils::plpath::{PlPath, PlPathRef};

use super::UnifiedScanArgs;

/// Set of sources to scan from
///
/// This can either be a list of paths to files, opened files or in-memory buffers. Mixing of
/// buffers is not currently possible.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "dsl-schema", derive(schemars::JsonSchema))]
#[derive(Clone)]
pub enum ScanSources {
    Paths(Arc<[PlPath]>),

    #[cfg_attr(any(feature = "serde", feature = "dsl-schema"), serde(skip))]
    Files(Arc<[File]>),
    #[cfg_attr(any(feature = "serde", feature = "dsl-schema"), serde(skip))]
    Buffers(Arc<[MemSlice]>),
}

impl Debug for ScanSources {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paths(p) => write!(f, "paths: {:?}", p.as_ref()),
            Self::Files(p) => write!(f, "files: {} files", p.len()),
            Self::Buffers(b) => write!(f, "buffers: {} in-memory-buffers", b.len()),
        }
    }
}

/// A reference to a single item in [`ScanSources`]
#[derive(Debug, Clone, Copy)]
pub enum ScanSourceRef<'a> {
    Path(PlPathRef<'a>),
    File(&'a File),
    Buffer(&'a MemSlice),
}

/// A single source to scan from
#[derive(Debug, Clone)]
pub enum ScanSource {
    Path(PlPath),
    File(Arc<File>),
    Buffer(MemSlice),
}

impl ScanSource {
    pub fn from_sources(sources: ScanSources) -> Result<Self, ScanSources> {
        if sources.len() == 1 {
            match sources {
                ScanSources::Paths(ps) => Ok(Self::Path(ps.as_ref()[0].clone())),
                ScanSources::Files(fs) => {
                    assert_eq!(fs.len(), 1);
                    let ptr: *const File = Arc::into_raw(fs) as *const File;
                    // SAFETY: A [T] with length 1 can be interpreted as T
                    let f: Arc<File> = unsafe { Arc::from_raw(ptr) };

                    Ok(Self::File(f))
                },
                ScanSources::Buffers(bs) => Ok(Self::Buffer(bs.as_ref()[0].clone())),
            }
        } else {
            Err(sources)
        }
    }

    pub fn into_sources(self) -> ScanSources {
        match self {
            ScanSource::Path(p) => ScanSources::Paths([p].into()),
            ScanSource::File(f) => {
                let ptr: *const [File] = std::ptr::slice_from_raw_parts(Arc::into_raw(f), 1);
                // SAFETY: A T can be interpreted as [T] with length 1.
                let fs: Arc<[File]> = unsafe { Arc::from_raw(ptr) };
                ScanSources::Files(fs)
            },
            ScanSource::Buffer(m) => ScanSources::Buffers([m].into()),
        }
    }

    pub fn as_scan_source_ref(&self) -> ScanSourceRef<'_> {
        match self {
            ScanSource::Path(path) => ScanSourceRef::Path(path.as_ref()),
            ScanSource::File(file) => ScanSourceRef::File(file.as_ref()),
            ScanSource::Buffer(mem_slice) => ScanSourceRef::Buffer(mem_slice),
        }
    }

    pub fn run_async(&self) -> bool {
        self.as_scan_source_ref().run_async()
    }

    pub fn is_cloud_url(&self) -> bool {
        if let ScanSource::Path(path) = self {
            path.is_cloud_url()
        } else {
            false
        }
    }
}

/// An iterator for [`ScanSources`]
pub struct ScanSourceIter<'a> {
    sources: &'a ScanSources,
    offset: usize,
}

impl Default for ScanSources {
    fn default() -> Self {
        // We need to use `Paths` here to avoid erroring when doing hive-partitioned scans of empty
        // file lists.
        Self::Paths(Arc::default())
    }
}

impl std::hash::Hash for ScanSources {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);

        // @NOTE: This is a bit crazy
        //
        // We don't really want to hash the file descriptors or the whole buffers so for now we
        // just settle with the fact that the memory behind Arc's does not really move. Therefore,
        // we can just hash the pointer.
        match self {
            Self::Paths(paths) => paths.hash(state),
            Self::Files(files) => files.as_ptr().hash(state),
            Self::Buffers(buffers) => buffers.as_ptr().hash(state),
        }
    }
}

impl PartialEq for ScanSources {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ScanSources::Paths(l), ScanSources::Paths(r)) => l == r,
            (ScanSources::Files(l), ScanSources::Files(r)) => std::ptr::eq(l.as_ptr(), r.as_ptr()),
            (ScanSources::Buffers(l), ScanSources::Buffers(r)) => {
                std::ptr::eq(l.as_ptr(), r.as_ptr())
            },
            _ => false,
        }
    }
}

impl Eq for ScanSources {}

impl ScanSources {
    pub fn expand_paths(
        &self,
        scan_args: &UnifiedScanArgs,
        #[allow(unused_variables)] cloud_options: Option<&CloudOptions>,
    ) -> PolarsResult<Self> {
        match self {
            Self::Paths(paths) => Ok(Self::Paths(expand_paths(
                paths,
                scan_args.glob,
                cloud_options,
            )?)),
            v => Ok(v.clone()),
        }
    }

    /// This will update `scan_args.hive_options.enabled` to `true` if the existing value is `None`
    /// and the paths are expanded from a single directory. Otherwise the existing value is maintained.
    #[cfg(any(feature = "ipc", feature = "parquet"))]
    pub fn expand_paths_with_hive_update(
        &self,
        scan_args: &mut UnifiedScanArgs,
        #[allow(unused_variables)] cloud_options: Option<&CloudOptions>,
    ) -> PolarsResult<Self> {
        match self {
            Self::Paths(paths) => {
                let (expanded_paths, hive_start_idx) = expand_paths_hive(
                    paths,
                    scan_args.glob,
                    cloud_options,
                    scan_args.hive_options.enabled.unwrap_or(false),
                )?;

                if scan_args.hive_options.enabled.is_none()
                    && expanded_from_single_directory(paths, expanded_paths.as_ref())
                {
                    scan_args.hive_options.enabled = Some(true);
                }
                scan_args.hive_options.hive_start_idx = hive_start_idx;

                Ok(Self::Paths(expanded_paths))
            },
            v => Ok(v.clone()),
        }
    }

    pub fn iter(&self) -> ScanSourceIter<'_> {
        ScanSourceIter {
            sources: self,
            offset: 0,
        }
    }

    /// Are the sources all paths?
    pub fn is_paths(&self) -> bool {
        matches!(self, Self::Paths(_))
    }

    /// Try cast the scan sources to [`ScanSources::Paths`]
    pub fn as_paths(&self) -> Option<&[PlPath]> {
        match self {
            Self::Paths(paths) => Some(paths.as_ref()),
            Self::Files(_) | Self::Buffers(_) => None,
        }
    }

    /// Try cast the scan sources to [`ScanSources::Paths`] with a clone
    pub fn into_paths(&self) -> Option<Arc<[PlPath]>> {
        match self {
            Self::Paths(paths) => Some(paths.clone()),
            Self::Files(_) | Self::Buffers(_) => None,
        }
    }

    /// Try get the first path in the scan sources
    pub fn first_path(&self) -> Option<PlPathRef<'_>> {
        match self {
            Self::Paths(paths) => paths.first().map(|p| p.as_ref()),
            Self::Files(_) | Self::Buffers(_) => None,
        }
    }

    /// Is the first path a cloud URL?
    pub fn is_cloud_url(&self) -> bool {
        self.first_path().is_some_and(|path| path.is_cloud_url())
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Paths(s) => s.len(),
            Self::Files(s) => s.len(),
            Self::Buffers(s) => s.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn first(&self) -> Option<ScanSourceRef<'_>> {
        self.get(0)
    }

    /// Turn the [`ScanSources`] into some kind of identifier
    pub fn id(&self) -> PlSmallStr {
        if self.is_empty() {
            return PlSmallStr::from_static("EMPTY");
        }

        match self {
            Self::Paths(paths) => PlSmallStr::from_str(paths.first().unwrap().to_str()),
            Self::Files(_) => PlSmallStr::from_static("OPEN_FILES"),
            Self::Buffers(_) => PlSmallStr::from_static("IN_MEMORY"),
        }
    }

    /// Get the scan source at specific address
    pub fn get(&self, idx: usize) -> Option<ScanSourceRef<'_>> {
        match self {
            Self::Paths(paths) => paths.get(idx).map(|p| ScanSourceRef::Path(p.as_ref())),
            Self::Files(files) => files.get(idx).map(ScanSourceRef::File),
            Self::Buffers(buffers) => buffers.get(idx).map(ScanSourceRef::Buffer),
        }
    }

    /// Get the scan source at specific address
    ///
    /// # Panics
    ///
    /// If the `idx` is out of range.
    #[track_caller]
    pub fn at(&self, idx: usize) -> ScanSourceRef<'_> {
        self.get(idx).unwrap()
    }
}

impl ScanSourceRef<'_> {
    /// Get the name for `include_paths`
    pub fn to_include_path_name(&self) -> &str {
        match self {
            Self::Path(path) => path.to_str(),
            Self::File(_) => "open-file",
            Self::Buffer(_) => "in-mem",
        }
    }

    // @TODO: I would like to remove this function eventually.
    pub fn into_owned(&self) -> PolarsResult<ScanSource> {
        Ok(match self {
            ScanSourceRef::Path(path) => ScanSource::Path((*path).into_owned()),
            ScanSourceRef::File(file) => {
                if let Ok(file) = file.try_clone() {
                    ScanSource::File(Arc::new(file))
                } else {
                    ScanSource::Buffer(self.to_memslice()?)
                }
            },
            ScanSourceRef::Buffer(buffer) => ScanSource::Buffer((*buffer).clone()),
        })
    }

    /// Turn the scan source into a memory slice
    pub fn to_memslice(&self) -> PolarsResult<MemSlice> {
        self.to_memslice_possibly_async(false, None, 0)
    }

    #[allow(clippy::wrong_self_convention)]
    #[cfg(feature = "cloud")]
    fn to_memslice_async<F: Fn(Arc<FileCacheEntry>) -> PolarsResult<std::fs::File>>(
        &self,
        assume: F,
        run_async: bool,
    ) -> PolarsResult<MemSlice> {
        match self {
            ScanSourceRef::Path(path) => {
                let file = if run_async {
                    feature_gated!("cloud", {
                        // This isn't filled if we modified the DSL (e.g. in cloud)
                        let entry = polars_io::file_cache::FILE_CACHE.get_entry(*path);

                        if let Some(entry) = entry {
                            assume(entry)?
                        } else {
                            polars_utils::open_file(path.as_local_path().unwrap())?
                        }
                    })
                } else {
                    polars_utils::open_file(path.as_local_path().unwrap())?
                };

                MemSlice::from_file(&file)
            },
            ScanSourceRef::File(file) => MemSlice::from_file(file),
            ScanSourceRef::Buffer(buff) => Ok((*buff).clone()),
        }
    }

    #[cfg(feature = "cloud")]
    pub fn to_memslice_async_assume_latest(&self, run_async: bool) -> PolarsResult<MemSlice> {
        self.to_memslice_async(|entry| entry.try_open_assume_latest(), run_async)
    }

    #[cfg(feature = "cloud")]
    pub fn to_memslice_async_check_latest(&self, run_async: bool) -> PolarsResult<MemSlice> {
        self.to_memslice_async(|entry| entry.try_open_check_latest(), run_async)
    }

    #[cfg(not(feature = "cloud"))]
    #[allow(clippy::wrong_self_convention)]
    fn to_memslice_async(&self, run_async: bool) -> PolarsResult<MemSlice> {
        match self {
            ScanSourceRef::Path(path) => {
                let file = polars_utils::open_file(path.as_local_path().unwrap())?;
                MemSlice::from_file(&file)
            },
            ScanSourceRef::File(file) => MemSlice::from_file(file),
            ScanSourceRef::Buffer(buff) => Ok((*buff).clone()),
        }
    }

    #[cfg(not(feature = "cloud"))]
    pub fn to_memslice_async_assume_latest(&self, run_async: bool) -> PolarsResult<MemSlice> {
        self.to_memslice_async(run_async)
    }

    #[cfg(not(feature = "cloud"))]
    pub fn to_memslice_async_check_latest(&self, run_async: bool) -> PolarsResult<MemSlice> {
        self.to_memslice_async(run_async)
    }

    pub fn to_memslice_possibly_async(
        &self,
        run_async: bool,
        #[cfg(feature = "cloud")] cache_entries: Option<
            &Vec<Arc<polars_io::file_cache::FileCacheEntry>>,
        >,
        #[cfg(not(feature = "cloud"))] cache_entries: Option<&()>,
        index: usize,
    ) -> PolarsResult<MemSlice> {
        match self {
            Self::Path(path) => {
                let file = if run_async {
                    feature_gated!("cloud", {
                        cache_entries.unwrap()[index].try_open_check_latest()?
                    })
                } else {
                    polars_utils::open_file(path.as_local_path().unwrap())?
                };

                MemSlice::from_file(&file)
            },
            Self::File(file) => MemSlice::from_file(file),
            Self::Buffer(buff) => Ok((*buff).clone()),
        }
    }

    #[cfg(feature = "cloud")]
    pub async fn to_dyn_byte_source(
        &self,
        builder: &DynByteSourceBuilder,
        cloud_options: Option<&CloudOptions>,
    ) -> PolarsResult<DynByteSource> {
        match self {
            Self::Path(path) => {
                builder
                    .try_build_from_path(path.to_str(), cloud_options)
                    .await
            },
            Self::File(file) => Ok(DynByteSource::from(MemSlice::from_file(file)?)),
            Self::Buffer(buff) => Ok(DynByteSource::from((*buff).clone())),
        }
    }

    pub(crate) fn run_async(&self) -> bool {
        matches!(self, Self::Path(p) if p.is_cloud_url() || polars_core::config::force_async())
    }
}

impl<'a> Iterator for ScanSourceIter<'a> {
    type Item = ScanSourceRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = match self.sources {
            ScanSources::Paths(paths) => ScanSourceRef::Path(paths.get(self.offset)?.as_ref()),
            ScanSources::Files(files) => ScanSourceRef::File(files.get(self.offset)?),
            ScanSources::Buffers(buffers) => ScanSourceRef::Buffer(buffers.get(self.offset)?),
        };

        self.offset += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.sources.len() - self.offset;
        (len, Some(len))
    }
}

impl ExactSizeIterator for ScanSourceIter<'_> {}
