window.SIDEBAR_ITEMS = {"constant":[["IDX_DTYPE",""],["NULL_DTYPE",""]],"enum":[["AnyValue",""],["ArrowDataType","The set of supported logical types in this crate."],["ArrowTimeUnit","The time units defined in Arrow."],["AsofStrategy",""],["DataType",""],["FillNullStrategy",""],["GroupsProxy",""],["JoinType",""],["PolarsError",""],["QuantileInterpolOptions",""],["RankMethod",""],["RevMapping",""],["RevMappingBuilder",""],["TakeIdx","One of the three arguments allowed in unchecked_take"],["TakeRandBranch2",""],["TakeRandBranch3",""],["TimeUnit",""],["UniqueKeepStrategy",""]],"fn":[["datetime_to_timestamp_ms",""],["datetime_to_timestamp_ns",""],["datetime_to_timestamp_us",""],["merge_dtypes",""]],"macro":[["df",""]],"mod":[["builder",""],["categorical",""],["datatypes","Data types supported by Polars."],["default_arrays",""],["list",""],["registry",""],["stringcache",""]],"struct":[["ArrowField","Represents Arrow’s metadata of a “column”."],["ArrowSchema","An ordered sequence of [`Field`]s with associated [`Metadata`]."],["AsOfOptions",""],["BinaryChunkedBuilder",""],["BinaryTakeRandom",""],["BinaryTakeRandomSingleChunk",""],["BinaryType",""],["BoolTakeRandom",""],["BoolTakeRandomSingleChunk",""],["BooleanChunkedBuilder",""],["BooleanType",""],["CatIter",""],["CategoricalChunked",""],["CategoricalChunkedBuilder",""],["CategoricalType",""],["ChunkedArray","ChunkedArray"],["DataFrame","A contiguous growable collection of `Series` that have the same length."],["DateType",""],["DatetimeType",""],["DurationType",""],["Field","Characterizes the name and the [`DataType`] of a column."],["Float32Type",""],["Float64Type",""],["GroupsIdx","Indexes of the groups, the first index is stored separately. this make sorting fast."],["Int16Type",""],["Int32Type",""],["Int64Type",""],["Int8Type",""],["ListBinaryChunkedBuilder",""],["ListBooleanChunkedBuilder",""],["ListPrimitiveChunkedBuilder",""],["ListTakeRandom",""],["ListTakeRandomSingleChunk",""],["ListType",""],["ListUtf8ChunkedBuilder",""],["Logical","Maps a logical type to a a chunked array implementation of the physical type. This saves a lot of compiler bloat and allows us to reuse functionality."],["MeltArgs","Arguments for `[DataFrame::melt]` function"],["NumTakeRandomChunked",""],["NumTakeRandomCont",""],["NumTakeRandomSingleChunk",""],["ObjectArray",""],["ObjectTakeRandom",""],["ObjectTakeRandomSingleChunk",""],["ObjectType",""],["PrimitiveChunkedBuilder",""],["RankOptions",""],["RollingOptionsFixedWindow",""],["Schema",""],["Series","Series"],["SortOptions",""],["StrHashLocal",""],["StructChunked","This is logical type [`StructChunked`] that dispatches most logic to the `fields` implementations"],["TakeRandomBitmap",""],["TimeType",""],["UInt16Type",""],["UInt32Type",""],["UInt64Type",""],["UInt8Type",""],["Utf8ChunkedBuilder",""],["Utf8TakeRandom",""],["Utf8TakeRandomSingleChunk",""],["Utf8Type",""]],"trait":[["ArgAgg","Argmin/ Argmax"],["ArrowGetItem",""],["ChunkAgg","Aggregation operations"],["ChunkAggSeries","Aggregations that return Series of unit length. Those can be used in broadcasting operations."],["ChunkAnyValue",""],["ChunkApply","Fastest way to do elementwise operations on a ChunkedArray when the operation is cheaper than branching due to null checking"],["ChunkApplyKernel","Apply kernels on the arrow array chunks in a ChunkedArray."],["ChunkBytes",""],["ChunkCast","Cast `ChunkedArray<T>` to `ChunkedArray<N>`"],["ChunkCompare","Compare Series and ChunkedArray’s and get a `boolean` mask that can be used to filter rows."],["ChunkCumAgg",""],["ChunkExpandAtIndex","Create a new ChunkedArray filled with values at that index."],["ChunkExplode","Explode/ flatten a List or Utf8 Series"],["ChunkFillNull","Replace None values with various strategies"],["ChunkFillNullValue","Replace None values with a value"],["ChunkFilter","Filter values by a boolean mask."],["ChunkFull","Fill a ChunkedArray with one value."],["ChunkFullNull",""],["ChunkPeaks","Find local minima/ maxima"],["ChunkQuantile","Quantile and median aggregation"],["ChunkReverse","Reverse a ChunkedArray"],["ChunkRollApply","This differs from ChunkWindowCustom and ChunkWindow by not using a fold aggregator, but reusing a `Series` wrapper and calling `Series` aggregators. This likely is a bit slower than ChunkWindow"],["ChunkSet","Create a `ChunkedArray` with new values by index or by boolean mask. Note that these operations clone data. This is however the only way we can modify at mask or index level as the underlying Arrow arrays are immutable."],["ChunkShift",""],["ChunkShiftFill","Shift the values of a ChunkedArray by a number of periods."],["ChunkSort","Sort operations on `ChunkedArray`."],["ChunkTake","Fast access by index."],["ChunkTakeEvery","Traverse and collect every nth element"],["ChunkUnique","Get unique values in a `ChunkedArray`"],["ChunkVar","Variance and standard deviation aggregation."],["ChunkZip","Combine 2 ChunkedArrays based on some predicate."],["ChunkedBuilder",""],["FromData",""],["FromDataBinary",""],["FromDataUtf8",""],["GetAnyValue",""],["IndexOfSchema","This trait exists to be unify the API of polars Schema and arrows Schema"],["IndexToUsize",""],["InitHashMaps",""],["Interpolate",""],["IntoGroupsProxy","Used to create the tuples for a groupby operation."],["IntoSeries","Used to convert a [`ChunkedArray`], `&dyn SeriesTrait` and [`Series`] into a [`Series`]."],["IntoTakeRandom","Create a type that implements a faster `TakeRandom`."],["IntoVec",""],["IsFirst","Mask the first unique values as `true`"],["IsFloat","Safety"],["IsIn","Check if element is member of list array"],["IsLast","Mask the last unique values as `true`"],["LhsNumOps",""],["ListBuilderTrait",""],["ListFromIter",""],["LogicalType",""],["MutableBitmapExtension",""],["NamedFrom",""],["NamedFromOwned",""],["NewChunkedArray",""],["NumOpsDispatch",""],["NumOpsDispatchChecked",""],["NumericNative",""],["PolarsArray",""],["PolarsDataType",""],["PolarsFloatType",""],["PolarsIntegerType",""],["PolarsIterator","A `PolarsIterator` is an iterator over a `ChunkedArray` which contains polars types. A `PolarsIterator` must implement `ExactSizeIterator` and `DoubleEndedIterator`."],["PolarsNumericType",""],["PolarsObject","Values need to implement this so that they can be stored into a Series and DataFrame"],["PolarsObjectSafe","Trimmed down object safe polars object"],["PolarsSingleType","Any type that is not nested"],["QuantileAggSeries",""],["RepeatBy","Repeat the values `n` times."],["SeriesTrait",""],["StrConcat","Concat the values into a string array."],["TakeIterator",""],["TakeIteratorNulls",""],["TakeRandom","Random access"],["TakeRandomUtf8",""],["ValueSize",""],["VarAggSeries",""],["VecHash",""]],"type":[["ArrayRef",""],["BinaryChunked",""],["BooleanChunked",""],["DateChunked",""],["DatetimeChunked",""],["Dummy","Dummy type, we need to instantiate all generic types, so we fill one with a dummy."],["DurationChunked",""],["FillNullLimit",""],["Float32Chunked",""],["Float64Chunked",""],["GroupsSlice","Every group is indicated by an array where the"],["IdxArr",""],["IdxCa",""],["IdxSize","The type used by polars to index data."],["IdxType",""],["Int16Chunked",""],["Int32Chunked",""],["Int64Chunked",""],["Int8Chunked",""],["LargeBinaryArray",""],["LargeListArray",""],["LargeStringArray",""],["ListChunked",""],["ObjectChunked",""],["PlHashMap",""],["PlHashSet",""],["PlIdHashMap","This hashmap has the uses an IdHasher"],["PlIndexMap",""],["PlIndexSet",""],["PolarsResult",""],["SchemaRef",""],["TimeChunked",""],["TimeZone",""],["UInt16Chunked",""],["UInt32Chunked",""],["UInt64Chunked",""],["UInt8Chunked",""],["Utf8Chunked",""]]};