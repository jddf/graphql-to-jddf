use failure::{format_err, Error};
use graphql_client::{GraphQLQuery, Response};
use serde_json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let graphql_schema: Response<introspection_query::ResponseData> =
        serde_json::from_reader(std::io::stdin())?;

    let graphql_schema = graphql_schema
        .data
        .ok_or(format_err!("no data in graphql response"))?
        .schema
        .ok_or(format_err!("no schema in graphql response"))?;

    let root_name = graphql_schema.query_type.name.clone();

    let mut defs = HashMap::new();
    for gql_schema in GraphQLType::from_schema(graphql_schema) {
        let name = match &gql_schema {
            GraphQLType::Object { ref name, .. } => name.clone(),
            GraphQLType::Interface { ref name, .. } => name.clone(),
            GraphQLType::Union { ref name, .. } => name.clone(),
            GraphQLType::Enum { ref name, .. } => name.clone(),
            GraphQLType::Input { ref name, .. } => name.clone(),
            GraphQLType::Scalar(ref name) => name.clone(),
            _ => {
                dbg!(gql_schema);
                unreachable!()
            }
        };

        defs.insert(name, gql_schema.into_jddf());
    }

    let schema = jddf::Schema::from_parts(
        Some(defs),
        Box::new(jddf::Form::Ref(root_name.unwrap())),
        HashMap::new(),
    );

    println!("{}", serde_json::to_string(&schema.into_serde())?);
    Ok(())
}

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "src/graphql/introspection_query.graphql",
    schema_path = "src/graphql/introspection_schema.graphql",
    response_derives = "Debug"
)]
struct IntrospectionQuery;

#[derive(Debug)]
enum GraphQLType {
    Ref(String),
    Scalar(String),
    Object {
        name: String,
        fields: HashMap<String, GraphQLType>,
    },
    Interface {
        name: String,
        impls: Vec<GraphQLType>,
    },
    Union {
        name: String,
        types: Vec<GraphQLType>,
    },
    Enum {
        name: String,
        values: Vec<String>,
    },
    Input {
        name: String,
        fields: HashMap<String, GraphQLType>,
    },
    NonNull(Box<GraphQLType>),
    List(Box<GraphQLType>),
}

impl GraphQLType {
    fn into_jddf(self) -> jddf::Schema {
        match self {
            Self::Ref(name) => {
                jddf::Schema::from_parts(None, Box::new(jddf::Form::Ref(name)), HashMap::new())
            }

            Self::Scalar(scalar) => match scalar.as_str() {
                "Int" => jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Type(jddf::Type::Int32)),
                    HashMap::new(),
                ),
                "Float" => jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Type(jddf::Type::Float64)),
                    HashMap::new(),
                ),
                "Boolean" => jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Type(jddf::Type::Boolean)),
                    HashMap::new(),
                ),
                "String" | "ID" => jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Type(jddf::Type::String)),
                    HashMap::new(),
                ),
                _ => jddf::Schema::from_parts(None, Box::new(jddf::Form::Empty), HashMap::new()),
            },

            Self::Object { fields, .. } => {
                let mut required = HashMap::new();
                let mut optional = HashMap::new();
                for (name, field) in fields {
                    match field {
                        Self::NonNull(gql_type) => {
                            required.insert(name, gql_type.into_jddf());
                        }
                        _ => {
                            optional.insert(name, field.into_jddf());
                        }
                    }
                }

                jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Properties {
                        required,
                        optional,
                        allow_additional: false,
                        has_required: true,
                    }),
                    HashMap::new(),
                )
            }

            Self::List(gql_type) => match *gql_type {
                Self::NonNull(gql_type) => jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Elements(gql_type.into_jddf())),
                    HashMap::new(),
                ),
                _ => jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Elements(jddf::Schema::from_parts(
                        None,
                        Box::new(jddf::Form::Empty),
                        HashMap::new(),
                    ))),
                    HashMap::new(),
                ),
            },

            // TODO: Maybe have a struct with the known-existing fields, instead?
            Self::Interface { .. } => {
                jddf::Schema::from_parts(None, Box::new(jddf::Form::Empty), HashMap::new())
            }

            // TODO: Maybe have a struct with the known-existing fields, instead?
            Self::Union { .. } => {
                jddf::Schema::from_parts(None, Box::new(jddf::Form::Empty), HashMap::new())
            }

            Self::Enum { values, .. } => jddf::Schema::from_parts(
                None,
                Box::new(jddf::Form::Enum(values.into_iter().collect())),
                HashMap::new(),
            ),

            Self::Input { fields, .. } => {
                let mut required = HashMap::new();
                let mut optional = HashMap::new();
                for (name, field) in fields {
                    match field {
                        Self::NonNull(gql_type) => {
                            required.insert(name, gql_type.into_jddf());
                        }
                        _ => {
                            optional.insert(name, field.into_jddf());
                        }
                    }
                }

                jddf::Schema::from_parts(
                    None,
                    Box::new(jddf::Form::Properties {
                        required,
                        optional,
                        allow_additional: false,
                        has_required: true,
                    }),
                    HashMap::new(),
                )
            }

            _ => unreachable!(),
        }
    }

    fn from_schema(schema: introspection_query::IntrospectionQuerySchema) -> Vec<GraphQLType> {
        schema
            .types
            .into_iter()
            .map(|t| Self::from_full_type(t.full_type))
            .collect()
    }

    fn from_full_type(full_type: introspection_query::FullType) -> GraphQLType {
        use introspection_query::{FullType, __TypeKind as Kind};

        match full_type {
            FullType {
                kind: Kind::SCALAR,
                name: Some(name),
                ..
            } => GraphQLType::Scalar(name),

            FullType {
                kind: Kind::OBJECT,
                name: Some(name),
                fields: Some(fields),
                ..
            } => GraphQLType::Object {
                name,
                fields: fields
                    .into_iter()
                    .map(|field| (field.name, Self::from_type_ref(field.type_.type_ref)))
                    .collect(),
            },

            FullType {
                kind: Kind::INTERFACE,
                name: Some(name),
                possible_types: Some(possible_types),
                ..
            } => GraphQLType::Interface {
                name,
                impls: possible_types
                    .into_iter()
                    .map(|t| Self::from_type_ref(t.type_ref))
                    .collect(),
            },

            FullType {
                kind: Kind::UNION,
                name: Some(name),
                possible_types: Some(possible_types),
                ..
            } => GraphQLType::Union {
                name,
                types: possible_types
                    .into_iter()
                    .map(|t| Self::from_type_ref(t.type_ref))
                    .collect(),
            },

            FullType {
                kind: Kind::ENUM,
                name: Some(name),
                enum_values: Some(enum_values),
                ..
            } => GraphQLType::Enum {
                name,
                values: enum_values.into_iter().map(|v| v.name).collect(),
            },

            FullType {
                kind: Kind::INPUT_OBJECT,
                name: Some(name),
                input_fields: Some(input_fields),
                ..
            } => GraphQLType::Input {
                name,
                fields: input_fields
                    .into_iter()
                    .map(|field| {
                        (
                            field.input_value.name,
                            Self::from_type_ref(field.input_value.type_.type_ref),
                        )
                    })
                    .collect(),
            },

            _ => unreachable!(),
        }
    }

    fn from_type_ref(type_ref: introspection_query::TypeRef) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRef {
                name: Some(name), ..
            } => GraphQLType::Ref(name),
            introspection_query::TypeRef {
                kind: introspection_query::__TypeKind::NON_NULL,
                of_type: Some(of_type),
                ..
            } => GraphQLType::NonNull(Box::new(Self::from_type_ref2(of_type))),
            introspection_query::TypeRef {
                kind: introspection_query::__TypeKind::LIST,
                of_type: Some(of_type),
                ..
            } => GraphQLType::List(Box::new(Self::from_type_ref2(of_type))),
            _ => unreachable!(),
        }
    }

    fn from_type_ref2(type_ref: introspection_query::TypeRefOfType) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRefOfType {
                name: Some(name), ..
            } => GraphQLType::Ref(name),
            introspection_query::TypeRefOfType {
                kind: introspection_query::__TypeKind::NON_NULL,
                of_type: Some(of_type),
                ..
            } => GraphQLType::NonNull(Box::new(Self::from_type_ref3(of_type))),
            introspection_query::TypeRefOfType {
                kind: introspection_query::__TypeKind::LIST,
                of_type: Some(of_type),
                ..
            } => GraphQLType::List(Box::new(Self::from_type_ref3(of_type))),
            _ => unreachable!(),
        }
    }

    fn from_type_ref3(type_ref: introspection_query::TypeRefOfTypeOfType) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRefOfTypeOfType {
                name: Some(name), ..
            } => GraphQLType::Ref(name),
            introspection_query::TypeRefOfTypeOfType {
                kind: introspection_query::__TypeKind::NON_NULL,
                of_type: Some(of_type),
                ..
            } => GraphQLType::NonNull(Box::new(Self::from_type_ref4(of_type))),
            introspection_query::TypeRefOfTypeOfType {
                kind: introspection_query::__TypeKind::LIST,
                of_type: Some(of_type),
                ..
            } => GraphQLType::List(Box::new(Self::from_type_ref4(of_type))),
            _ => unreachable!(),
        }
    }

    fn from_type_ref4(type_ref: introspection_query::TypeRefOfTypeOfTypeOfType) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRefOfTypeOfTypeOfType {
                name: Some(name), ..
            } => GraphQLType::Ref(name),
            introspection_query::TypeRefOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::NON_NULL,
                of_type: Some(of_type),
                ..
            } => GraphQLType::NonNull(Box::new(Self::from_type_ref5(of_type))),
            introspection_query::TypeRefOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::LIST,
                of_type: Some(of_type),
                ..
            } => GraphQLType::List(Box::new(Self::from_type_ref5(of_type))),
            _ => unreachable!(),
        }
    }

    fn from_type_ref5(
        type_ref: introspection_query::TypeRefOfTypeOfTypeOfTypeOfType,
    ) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfType {
                name: Some(name), ..
            } => GraphQLType::Ref(name),
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::NON_NULL,
                of_type: Some(of_type),
                ..
            } => GraphQLType::NonNull(Box::new(Self::from_type_ref6(of_type))),
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::LIST,
                of_type: Some(of_type),
                ..
            } => GraphQLType::List(Box::new(Self::from_type_ref6(of_type))),
            _ => unreachable!(),
        }
    }

    fn from_type_ref6(
        type_ref: introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfType,
    ) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfType {
                name: Some(name), ..
            } => GraphQLType::Ref(name),
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::NON_NULL,
                of_type: Some(of_type),
                ..
            } => GraphQLType::NonNull(Box::new(Self::from_type_ref7(of_type))),
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::LIST,
                of_type: Some(of_type),
                ..
            } => GraphQLType::List(Box::new(Self::from_type_ref7(of_type))),
            _ => unreachable!(),
        }
    }

    fn from_type_ref7(
        type_ref: introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfTypeOfType,
    ) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfTypeOfType {
                name: Some(name),
                ..
            } => GraphQLType::Ref(name),
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::NON_NULL,
                of_type: Some(of_type),
                ..
            } => GraphQLType::NonNull(Box::new(Self::from_type_ref8(of_type))),
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfTypeOfType {
                kind: introspection_query::__TypeKind::LIST,
                of_type: Some(of_type),
                ..
            } => GraphQLType::List(Box::new(Self::from_type_ref8(of_type))),
            _ => unreachable!(),
        }
    }

    fn from_type_ref8(
        type_ref: introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfTypeOfTypeOfType,
    ) -> GraphQLType {
        match type_ref {
            introspection_query::TypeRefOfTypeOfTypeOfTypeOfTypeOfTypeOfTypeOfType {
                name: Some(name),
                ..
            } => GraphQLType::Ref(name),
            _ => unreachable!(),
        }
    }
}
