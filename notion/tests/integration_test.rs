use notion_mcp_server::openapi_mcp_server::openapi::parser::OpenAPIToMCPConverter;
use notion_mcp_server::openapi_mcp_server::client::{HttpClient, HttpClientConfig};
use openapiv3::OpenAPI;
use std::collections::HashMap;

#[tokio::test]
async fn test_openapi_parser() {
    let openapi_spec = r#"{
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "servers": [
            {
                "url": "https://api.example.com"
            }
        ],
        "paths": {
            "/test": {
                "get": {
                    "operationId": "testOperation",
                    "summary": "Test operation",
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            }
        }
    }"#;

    let spec: OpenAPI = serde_json::from_str(openapi_spec).expect("Failed to parse OpenAPI spec");
    let mut converter = OpenAPIToMCPConverter::new(spec);
    let result = converter.convert_to_mcp_tools().expect("Failed to convert to MCP tools");

    assert!(!result.tools.is_empty());
    assert!(!result.openapi_lookup.is_empty());
}

#[tokio::test]
async fn test_http_client_creation() {
    let openapi_spec = r#"{
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "servers": [
            {
                "url": "https://api.example.com"
            }
        ],
        "paths": {}
    }"#;

    let spec: OpenAPI = serde_json::from_str(openapi_spec).expect("Failed to parse OpenAPI spec");
    
    let config = HttpClientConfig {
        base_url: "https://api.example.com".to_string(),
        headers: HashMap::new(),
    };

    let client = HttpClient::new(config, spec);
    assert!(client.is_ok());
}

#[test]
fn test_file_upload_detection() {
    use notion_mcp_server::openapi_mcp_server::openapi::file_upload::is_file_upload_parameter;
    use openapiv3::{Operation, RequestBody, MediaType, ReferenceOr, Schema, SchemaKind, Type, StringType};
    use std::collections::BTreeMap;

    let mut operation = Operation::default();
    
    // Create a multipart/form-data request body with a file field
    let mut content = BTreeMap::new();
    let mut schema_properties = BTreeMap::new();
    
    schema_properties.insert(
        "file".to_string(),
        ReferenceOr::Item(Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Type(Type::String(StringType {
                format: Some("binary".to_string()),
                ..Default::default()
            })),
        }),
    );

    let schema = Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(Type::Object(openapiv3::ObjectType {
            properties: schema_properties,
            ..Default::default()
        })),
    };

    content.insert(
        "multipart/form-data".to_string(),
        MediaType {
            schema: Some(ReferenceOr::Item(schema)),
            ..Default::default()
        },
    );

    operation.request_body = Some(ReferenceOr::Item(RequestBody {
        content,
        ..Default::default()
    }));

    let file_params = is_file_upload_parameter(&operation);
    assert_eq!(file_params, vec!["file"]);
}