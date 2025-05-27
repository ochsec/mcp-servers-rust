use openapiv3::{Operation, ReferenceOr, Schema, SchemaKind, StringFormat, Type, VariantOrUnknownOrEmpty};

pub fn is_file_upload_parameter(operation: &Operation) -> Vec<String> {
    let mut file_params = Vec::new();

    // Check requestBody for multipart/form-data
    if let Some(request_body_ref) = &operation.request_body {
        if let ReferenceOr::Item(request_body) = request_body_ref {
            if let Some(media_type) = request_body.content.get("multipart/form-data") {
                if let Some(schema_ref) = &media_type.schema {
                    if let ReferenceOr::Item(schema) = schema_ref {
                        if let SchemaKind::Type(Type::Object(object_type)) = &schema.schema_kind {
                            for (prop_name, prop_schema_ref) in &object_type.properties {
                                let cloned_schema = match prop_schema_ref {
                                    ReferenceOr::Item(boxed_schema) => ReferenceOr::Item(boxed_schema.as_ref().clone()),
                                    ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference: reference.clone() },
                                };
                                if is_file_schema(&cloned_schema) {
                                    file_params.push(prop_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    file_params
}

fn is_file_schema(schema_ref: &ReferenceOr<Schema>) -> bool {
    match schema_ref {
        ReferenceOr::Item(schema) => {
            match &schema.schema_kind {
                SchemaKind::Type(Type::String(string_type)) => {
                    // Check if it's a binary format
                    match &string_type.format {
                        VariantOrUnknownOrEmpty::Item(format) => {
                            return format == &StringFormat::Binary;
                        }
                        _ => {}
                    }
                }
                SchemaKind::Type(Type::Array(array_type)) => {
                    // Check if it's an array of files
                    if let Some(items) = &array_type.items {
                        let cloned_items = match items {
                            ReferenceOr::Item(boxed_schema) => ReferenceOr::Item(boxed_schema.as_ref().clone()),
                            ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference: reference.clone() },
                        };
                        return is_file_schema(&cloned_items);
                    }
                }
                _ => {}
            }
        }
        ReferenceOr::Reference { .. } => {
            // For referenced schemas, we'd need to resolve them first
            // For now, we'll assume they're not file schemas
            return false;
        }
    }
    false
}