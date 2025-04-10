use avro_schema::schema::{Enum, Fixed, Record, Schema as AvroSchema};
use polars_error::{PolarsResult, polars_bail};
use polars_utils::pl_str::PlSmallStr;

use crate::datatypes::*;

fn external_props(schema: &AvroSchema) -> Metadata {
    let mut props = Metadata::new();
    match schema {
        AvroSchema::Record(Record { doc: Some(doc), .. })
        | AvroSchema::Enum(Enum { doc: Some(doc), .. }) => {
            props.insert(
                PlSmallStr::from_static("avro::doc"),
                PlSmallStr::from_str(doc.as_str()),
            );
        },
        _ => {},
    }
    props
}

/// Infers an [`ArrowSchema`] from the root [`Record`].
/// This
pub fn infer_schema(record: &Record) -> PolarsResult<ArrowSchema> {
    record
        .fields
        .iter()
        .map(|field| {
            let field = schema_to_field(
                &field.schema,
                Some(&field.name),
                external_props(&field.schema),
            )?;

            Ok((field.name.clone(), field))
        })
        .collect::<PolarsResult<ArrowSchema>>()
}

fn schema_to_field(
    schema: &AvroSchema,
    name: Option<&str>,
    props: Metadata,
) -> PolarsResult<Field> {
    let mut nullable = false;
    let dtype = match schema {
        AvroSchema::Null => ArrowDataType::Null,
        AvroSchema::Boolean => ArrowDataType::Boolean,
        AvroSchema::Int(logical) => match logical {
            Some(logical) => match logical {
                avro_schema::schema::IntLogical::Date => ArrowDataType::Date32,
                avro_schema::schema::IntLogical::Time => {
                    ArrowDataType::Time32(TimeUnit::Millisecond)
                },
            },
            None => ArrowDataType::Int32,
        },
        AvroSchema::Long(logical) => match logical {
            Some(logical) => match logical {
                avro_schema::schema::LongLogical::Time => {
                    ArrowDataType::Time64(TimeUnit::Microsecond)
                },
                avro_schema::schema::LongLogical::TimestampMillis => ArrowDataType::Timestamp(
                    TimeUnit::Millisecond,
                    Some(PlSmallStr::from_static("00:00")),
                ),
                avro_schema::schema::LongLogical::TimestampMicros => ArrowDataType::Timestamp(
                    TimeUnit::Microsecond,
                    Some(PlSmallStr::from_static("00:00")),
                ),
                avro_schema::schema::LongLogical::LocalTimestampMillis => {
                    ArrowDataType::Timestamp(TimeUnit::Millisecond, None)
                },
                avro_schema::schema::LongLogical::LocalTimestampMicros => {
                    ArrowDataType::Timestamp(TimeUnit::Microsecond, None)
                },
            },
            None => ArrowDataType::Int64,
        },
        AvroSchema::Float => ArrowDataType::Float32,
        AvroSchema::Double => ArrowDataType::Float64,
        AvroSchema::Bytes(logical) => match logical {
            Some(logical) => match logical {
                avro_schema::schema::BytesLogical::Decimal(precision, scale) => {
                    ArrowDataType::Decimal(*precision, *scale)
                },
            },
            None => ArrowDataType::Binary,
        },
        AvroSchema::String(_) => ArrowDataType::Utf8,
        AvroSchema::Array(item_schema) => ArrowDataType::List(Box::new(schema_to_field(
            item_schema,
            Some("item"), // default name for list items
            Metadata::default(),
        )?)),
        AvroSchema::Map(_) => todo!("Avro maps are mapped to MapArrays"),
        AvroSchema::Union(schemas) => {
            // If there are only two variants and one of them is null, set the other type as the field data type
            let has_nullable = schemas.iter().any(|x| x == &AvroSchema::Null);
            if has_nullable && schemas.len() == 2 {
                nullable = true;
                if let Some(schema) = schemas
                    .iter()
                    .find(|&schema| !matches!(schema, AvroSchema::Null))
                {
                    schema_to_field(schema, None, Metadata::default())?.dtype
                } else {
                    polars_bail!(nyi = "Can't read avro union {schema:?}");
                }
            } else {
                let fields = schemas
                    .iter()
                    .map(|s| schema_to_field(s, None, Metadata::default()))
                    .collect::<PolarsResult<Vec<Field>>>()?;
                ArrowDataType::Union(Box::new(UnionType {
                    fields,
                    ids: None,
                    mode: UnionMode::Dense,
                }))
            }
        },
        AvroSchema::Record(Record { fields, .. }) => {
            let fields = fields
                .iter()
                .map(|field| {
                    let mut props = Metadata::new();
                    if let Some(doc) = &field.doc {
                        props.insert(
                            PlSmallStr::from_static("avro::doc"),
                            PlSmallStr::from_str(doc),
                        );
                    }
                    schema_to_field(&field.schema, Some(&field.name), props)
                })
                .collect::<PolarsResult<_>>()?;
            ArrowDataType::Struct(fields)
        },
        AvroSchema::Enum { .. } => {
            return Ok(Field::new(
                PlSmallStr::from_str(name.unwrap_or_default()),
                ArrowDataType::Dictionary(IntegerType::Int32, Box::new(ArrowDataType::Utf8), false),
                false,
            ));
        },
        AvroSchema::Fixed(Fixed { size, logical, .. }) => match logical {
            Some(logical) => match logical {
                avro_schema::schema::FixedLogical::Decimal(precision, scale) => {
                    ArrowDataType::Decimal(*precision, *scale)
                },
                avro_schema::schema::FixedLogical::Duration => {
                    ArrowDataType::Interval(IntervalUnit::MonthDayNano)
                },
            },
            None => ArrowDataType::FixedSizeBinary(*size),
        },
    };

    let name = name.unwrap_or_default();

    Ok(Field::new(PlSmallStr::from_str(name), dtype, nullable).with_metadata(props))
}
