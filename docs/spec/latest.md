# A2A Protocol Specification - Latest (v1.0.0)

## 1. Abstract Data Model

### Task
The core unit of work with lifecycle management.
- `taskId`: (string) Server-generated unique identifier.
- `contextId`: (string) Logical grouping for related tasks.
- `status`: ([TaskStatus](#taskstatus)) Current state indicator.
- `history`: (Array of [Message](#message)) Conversation history.
- `artifacts`: (Array of [Artifact](#artifact)) Generated outputs.
- `createdTime`: (string) ISO 8601 timestamp.
- `updatedTime`: (string) ISO 8601 timestamp.
- `metadata`: (object) Flexible key-value context.

### TaskStatus
- `state`: ([TaskState](#taskstate)) Current state enum.
- `message`: (string) Human-readable progress update.
- `timestamp`: (string) ISO 8601 formatted timestamp.

### TaskState
Enum values:
- `WORKING` — Processing in progress.
- `COMPLETED` — Successfully finished.
- `FAILED` — Encountered error.
- `CANCELED` — Client-initiated cancellation.
- `REJECTED` — Agent declined request.
- `INPUT_REQUIRED` — Awaiting client input.
- `AUTH_REQUIRED` — Secondary authentication needed.

### Message
- `messageId`: (string) Optional unique identifier.
- `role`: (string) `"user"` or `"agent"`.
- `parts`: (Array of [Part](#part)) Content segments.
- `timestamp`: (string) ISO 8601 timestamp.

### Part (Union)
Parts are defined as a formal union of types:
- **Text**: `{ "text": string }`
- **File**: `{ "file": { "uri": string, ... }, "mimeType": string }`
- **StructuredData**: `{ "structuredData": object }` (JSON-serializable structured content)
- `mimeType`: (string) Media type descriptor (top-level on Part).

### Artifact
- `artifactId`: (string) Unique identifier.
- `parts`: (Array of [Part](#part)) Constituent content.
- `mimeType`: (string) Primary media type.
- `metadata`: (object) Additional context.

---

## 2. Discovery & Identity

### AgentCard
Standardized metadata for agent discovery, typically served at `/.well-known/agent-card.json`.
- `id`: (string) Agent identifier.
- `name`: (string) Display name.
- `description`: (string) Purpose summary.
- `provider`: ([AgentProvider](#agentprovider)) Organization details.
- `interface`: ([AgentInterface](#agentinterface)) Protocol and endpoint information.
- `capabilities`: ([AgentCapabilities](#agentcapabilities)) Feature declarations.
- `skills`: (Array of [AgentSkill](#agentskill)) Task-specific competencies.
- `extensions`: (Array of [AgentExtension](#agentextension)) Optional enhancements.
- `securitySchemes`: (object) Authentication method definitions.
- `security`: (Array) Required schemes for operations.
- `signature`: ([AgentCardSignature](#agentcardsignature)) Cryptographic verification.

### AgentInterface
- `protocol`: (string) `"json-rpc"`, `"grpc"`, `"http"`, or custom.
- `endpoint`: (string) Service URL or connection details.
- `version`: (string) Interface version number.

### AgentCapabilities
- `streaming`: (boolean) Real-time event delivery support.
- `pushNotifications`: (boolean) Webhook delivery capability.
- `extendedAgentCard`: (boolean) Authenticated detailed card availability.
- `multiTurn`: (boolean) Conversation context maintenance.

### AgentSkill
- `id`: (string) Skill identifier.
- `name`: (string) Human-readable name.
- `description`: (string) Capability overview.
- `inputSchema`: (object) Expected input structure.
- `outputSchema`: (object) Produced output structure.
- `acceptedMimeTypes`: (string[]) Supported content types.

---

## 3. Operations (Abstract Methods)
These operations are binding-agnostic and map to JSON-RPC methods, REST endpoints, or gRPC RPCs.

| Operation | Input | Output |
| :--- | :--- | :--- |
| **SendMessage** | `SendMessageRequest` | `Task \| Message` |
| **SendStreamingMessage** | `SendMessageRequest` | `Stream[Task \| Message \| TaskStatusUpdateEvent \| TaskArtifactUpdateEvent]` |
| **GetTask** | `taskId`, `historyLength?` | `Task` |
| **ListTasks** | `contextId?`, `status?`, `pageSize?`, `pageToken?`, `historyLength?`, `statusTimestampAfter?`, `includeArtifacts?` | `ListTasksResponse` |
| **CancelTask** | `taskId` | `Task` |
| **SubscribeToTask** | `taskId` | `Stream[Task \| TaskStatusUpdateEvent \| TaskArtifactUpdateEvent]` |
| **CreateTaskPushNotificationConfig** | `taskId`, `configId`, `PushNotificationConfig` | `PushNotificationConfig` |
| **GetTaskPushNotificationConfig** | `taskId`, `configId` | `PushNotificationConfig` |
| **ListTaskPushNotificationConfig** | `taskId`, `pageSize?`, `pageToken?` | `ListPushNotificationConfigResponse` |
| **DeleteTaskPushNotificationConfig** | `taskId`, `configId` | Confirmation |
| **GetExtendedAgentCard** | (authenticated) | `AgentCard` |

### SendMessageRequest
- `message`: (Message, required) Content to send.
- `configuration`: (SendMessageConfiguration) Behavior options.
- `metadata`: (object) Context parameters.

### SendMessageConfiguration
- `acceptedOutputModes`: (string[]) Preferred response types.
- `pushNotificationConfig`: (PushNotificationConfig) Webhook setup.
- `historyLength`: (integer) Message retrieval limit.
- `blocking`: (boolean) Wait for completion (default: false).

---

## 4. Binding Requirements

### HTTP Binding
- **Headers**:
  - `A2A-Version`: (Required) e.g., `1.0`.
  - `A2A-Extensions`: (Optional) Comma-separated list of extension URIs.
  - `Authorization`: Authentication credentials.
  - `Content-Type`: Request media type.
- **REST Mappings**:
  - `POST /messages` → `SendMessage`
  - `POST /messages/stream` → `SendStreamingMessage`
  - `GET /tasks/{id}` → `GetTask`
  - `GET /tasks` → `ListTasks`
  - `DELETE /tasks/{id}` → `CancelTask`
  - `GET /tasks/{id}/stream` → `SubscribeToTask`
  - `POST /tasks/{id}/notifications` → `CreateTaskPushNotificationConfig`
  - `GET /tasks/{id}/notifications/{configId}` → `GetTaskPushNotificationConfig`
  - `GET /tasks/{id}/notifications` → `ListTaskPushNotificationConfig`
  - `DELETE /tasks/{id}/notifications/{configId}` → `DeleteTaskPushNotificationConfig`
  - `GET /agent-card` → `GetExtendedAgentCard` (authenticated)
- **Query Parameters**: Lowercase snake_case naming (`context_id`, `page_size`, `page_token`, `history_length`, `include_artifacts`, `status`, `status_timestamp_after`).
- **Response Format**:
  - Success: HTTP 200-299 with JSON body.
  - Errors: HTTP 4xx/5xx with error object containing `code`, `message`, `details`.

### JSON-RPC Binding
- **Method Naming**: Uses `A2A.` prefix with CamelCase (e.g., `A2A.SendMessage`, `A2A.GetTask`).
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
- `flows`: (OAuthFlows) Authorization code, client credentials, device code.
- `tokenUrl`: (string) Token endpoint.
- `authorizationUrl`: (string) User authorization endpoint.
- `refreshUrl`: (string) Token refresh endpoint.
- `scopes`: (object) Requested permissions.

**OpenIdConnectSecurityScheme**:
- `openIdConnectUrl`: (string) Discovery endpoint.

**MutualTLSSecurityScheme**:
- Certificate-based mutual authentication.

### Integrity & Verification
- **Agent Card Signing**: Canonicalization with fields sorted lexicographically, compact JSON representation. Signature is base64-encoded hash with algorithm identifier. Clients validate using published public key or JWKS endpoint.
- **Transport Security**: HTTPS/TLS required for all HTTP bindings. TLS for gRPC connections.

### Authorization Scoping
- Agents MUST enforce user/client authorization for task access.
- Clients can only retrieve tasks they created or are explicitly granted.
- List operations filter by authenticated identity.
- Clients MUST NOT be informed of existence of unauthorized resources.

### Push Notification Security

**AuthenticationInfo**:
- `scheme`: (string) `"bearer"`, `"basic"`, `"api_key"`.
- `headerName`: (string) Header for credential transmission.
- `value`: (string) Credential (bearer token, API key).

**Webhook Delivery**:
- HTTP POST to client-provided URL.
- Credentials sent in request headers/query parameters.
- TLS certificate validation recommended.
- Agents retry failed deliveries (implementation-defined policy).
