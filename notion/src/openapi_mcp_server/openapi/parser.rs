use anyhow::Result;
use openapiv3::{OpenAPI, Operation, Parameter, ParameterData, ReferenceOr, RequestBody, Response, Schema, SchemaKind, Type};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub struct MCPMethod {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub return_schema: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct MCPTool {
    pub methods: Vec<MCPMethod>,
}

#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub operation: Operation,
    pub method: String,
    pub path: String,
}

pub struct ConversionResult {
    pub tools: HashMap<String, MCPTool>,
    pub openapi_lookup: HashMap<String, OperationInfo>,
}

pub struct OpenAPIToMCPConverter {
    openapi_spec: OpenAPI,
    schema_cache: HashMap<String, Value>,
    name_counter: u32,
}

impl OpenAPIToMCPConverter {
    pub fn new(openapi_spec: OpenAPI) -> Self {
        Self {
            openapi_spec,
            schema_cache: HashMap::new(),
            name_counter: 0,
        }
    }

    pub fn convert_to_mcp_tools(&mut self) -> Result<ConversionResult> {
        let api_name = "API";
        let mut tools = HashMap::new();
        let mut openapi_lookup = HashMap::new();
        let mut methods = Vec::new();

        // Process each path and operation
        for (path, path_item) in &self.openapi_spec.paths.paths {
            if let ReferenceOr::Item(path_item) = path_item {
                // Process each HTTP method
                if let Some(operation) = &path_item.get {
                    if let Some(method) = self.convert_operation_to_mcp_method(operation, "get", path)? {
                        let unique_name = self.ensure_unique_name(&method.name);
                        let mut method = method;
                        method.name = unique_name.clone();
                        methods.push(method);
                        openapi_lookup.insert(
                            format!("{}-{}", api_name, unique_name),
                            OperationInfo {
                                operation: operation.clone(),
                                method: "get".to_string(),
                                path: path.clone(),
                            },
                        );
                    }
                }
                
                if let Some(operation) = &path_item.post {
                    if let Some(method) = self.convert_operation_to_mcp_method(operation, "post", path)? {
                        let unique_name = self.ensure_unique_name(&method.name);
                        let mut method = method;
                        method.name = unique_name.clone();
                        methods.push(method);
                        openapi_lookup.insert(
                            format!("{}-{}", api_name, unique_name),
                            OperationInfo {
                                operation: operation.clone(),
                                method: "post".to_string(),
                                path: path.clone(),
                            },
                        );
                    }
                }
                
                if let Some(operation) = &path_item.put {
                    if let Some(method) = self.convert_operation_to_mcp_method(operation, "put", path)? {
                        let unique_name = self.ensure_unique_name(&method.name);
                        let mut method = method;
                        method.name = unique_name.clone();
                        methods.push(method);
                        openapi_lookup.insert(
                            format!("{}-{}", api_name, unique_name),
                            OperationInfo {
                                operation: operation.clone(),
                                method: "put".to_string(),
                                path: path.clone(),
                            },
                        );
                    }
                }
                
                if let Some(operation) = &path_item.delete {
                    if let Some(method) = self.convert_operation_to_mcp_method(operation, "delete", path)? {
                        let unique_name = self.ensure_unique_name(&method.name);
                        let mut method = method;
                        method.name = unique_name.clone();
                        methods.push(method);
                        openapi_lookup.insert(
                            format!("{}-{}", api_name, unique_name),
                            OperationInfo {
                                operation: operation.clone(),
                                method: "delete".to_string(),
                                path: path.clone(),
                            },
                        );
                    }
                }
                
                if let Some(operation) = &path_item.patch {
                    if let Some(method) = self.convert_operation_to_mcp_method(operation, "patch", path)? {
                        let unique_name = self.ensure_unique_name(&method.name);
                        let mut method = method;
                        method.name = unique_name.clone();
                        methods.push(method);
                        openapi_lookup.insert(
                            format!("{}-{}", api_name, unique_name),
                            OperationInfo {
                                operation: operation.clone(),
                                method: "patch".to_string(),
                                path: path.clone(),
                            },
                        );
                    }
                }
            }
        }

        tools.insert(api_name.to_string(), MCPTool { methods });

        Ok(ConversionResult {
            tools,
            openapi_lookup,
        })
    }

    fn convert_operation_to_mcp_method(
        &mut self,
        operation: &Operation,
        method: &str,
        path: &str,
    ) -> Result<Option<MCPMethod>> {
        let Some(operation_id) = &operation.operation_id else {
            warn!("Operation without operationId at {} {}", method, path);
            return Ok(None);
        };

        let method_name = operation_id.clone();

        // Build input schema
        let mut input_schema = Value::Object(Map::new());
        let input_obj = input_schema.as_object_mut().unwrap();
        input_obj.insert("type".to_string(), Value::String("object".to_string()));
        input_obj.insert("properties".to_string(), Value::Object(Map::new()));
        input_obj.insert("required".to_string(), Value::Array(Vec::new()));
        
        // Add $defs for component schemas
        let defs = self.convert_components_to_json_schema();
        if !defs.is_empty() {
            input_obj.insert("$defs".to_string(), Value::Object(defs));
        }

        let properties = input_obj.get_mut("properties").unwrap().as_object_mut().unwrap();
        let required = input_obj.get_mut("required").unwrap().as_array_mut().unwrap();

        // Handle parameters (path, query, header, cookie)
        for param_ref in &operation.parameters {
            if let Some(param) = self.resolve_parameter(param_ref) {
                if let Some(schema) = &param.schema {
                    let mut param_schema = self.convert_openapi_schema_to_json_schema(schema, &mut HashSet::new(), false)?;
                    
                    // Merge parameter-level description if available
                    if let Some(description) = &param.description {
                        if let Value::Object(ref mut schema_obj) = param_schema {
                            schema_obj.insert("description".to_string(), Value::String(description.clone()));
                        }
                    }
                    
                    properties.insert(param.name.clone(), param_schema);
                    if param.required {
                        required.push(Value::String(param.name.clone()));
                    }
                }
            }
        }

        // Handle requestBody
        if let Some(request_body_ref) = &operation.request_body {
            if let Some(request_body) = self.resolve_request_body(request_body_ref) {
                // Handle multipart/form-data for file uploads
                if let Some(media_type) = request_body.content.get("multipart/form-data") {
                    if let Some(schema) = &media_type.schema {
                        let form_schema = self.convert_openapi_schema_to_json_schema(schema, &mut HashSet::new(), false)?;
                        if let Value::Object(form_obj) = form_schema {
                            if let Some(Value::Object(form_properties)) = form_obj.get("properties") {
                                for (name, prop_schema) in form_properties {
                                    properties.insert(name.clone(), prop_schema.clone());
                                }
                            }
                            if let Some(Value::Array(form_required)) = form_obj.get("required") {
                                for req in form_required {
                                    if let Value::String(req_str) = req {
                                        required.push(Value::String(req_str.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
                // Handle application/json
                else if let Some(media_type) = request_body.content.get("application/json") {
                    if let Some(schema) = &media_type.schema {
                        let body_schema = self.convert_openapi_schema_to_json_schema(schema, &mut HashSet::new(), false)?;
                        
                        // Merge body schema into the input schema's properties
                        if let Value::Object(body_obj) = body_schema {
                            if let Some(Value::Object(body_properties)) = body_obj.get("properties") {
                                for (name, prop_schema) in body_properties {
                                    properties.insert(name.clone(), prop_schema.clone());
                                }
                            }
                            if let Some(Value::Array(body_required)) = body_obj.get("required") {
                                for req in body_required {
                                    if let Value::String(req_str) = req {
                                        required.push(Value::String(req_str.clone()));
                                    }
                                }
                            }
                        } else {
                            // If the request body is not an object, put it under "body"
                            properties.insert("body".to_string(), body_schema);
                            required.push(Value::String("body".to_string()));
                        }
                    }
                }
            }
        }

        // Build description including error responses
        let mut description = operation.summary.clone()
            .or_else(|| operation.description.clone())
            .unwrap_or_default();

        if let Some(responses) = &operation.responses {
            let error_responses: Vec<String> = responses.responses.iter()
                .filter(|(code, _)| code.starts_with('4') || code.starts_with('5'))
                .map(|(code, response_ref)| {
                    let error_desc = match self.resolve_response(response_ref) {
                        Some(response) => response.description.clone(),
                        None => String::new(),
                    };
                    format!("{}: {}", code, error_desc)
                })
                .collect();

            if !error_responses.is_empty() {
                description.push_str("\nError Responses:\n");
                description.push_str(&error_responses.join("\n"));
            }
        }

        // Extract return type (response schema)
        let return_schema = self.extract_response_type(&operation.responses);

        Ok(Some(MCPMethod {
            name: method_name,
            description,
            input_schema,
            return_schema,
        }))
    }

    fn convert_openapi_schema_to_json_schema(
        &mut self,
        schema_ref: &ReferenceOr<Schema>,
        resolved_refs: &mut HashSet<String>,
        resolve_refs: bool,
    ) -> Result<Value> {
        match schema_ref {
            ReferenceOr::Reference { reference } => {
                if !resolve_refs {
                    if reference.starts_with("#/components/schemas/") {
                        let mut ref_schema = Map::new();
                        ref_schema.insert(
                            "$ref".to_string(),
                            Value::String(reference.replace("#/components/schemas/", "#/$defs/")),
                        );
                        return Ok(Value::Object(ref_schema));
                    }
                    error!("Attempting to resolve ref {} not found in components collection.", reference);
                }

                // Check if already cached
                if let Some(cached) = self.schema_cache.get(reference) {
                    return Ok(cached.clone());
                }

                if resolved_refs.contains(reference) {
                    // Return a reference to avoid infinite recursion
                    let mut ref_schema = Map::new();
                    ref_schema.insert(
                        "$ref".to_string(),
                        Value::String(reference.replace("#/components/schemas/", "#/$defs/")),
                    );
                    return Ok(Value::Object(ref_schema));
                }

                resolved_refs.insert(reference.clone());
                
                // Try to resolve the reference
                if let Some(resolved_schema) = self.internal_resolve_ref(reference) {
                    let converted = self.convert_openapi_schema_to_json_schema(&ReferenceOr::Item(resolved_schema), resolved_refs, resolve_refs)?;
                    self.schema_cache.insert(reference.clone(), converted.clone());
                    Ok(converted)
                } else {
                    error!("Failed to resolve ref {}", reference);
                    let mut ref_schema = Map::new();
                    ref_schema.insert(
                        "$ref".to_string(),
                        Value::String(reference.replace("#/components/schemas/", "#/$defs/")),
                    );
                    Ok(Value::Object(ref_schema))
                }
            }
            ReferenceOr::Item(schema) => {
                let mut result = Map::new();

                match &schema.schema_kind {
                    SchemaKind::Type(Type::Object(object_type)) => {
                        result.insert("type".to_string(), Value::String("object".to_string()));
                        
                        if !object_type.properties.is_empty() {
                            let mut properties = Map::new();
                            for (name, prop_schema) in &object_type.properties {
                                properties.insert(
                                    name.clone(),
                                    self.convert_openapi_schema_to_json_schema(prop_schema, resolved_refs, resolve_refs)?,
                                );
                            }
                            result.insert("properties".to_string(), Value::Object(properties));
                        }

                        if !object_type.required.is_empty() {
                            let required: Vec<Value> = object_type.required.iter()
                                .map(|s| Value::String(s.clone()))
                                .collect();
                            result.insert("required".to_string(), Value::Array(required));
                        }

                        match &object_type.additional_properties {
                            Some(additional) => {
                                result.insert("additionalProperties".to_string(), 
                                    self.convert_openapi_schema_to_json_schema(additional, resolved_refs, resolve_refs)?);
                            }
                            None => {
                                result.insert("additionalProperties".to_string(), Value::Bool(true));
                            }
                        }
                    }
                    SchemaKind::Type(Type::Array(array_type)) => {
                        result.insert("type".to_string(), Value::String("array".to_string()));
                        if let Some(items) = &array_type.items {
                            result.insert("items".to_string(), 
                                self.convert_openapi_schema_to_json_schema(items, resolved_refs, resolve_refs)?);
                        }
                    }
                    SchemaKind::Type(Type::String(string_type)) => {
                        result.insert("type".to_string(), Value::String("string".to_string()));
                        if let Some(format) = &string_type.format {
                            // Convert binary format to uri-reference and enhance description
                            if format == "binary" {
                                result.insert("format".to_string(), Value::String("uri-reference".to_string()));
                                let binary_desc = "absolute paths to local files";
                                let description = schema.schema_data.description.as_ref()
                                    .map(|d| format!("{} ({})", d, binary_desc))
                                    .unwrap_or_else(|| binary_desc.to_string());
                                result.insert("description".to_string(), Value::String(description));
                            } else {
                                result.insert("format".to_string(), Value::String(format.clone()));
                            }
                        }
                    }
                    SchemaKind::Type(Type::Number(_)) => {
                        result.insert("type".to_string(), Value::String("number".to_string()));
                    }
                    SchemaKind::Type(Type::Integer(_)) => {
                        result.insert("type".to_string(), Value::String("integer".to_string()));
                    }
                    SchemaKind::Type(Type::Boolean {}) => {
                        result.insert("type".to_string(), Value::String("boolean".to_string()));
                    }
                    SchemaKind::OneOf { one_of } => {
                        let mut schemas = Vec::new();
                        for schema in one_of {
                            schemas.push(self.convert_openapi_schema_to_json_schema(schema, resolved_refs, resolve_refs)?);
                        }
                        result.insert("oneOf".to_string(), Value::Array(schemas));
                    }
                    SchemaKind::AnyOf { any_of } => {
                        let mut schemas = Vec::new();
                        for schema in any_of {
                            schemas.push(self.convert_openapi_schema_to_json_schema(schema, resolved_refs, resolve_refs)?);
                        }
                        result.insert("anyOf".to_string(), Value::Array(schemas));
                    }
                    SchemaKind::AllOf { all_of } => {
                        let mut schemas = Vec::new();
                        for schema in all_of {
                            schemas.push(self.convert_openapi_schema_to_json_schema(schema, resolved_refs, resolve_refs)?);
                        }
                        result.insert("allOf".to_string(), Value::Array(schemas));
                    }
                    _ => {
                        // Handle other schema types as needed
                    }
                }

                if let Some(description) = &schema.schema_data.description {
                    if !result.contains_key("description") {
                        result.insert("description".to_string(), Value::String(description.clone()));
                    }
                }

                if let Some(default) = &schema.schema_data.default {
                    result.insert("default".to_string(), default.clone());
                }

                if let Some(enum_values) = &schema.schema_data.enum_values {
                    result.insert("enum".to_string(), Value::Array(enum_values.clone()));
                }

                Ok(Value::Object(result))
            }
        }
    }

    fn internal_resolve_ref(&self, reference: &str) -> Option<Schema> {
        if !reference.starts_with("#/") {
            return None;
        }

        let parts: Vec<&str> = reference.trim_start_matches("#/").split('/').collect();
        
        // Navigate through the OpenAPI spec
        match parts.as_slice() {
            ["components", "schemas", schema_name] => {
                if let Some(components) = &self.openapi_spec.components {
                    if let Some(ReferenceOr::Item(schema)) = components.schemas.get(*schema_name) {
                        return Some(schema.clone());
                    }
                }
            }
            _ => {
                // Handle other reference types as needed
            }
        }

        None
    }

    fn convert_components_to_json_schema(&mut self) -> Map<String, Value> {
        let mut defs = Map::new();
        
        if let Some(components) = &self.openapi_spec.components {
            for (key, schema_ref) in &components.schemas {
                if let ReferenceOr::Item(schema) = schema_ref {
                    if let Ok(converted) = self.convert_openapi_schema_to_json_schema(
                        &ReferenceOr::Item(schema.clone()),
                        &mut HashSet::new(),
                        true,
                    ) {
                        defs.insert(key.clone(), converted);
                    }
                }
            }
        }
        
        defs
    }

    fn resolve_parameter(&self, param_ref: &ReferenceOr<Parameter>) -> Option<ParameterData> {
        match param_ref {
            ReferenceOr::Item(param) => {
                // Extract the parameter data from the specific parameter type
                match param {
                    Parameter::Query { parameter_data, .. } => Some(parameter_data.clone()),
                    Parameter::Header { parameter_data, .. } => Some(parameter_data.clone()),
                    Parameter::Path { parameter_data, .. } => Some(parameter_data.clone()),
                    Parameter::Cookie { parameter_data, .. } => Some(parameter_data.clone()),
                }
            }
            ReferenceOr::Reference { reference } => {
                // Try to resolve parameter reference
                if let Some(components) = &self.openapi_spec.components {
                    if let Some(ReferenceOr::Item(param)) = components.parameters.get(reference) {
                        return self.resolve_parameter(&ReferenceOr::Item(param.clone()));
                    }
                }
                None
            }
        }
    }

    fn resolve_request_body(&self, body_ref: &ReferenceOr<RequestBody>) -> Option<RequestBody> {
        match body_ref {
            ReferenceOr::Item(body) => Some(body.clone()),
            ReferenceOr::Reference { reference } => {
                // Try to resolve request body reference
                if let Some(components) = &self.openapi_spec.components {
                    if let Some(ReferenceOr::Item(body)) = components.request_bodies.get(reference) {
                        return Some(body.clone());
                    }
                }
                None
            }
        }
    }

    fn resolve_response(&self, response_ref: &ReferenceOr<Response>) -> Option<Response> {
        match response_ref {
            ReferenceOr::Item(response) => Some(response.clone()),
            ReferenceOr::Reference { reference } => {
                // Try to resolve response reference
                if let Some(components) = &self.openapi_spec.components {
                    if let Some(ReferenceOr::Item(response)) = components.responses.get(reference) {
                        return Some(response.clone());
                    }
                }
                None
            }
        }
    }

    fn extract_response_type(&mut self, responses: &Option<openapiv3::Responses>) -> Option<Value> {
        let responses = responses.as_ref()?;
        
        // Look for success responses (200, 201, 202, 204)
        let success_response = responses.responses.get("200")
            .or_else(|| responses.responses.get("201"))
            .or_else(|| responses.responses.get("202"))
            .or_else(|| responses.responses.get("204"))?;

        let response = self.resolve_response(success_response)?;
        
        // Look for application/json content
        if let Some(media_type) = response.content.get("application/json") {
            if let Some(schema) = &media_type.schema {
                if let Ok(mut return_schema) = self.convert_openapi_schema_to_json_schema(schema, &mut HashSet::new(), false) {
                    // Add $defs
                    if let Value::Object(ref mut schema_obj) = return_schema {
                        let defs = self.convert_components_to_json_schema();
                        if !defs.is_empty() {
                            schema_obj.insert("$defs".to_string(), Value::Object(defs));
                        }
                        
                        // Preserve response description if available
                        if !response.description.is_empty() && !schema_obj.contains_key("description") {
                            schema_obj.insert("description".to_string(), Value::String(response.description.clone()));
                        }
                    }
                    
                    return Some(return_schema);
                }
            }
        }

        // Handle other content types
        if response.content.contains_key("image/png") || response.content.contains_key("image/jpeg") {
            let mut schema = Map::new();
            schema.insert("type".to_string(), Value::String("string".to_string()));
            schema.insert("format".to_string(), Value::String("binary".to_string()));
            schema.insert("description".to_string(), Value::String(response.description.clone()));
            return Some(Value::Object(schema));
        }

        // Fallback
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("string".to_string()));
        schema.insert("description".to_string(), Value::String(response.description.clone()));
        Some(Value::Object(schema))
    }

    fn ensure_unique_name(&mut self, name: &str) -> String {
        if name.len() <= 64 {
            return name.to_string();
        }

        let truncated_name = &name[..59]; // Reserve space for suffix
        let unique_suffix = self.generate_unique_suffix();
        format!("{}-{}", truncated_name, unique_suffix)
    }

    fn generate_unique_suffix(&mut self) -> String {
        self.name_counter += 1;
        format!("{:04}", self.name_counter)
    }
}