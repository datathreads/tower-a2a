# A2A Protocol Specification - Latest (v1.0.0)

## 1. Abstract Data Model

### Task
The core unit of work with lifecycle management.
- `id`: (string) Server-generated unique identifier.
- `contextId`: (string) Logical grouping for related tasks.
- `status`: ([TaskStatus](#taskstatus)) Current state indicator.
- `state`: ([TaskState](#taskstate)) Current state enum value (also present inside `status`).
- `messages`: (Array of [Message](#message), optional) Conversation history (replaces `history` from v0.3).
- `artifacts`: (Array of [Artifact](#artifact)) Generated outputs.
- `createdTime`: (string, optional) ISO 8601 timestamp of creation.
- `updatedTime`: (string, optional) ISO 8601 timestamp of last update.

### TaskStatus
- `state`: ([TaskState](#taskstate)) Current state enum.
- `timestamp`: (string, optional) ISO 8601 formatted timestamp.

### TaskState
Enum values (SCREAMING_SNAKE_CASE):
- `CREATED` — Task has been created, not yet started.
- `WORKING` — Processing in progress.
- `COMPLETED` — Successfully finished.
- `FAILED` — Encountered error.
- `CANCELED` — Client-initiated cancellation.
- `REJECTED` — Agent declined request.
- `INPUT_REQUIRED` — Awaiting client input.
- `AUTH_REQUIRED` — Secondary authentication needed.

### Message
- `role`: (string) `"user"` or `"agent"`.
- `parts`: (Array of [Part](#part)) Content segments.
- `contextId`: (string, optional) Context grouping identifier.
- `taskId`: (string, optional) Associated task identifier.
- `messageId`: (string, optional) Unique message identifier.
- `referenceTaskIds`: (string[], optional) IDs of tasks this message references.

### Part (Flat Struct)
Parts are flat structs (proto3 oneof style) — exactly one content field should be populated:
- `text`: (string, optional) Plain text content.
- `mimeType`: (string, optional) Media type descriptor for this part.
- `data`: (any, optional) Structured JSON-serializable content.
- `fileUri`: (string, optional) URI reference to a file resource.

### Artifact
- `artifactId`: (string) Unique identifier.
- `name`: (string, optional) Display name.
- `description`: (string, optional) Purpose explanation.
- `parts`: (Array of [Part](#part)) Constituent content.
- `metadata`: (object, optional) Additional context.
- `extensions`: (string[], optional) Extension URIs.

---

## 2. Discovery & Identity

### AgentCard
Standardized metadata for agent discovery, served at `/.well-known/agent-card.json`.
- `id`: (string) Agent identifier.
- `name`: (string) Display name.
- `description`: (string) Purpose summary.
- `provider`: ([AgentProvider](#agentprovider)) Organization details.
- `capabilities`: ([AgentCapabilities](#agentcapabilities)) Feature declarations.
- `skills`: (Array of [AgentSkill](#agentskill)) Task-specific competencies.
- `interfaces`: (Array of [AgentInterface](#agentinterface)) Protocol and endpoint information (plural, replaces `interface` from earlier drafts).
- `version`: (string) Agent version string.
- `extensions`: (Array of [AgentExtension](#agentextension), optional) Optional enhancements.
- `securitySchemes`: (object, optional) Authentication method definitions.
- `security`: (Array, optional) Required schemes for operations.
- `signature`: ([AgentCardSignature](#agentcardsignature), optional) Cryptographic verification.

### AgentProvider
- `id`: (string) Provider identifier.
- `name`: (string) Organization or provider name.
- `description`: (string, optional) Provider description.

### AgentInterface
- `type`: (string) Protocol type: `"json-rpc"`, `"grpc"`, `"http"`, or custom.
- `uri`: (string URL) Service endpoint URI.

### AgentCapabilities
- `streaming`: (boolean, optional) Real-time SSE event delivery support.
- `pushNotifications`: (boolean, optional) Webhook delivery capability.
- `extendedAgentCard`: (boolean, optional) Authenticated detailed card availability.

### AgentSkill
- `id`: (string) Skill identifier.
- `name`: (string) Human-readable name.
- `description`: (string) Capability overview.
- `tags`: (string[]) Categorical keywords.
- `examples`: (string[], optional) Usage examples.
- `inputModes`: (string[], optional) Supported input MIME types.
- `outputModes`: (string[], optional) Supported output MIME types.

### AgentExtension
- `uri`: (string) Unique extension identifier.
- `description`: (string, optional) Extension usage explanation.
- `required`: (boolean, optional) Whether client compliance is required.
- `params`: (object, optional) Extension-specific configuration.

### AgentCardSignature
JWS signature per RFC 7515.
- `protected`: (string) Base64url-encoded JSON object.
- `signature`: (string) Computed signature, Base64url-encoded.
- `header`: (object, optional) Unprotected JWS header values.

---

## 3. Operations (Abstract Methods)

These operations map to JSON-RPC methods with `A2A.` prefix.

| Operation | JSON-RPC Method | Input | Output |
| :--- | :--- | :--- | :--- |
| **SendMessage** | `A2A.SendMessage` | `SendMessageRequest` | `Task \| Message` |
| **SendStreamingMessage** | `A2A.SendStreamingMessage` | `SendMessageRequest` | SSE stream of events |
| **GetTask** | `A2A.GetTask` | `taskId`, `historyLength?` | `Task` |
| **ListTasks** | `A2A.ListTasks` | `ListTasksParams` | `ListTasksResponse` |
| **CancelTask** | `A2A.CancelTask` | `taskId` | `Task` |
| **SubscribeToTask** | `A2A.SubscribeToTask` | `taskId` | SSE stream |
| **CreateTaskPushNotificationConfig** | `A2A.CreateTaskPushNotificationConfig` | `taskId`, `PushNotificationConfig` | `PushNotificationConfig` |
| **GetTaskPushNotificationConfig** | `A2A.GetTaskPushNotificationConfig` | `taskId`, `id` | `PushNotificationConfig` |
| **ListTaskPushNotificationConfigs** | `A2A.ListTaskPushNotificationConfigs` | `taskId`, pagination params | `ListPushNotificationConfigsResponse` |
| **DeleteTaskPushNotificationConfig** | `A2A.DeleteTaskPushNotificationConfig` | `taskId`, `id` | (empty) |
| **GetExtendedAgentCard** | `A2A.GetExtendedAgentCard` | (authenticated) | `AgentCard` |

### ListTasksParams
- `contextId`: (string, optional) Filter by context.
- `status`: ([TaskState](#taskstate), optional) Filter by task state.
- `pageSize`: (integer, optional) Max results per page.
- `pageToken`: (string, optional) Cursor for pagination.
- `historyLength`: (integer, optional) Number of messages to include.
- `statusTimestampAfter`: (string, optional) Filter tasks updated after this ISO 8601 timestamp.
- `includeArtifacts`: (boolean, optional) Whether to include artifacts in results.
- `tenant`: (string, optional) Tenant scoping identifier.

### SendMessageRequest
- `message`: ([Message](#message), required) Content to send.
- `configuration`: ([SendMessageConfiguration](#sendmessageconfiguration), optional) Behavior options.

### SendMessageConfiguration
- `acceptedOutputModes`: (string[], optional) Preferred response MIME types.
- `pushNotificationConfig`: ([PushNotificationConfig](#pushnotificationconfig), optional) Webhook setup.
- `historyLength`: (integer, optional) Message retrieval limit.
- `blocking`: (boolean, optional) Wait for completion (default: false).

---

## 4. Binding Requirements

### HTTP Binding
- **Headers**:
  - `A2A-Version`: (Required) e.g., `1.0`.
  - `A2A-Extensions`: (Optional) Comma-separated list of extension URIs.
  - `Authorization`: Authentication credentials.
  - `Content-Type`: Request media type.
- **REST Mappings**:
  - `POST /messages` → `A2A.SendMessage`
  - `POST /messages/stream` → `A2A.SendStreamingMessage`
  - `GET /tasks/{id}` → `A2A.GetTask`
  - `GET /tasks` → `A2A.ListTasks`
  - `POST /tasks/{id}/cancel` → `A2A.CancelTask`
  - `GET /tasks/{id}/subscribe` → `A2A.SubscribeToTask`
  - `POST /tasks/{id}/pushNotificationConfigs` → `A2A.CreateTaskPushNotificationConfig`
  - `GET /tasks/{id}/pushNotificationConfigs/{configId}` → `A2A.GetTaskPushNotificationConfig`
  - `GET /tasks/{id}/pushNotificationConfigs` → `A2A.ListTaskPushNotificationConfigs`
  - `DELETE /tasks/{id}/pushNotificationConfigs/{configId}` → `A2A.DeleteTaskPushNotificationConfig`
  - `GET /.well-known/agent-card.json` → discovery (unauthenticated)
  - `GET /agent-card` → `A2A.GetExtendedAgentCard` (authenticated)
- **Query Parameters**: Lowercase snake_case naming (`context_id`, `page_size`, `page_token`, `history_length`, `include_artifacts`, `status`, `status_timestamp_after`).
- **Response Format**:
  - Success: HTTP 200-299 with JSON body.
  - Errors: HTTP 4xx/5xx with error object containing `code`, `message`, `details`.

### JSON-RPC Binding
- **Method Naming**: Uses `A2A.` prefix with PascalCase (e.g., `A2A.SendMessage`, `A2A.GetTask`).
- **Structure**: Standard JSON-RPC 2.0 envelopes.
  ```json
  {
    "jsonrpc": "2.0",
    "method": "A2A.SendMessage",
    "params": { ... },
    "id": "request-id"
  }
  ```
- **Error Codes**:
  - `-32600`: Invalid request.
  - `-32601`: Method not found.
  - `-32602`: Invalid params.
  - `-32603`: Internal error.
  - Custom codes for A2A-specific errors (e.g., TaskNotFound).

### gRPC Binding
- **Service Definition**:
  ```protobuf
  service A2A {
    rpc SendMessage(SendMessageRequest) returns (Task);
    rpc SendStreamingMessage(SendMessageRequest) returns (stream StreamResponse);
    rpc GetTask(GetTaskRequest) returns (Task);
    rpc ListTasks(ListTasksRequest) returns (ListTasksResponse);
    rpc CancelTask(CancelTaskRequest) returns (Task);
    rpc SubscribeToTask(SubscribeToTaskRequest) returns (stream StreamResponse);
    // Push notification RPCs ...
  }
  ```
- **Metadata Transmission**: Service parameters as gRPC metadata headers:
  - `a2a-version: 1.0`
  - `a2a-extensions: extension-uri-1,extension-uri-2`
  - `authorization: Bearer token`
- **Error Handling**: gRPC status codes (`UNAUTHENTICATED`, `PERMISSION_DENIED`, `NOT_FOUND`, `INVALID_ARGUMENT`, `INTERNAL`) with error details in `google.rpc.Status`.

---

## 5. Security Model

### Authentication Schemes

**APIKeySecurityScheme**:
- `in`: (string) `"header"`, `"query"`, or `"cookie"`.
- `name`: (string) Parameter name (e.g., `X-API-Key`).

**HTTPAuthSecurityScheme**:
- `scheme`: (string) `"basic"`, `"bearer"`, or custom.

**OAuth2SecurityScheme**:
- `flows`: ([OAuthFlows](#oauthflows)) Authorization code, client credentials, device code flows.

**OpenIdConnectSecurityScheme**:
- `openIdConnectUrl`: (string) Discovery endpoint.

**MutualTLSSecurityScheme**:
- Certificate-based mutual authentication.

### OAuthFlows
- `authorizationCode`: ([AuthorizationCodeOAuthFlow](#authorizationcodeoauthflow), optional)
- `clientCredentials`: ([ClientCredentialsOAuthFlow](#clientcredentialsoauthflow), optional)
- `deviceCode`: ([DeviceCodeOAuthFlow](#devicecodeoauthflow), optional)

### AuthorizationCodeOAuthFlow
- `authorizationUrl`: (string URL) Authorization endpoint.
- `tokenUrl`: (string URL) Token endpoint.
- `refreshUrl`: (string URL, optional) Token refresh endpoint.
- `scopes`: (Map\<string, string\>) Available scopes.

### ClientCredentialsOAuthFlow
- `tokenUrl`: (string URL) Token endpoint.
- `refreshUrl`: (string URL, optional) Token refresh endpoint.
- `scopes`: (Map\<string, string\>) Available scopes.

### DeviceCodeOAuthFlow
- `tokenUrl`: (string URL) Token endpoint.
- `deviceAuthorizationUrl`: (string URL) Device authorization endpoint.
- `scopes`: (Map\<string, string\>) Available scopes.

### Integrity & Verification
- **Agent Card Signing**: Canonicalization with fields sorted lexicographically, compact JSON representation. Signature is base64-encoded hash with algorithm identifier. Clients validate using published public key or JWKS endpoint.
- **Transport Security**: HTTPS/TLS required for all HTTP bindings.

### Authorization Scoping
- Agents MUST enforce user/client authorization for task access.
- Clients can only retrieve tasks they created or are explicitly granted.
- List operations filter by authenticated identity.
- Clients MUST NOT be informed of existence of unauthorized resources.

### Push Notification Security

**PushNotificationConfig**:
- `url`: (string URL) Webhook endpoint.
- `authentication`: ([PushNotificationAuthInfo](#pushnotificationauthinfo), optional) Credentials for webhook calls.

**PushNotificationAuthInfo**:
- `scheme`: (string) `"bearer"`, `"basic"`, `"api_key"`.
- `headerName`: (string, optional) Header for credential transmission.
- `value`: (string, optional) Credential value (bearer token, API key).
