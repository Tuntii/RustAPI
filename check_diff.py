import json

left_str = """{"components": {"schemas": {"ErrorBodySchema": {"properties": {"error_type": {"type": "string"}, "fields": {"items": {"": "#/components/schemas/FieldErrorSchema"}, "type": ["array", "null"]}, "message": {"type": "string"}}, "required": ["error_type", "message"], "type": "object"}, "ErrorSchema": {"properties": {"error": {"": "#/components/schemas/ErrorBodySchema"}, "request_id": {"type": ["string", "null"]}}, "required": ["error"], "type": "object"}, "FieldErrorSchema": {"properties": {"code": {"type": "string"}, "field": {"type": "string"}, "message": {"type": "string"}}, "required": ["field", "code", "message"], "type": "object"}, "SnapshotUser": {"properties": {"id": {"format": "int64", "type": "integer"}, "username": {"type": "string"}}, "required": ["id", "username"], "type": "object"}, "ValidationErrorBodySchema": {"properties": {"error_type": {"type": "string"}, "fields": {"items": {"": "#/components/schemas/FieldErrorSchema"}, "type": "array"}, "message": {"type": "string"}}, "required": ["error_type", "message", "fields"], "type": "object"}, "ValidationErrorSchema": {"properties": {"error": {"": "#/components/schemas/ValidationErrorBodySchema"}}, "required": ["error"], "type": "object"}}, "info": {"description": "Test Description", "title": "Snapshot API", "version": "1.0.0"}, "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema", "openapi": "3.1.0", "paths": {"/users": {"get": {"responses": {"200": {"content": {"text/plain": {"schema": {"type": "string"}}}, "description": "Successful response"}}}}, "/users/{id}": {"get": {"parameters": [{"in": "path", "name": "id", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"content": {"text/plain": {"schema": {"type": "string"}}}, "description": "Successful response"}}}}}}"""

right_str = """{"components": {"schemas": {"ErrorBodySchema": {"properties": {"error_type": {"type": "string"}, "fields": {"items": {"": "#/components/schemas/FieldErrorSchema"}, "type": ["array", "null"]}, "message": {"type": "string"}}, "required": ["error_type", "message"], "type": "object"}, "ErrorSchema": {"properties": {"error": {"": "#/components/schemas/ErrorBodySchema"}, "request_id": {"type": ["string", "null"]}}, "required": ["error"], "type": "object"}, "FieldErrorSchema": {"properties": {"code": {"type": "string"}, "field": {"type": "string"}, "message": {"type": "string"}}, "required": ["field", "code", "message"], "type": "object"}, "SnapshotUser": {"properties": {"id": {"format": "int64", "type": "integer"}, "username": {"type": "string"}}, "required": ["id", "username"], "type": "object"}, "ValidationErrorBodySchema": {"properties": {"error_type": {"type": "string"}, "fields": {"items": {"": "#/components/schemas/FieldErrorSchema"}, "type": "string"}, "message": {"type": "string"}}, "required": ["error_type", "message", "fields"], "type": "object"}, "ValidationErrorSchema": {"properties": {"error": {"": "#/components/schemas/ValidationErrorBodySchema"}}, "required": ["error"], "type": "object"}}, "info": {"description": "Test Description", "title": "Snapshot API", "version": "1.0.0"}, "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema", "openapi": "3.1.0", "paths": {"/users": {"get": {"responses": {"200": {"content": {"text/plain": {"schema": {"type": "string"}}}, "description": "Successful response"}}}}, "/users/{id}": {"get": {"parameters": [{"in": "path", "name": "id", "required": true, "schema": {"type": "string"}}], "responses": {"200": {"content": {"text/plain": {"schema": {"type": "string"}}}, "description": "Successful response"}}}}}}"""

left = json.loads(left_str)
right = json.loads(right_str)

def compare(path, l, r):
    if l != r:
        print(f"Difference at {path}: {l} != {r}")
    if isinstance(l, dict) and isinstance(r, dict):
        for k in l:
            if k in r:
                compare(f"{path}.{k}", l[k], r[k])
            else:
                print(f"Missing key in right: {path}.{k}")
        for k in r:
            if k not in l:
                print(f"Missing key in left: {path}.{k}")
    elif isinstance(l, list) and isinstance(r, list):
        if len(l) != len(r):
            print(f"Length mismatch at {path}")
        for i in range(min(len(l), len(r))):
            compare(f"{path}[{i}]", l[i], r[i])

compare("root", left, right)
