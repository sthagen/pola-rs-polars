pub(crate) mod groupby;
mod joins;
mod ordered;
mod slice;
mod utils;
mod parquet_sink;

pub(crate) use joins::*;
pub(crate) use ordered::*;
pub(crate) use slice::*;

// We must strike a balance between cache coherence and resizing costs.
// Overallocation seems a lot more expensive than resizing so we start reasonable small.
const HASHMAP_INIT_SIZE: usize = 64;
