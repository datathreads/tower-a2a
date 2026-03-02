//! A2A Protocol Specification Compliance Tests
//!
//! Feature-gated tests for both v1.0.0 and v0.3.0 protocol compliance.

// ─── v1.0.0 tests ────────────────────────────────────────────────────────────

#[cfg(feature = "v1")]
mod v1 {
    use serde_json::json;
    use tower_a2a::v1::types::*;

    #[test]
    fn test_role_serialization() {
        let json = serde_json::to_value(Role::User).unwrap();
        assert_eq!(json, "user");
        let json = serde_json::to_value(Role::Agent).unwrap();
        assert_eq!(json, "agent");
    }

    #[test]
    fn test_task_state_screaming_snake_case() {
        assert_eq!(serde_json::to_string(&TaskState::Created).unwrap(), "\"CREATED\"");
        assert_eq!(serde_json::to_string(&TaskState::Working).unwrap(), "\"WORKING\"");
        assert_eq!(serde_json::to_string(&TaskState::Completed).unwrap(), "\"COMPLETED\"");
        assert_eq!(serde_json::to_string(&TaskState::Failed).unwrap(), "\"FAILED\"");
        assert_eq!(serde_json::to_string(&TaskState::Canceled).unwrap(), "\"CANCELED\"");
        assert_eq!(serde_json::to_string(&TaskState::Rejected).unwrap(), "\"REJECTED\"");
        assert_eq!(
            serde_json::to_string(&TaskState::InputRequired).unwrap(),
            "\"INPUT_REQUIRED\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::AuthRequired).unwrap(),
            "\"AUTH_REQUIRED\""
        );
    }

    #[test]
    fn test_part_flat_struct() {
        // v1 Part is a flat struct (proto3 oneof style), not a tagged enum
        let text_part = Part::text("Hello");
        let json = serde_json::to_value(&text_part).unwrap();
        assert_eq!(json["text"], "Hello");
        // Unset fields should be absent
        assert!(json.get("mimeType").is_none());
        assert!(json.get("data").is_none());
        assert!(json.get("fileUri").is_none());
        // No "kind" discriminator field
        assert!(json.get("kind").is_none());
    }

    #[test]
    fn test_part_file_uri() {
        let part = Part::file_uri("https://example.com/doc.pdf", Some("application/pdf".to_string()));
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["fileUri"], "https://example.com/doc.pdf");
        assert_eq!(json["mimeType"], "application/pdf");
        assert!(json.get("text").is_none());
    }

    #[test]
    fn test_message_camelcase_fields() {
        let msg = Message {
            role: Role::User,
            parts: vec![Part::text("Hi")],
            context_id: Some("ctx-1".to_string()),
            task_id: Some("task-1".to_string()),
            message_id: Some("msg-1".to_string()),
            reference_task_ids: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["contextId"], "ctx-1");
        assert_eq!(json["taskId"], "task-1");
        assert_eq!(json["messageId"], "msg-1");
        // No snake_case
        assert!(json.get("context_id").is_none());
    }

    #[test]
    fn test_message_optional_fields_omitted() {
        let msg = Message::user("Test");
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json.get("contextId").is_none());
        assert!(json.get("taskId").is_none());
        assert!(json.get("messageId").is_none());
        assert!(json.get("referenceTaskIds").is_none());
    }

    #[test]
    fn test_message_reference_task_ids() {
        let msg = Message {
            role: Role::Agent,
            parts: vec![Part::text("result")],
            context_id: None,
            task_id: None,
            message_id: None,
            reference_task_ids: Some(vec!["task-a".to_string(), "task-b".to_string()]),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["referenceTaskIds"][0], "task-a");
        assert_eq!(json["referenceTaskIds"][1], "task-b");
    }

    #[test]
    fn test_task_has_messages_not_history() {
        // v1 uses `messages` not `history`
        let task = Task {
            id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                timestamp: None,
            },
            state: TaskState::Working,
            messages: Some(vec![Message::user("hello")]),
            artifacts: None,
            created_time: None,
            updated_time: None,
        };
        let json = serde_json::to_value(&task).unwrap();
        assert!(json.get("messages").is_some());
        // v1 does NOT use "history"
        assert!(json.get("history").is_none());
    }

    #[test]
    fn test_task_created_time_updated_time() {
        let task = Task {
            id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Created,
                timestamp: None,
            },
            state: TaskState::Created,
            messages: None,
            artifacts: None,
            created_time: Some(chrono::Utc::now()),
            updated_time: Some(chrono::Utc::now()),
        };
        let json = serde_json::to_value(&task).unwrap();
        assert!(json.get("createdTime").is_some());
        assert!(json.get("updatedTime").is_some());
    }

    #[test]
    fn test_agent_card_interfaces_plural() {
        // v1 uses `interfaces` (plural) not `interface`
        let card = AgentCard {
            id: "agent-1".to_string(),
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            provider: AgentProvider {
                id: "provider-1".to_string(),
                name: "Test Provider".to_string(),
                description: None,
            },
            capabilities: AgentCapabilities::default(),
            skills: vec![],
            interfaces: vec![AgentInterface {
                interface_type: "json-rpc".to_string(),
                uri: "https://example.com/rpc".parse().unwrap(),
            }],
            version: "1.0.0".to_string(),
            security_schemes: None,
            security: None,
            extensions: None,
            signature: None,
        };
        let json = serde_json::to_value(&card).unwrap();
        assert!(json.get("interfaces").is_some());
        assert!(json["interfaces"].is_array());
        // NOT singular "interface"
        assert!(json.get("interface").is_none());
    }

    #[test]
    fn test_agent_provider_v1_fields() {
        // v1 AgentProvider has id, name, description (not organization/url)
        let provider = AgentProvider {
            id: "prov-1".to_string(),
            name: "Acme Corp".to_string(),
            description: Some("AI provider".to_string()),
        };
        let json = serde_json::to_value(&provider).unwrap();
        assert_eq!(json["id"], "prov-1");
        assert_eq!(json["name"], "Acme Corp");
        assert_eq!(json["description"], "AI provider");
        // NOT "organization" or "url" (those are v0.3 fields)
        assert!(json.get("organization").is_none());
    }

    #[test]
    fn test_agent_interface_type_uri() {
        // v1 AgentInterface has type and uri (not protocol/endpoint/version)
        let iface = AgentInterface {
            interface_type: "json-rpc".to_string(),
            uri: "https://example.com/api".parse().unwrap(),
        };
        let json = serde_json::to_value(&iface).unwrap();
        assert_eq!(json["type"], "json-rpc");
        assert!(json.get("uri").is_some());
        // NOT protocol/endpoint/version
        assert!(json.get("protocol").is_none());
        assert!(json.get("endpoint").is_none());
        assert!(json.get("version").is_none());
    }

    #[test]
    fn test_agent_capabilities_no_multi_turn() {
        // v1 AgentCapabilities has streaming, pushNotifications, extendedAgentCard
        // but NOT multiTurn
        let caps = AgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
            extended_agent_card: Some(true),
        };
        let json = serde_json::to_value(&caps).unwrap();
        assert_eq!(json["streaming"], true);
        assert_eq!(json["pushNotifications"], false);
        assert_eq!(json["extendedAgentCard"], true);
        // NOT multiTurn
        assert!(json.get("multiTurn").is_none());
    }

    #[test]
    fn test_task_state_terminal() {
        assert!(TaskState::Completed.is_terminal());
        assert!(TaskState::Failed.is_terminal());
        assert!(TaskState::Canceled.is_terminal());
        assert!(TaskState::Rejected.is_terminal());
        assert!(!TaskState::Created.is_terminal());
        assert!(!TaskState::Working.is_terminal());
        assert!(!TaskState::InputRequired.is_terminal());
    }

    #[test]
    fn test_codec_method_names() {
        use tower_a2a::v1::codec::{Codec, JsonRpcCodec};

        let codec = JsonRpcCodec;

        let op = Operation::SendMessage {
            message: Message::user("hi"),
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "A2A.SendMessage");

        let op = Operation::SendStreamingMessage {
            message: Message::user("hi"),
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "A2A.SendStreamingMessage");

        let op = Operation::GetTask {
            id: "t1".to_string(),
            history_length: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "A2A.GetTask");

        let op = Operation::ListTasks(ListTasksParams::default());
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "A2A.ListTasks");

        let op = Operation::CancelTask { id: "t1".to_string() };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "A2A.CancelTask");
    }

    #[test]
    fn test_round_trip_serialization() {
        let msg = Message {
            role: Role::Agent,
            parts: vec![
                Part::text("Hello"),
                Part::data(json!({"key": "value"})),
            ],
            context_id: Some("ctx-1".to_string()),
            task_id: None,
            message_id: Some("msg-1".to_string()),
            reference_task_ids: None,
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        assert_eq!(msg, deserialized);
    }
}

// ─── v0.3.0 tests ────────────────────────────────────────────────────────────

#[cfg(feature = "v0_3")]
mod v0_3 {
    use tower_a2a::v0_3::types::*;

    #[test]
    fn test_role_serialization() {
        let json = serde_json::to_value(Role::User).unwrap();
        assert_eq!(json, "user");
        let json = serde_json::to_value(Role::Agent).unwrap();
        assert_eq!(json, "agent");
    }

    #[test]
    fn test_task_state_kebab_case() {
        assert_eq!(serde_json::to_string(&TaskState::Queued).unwrap(), "\"queued\"");
        assert_eq!(serde_json::to_string(&TaskState::Running).unwrap(), "\"running\"");
        assert_eq!(serde_json::to_string(&TaskState::Completed).unwrap(), "\"completed\"");
        assert_eq!(serde_json::to_string(&TaskState::Failed).unwrap(), "\"failed\"");
        assert_eq!(serde_json::to_string(&TaskState::Canceled).unwrap(), "\"canceled\"");
        assert_eq!(
            serde_json::to_string(&TaskState::AuthRequired).unwrap(),
            "\"auth-required\""
        );
        // v0.3 does NOT have "running" spelled as "in-progress" or "input-required" as a state
    }

    #[test]
    fn test_part_uses_kind_discriminator() {
        // v0.3 Part is a tagged enum with `kind` field
        let text = Part::text("Hello");
        let json = serde_json::to_value(&text).unwrap();
        assert_eq!(json["kind"], "text");
        assert_eq!(json["text"], "Hello");
        // No file/data fields present
        assert!(json.get("file").is_none());

        let data = Part::data(serde_json::json!({"x": 1}));
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["kind"], "data");
        assert!(json.get("data").is_some());
    }

    #[test]
    fn test_file_part_structure() {
        let file = Part::file_uri("doc.pdf", "https://example.com/doc.pdf");
        let json = serde_json::to_value(&file).unwrap();
        assert_eq!(json["kind"], "file");
        // Nested file object
        assert!(json["file"].is_object());
    }

    #[test]
    fn test_task_status_message_is_message_type() {
        // v0.3 TaskStatus.message is a Message, NOT a string
        let status = TaskStatus {
            state: TaskState::Running,
            message: Some(Message::agent("Processing your request")),
            timestamp: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        // message should be an object with role+parts, not a string
        assert!(json["message"].is_object());
        assert_eq!(json["message"]["role"], "agent");
        assert!(json["message"]["parts"].is_array());
    }

    #[test]
    fn test_task_uses_history_not_messages() {
        // v0.3 uses `history`, NOT `messages`
        let task = Task {
            id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Running,
                message: None,
                timestamp: None,
            },
            history: Some(vec![Message::user("hello")]),
            artifacts: None,
            metadata: None,
            kind: "task".to_string(),
        };
        let json = serde_json::to_value(&task).unwrap();
        assert!(json.get("history").is_some());
        assert!(json.get("messages").is_none());
    }

    #[test]
    fn test_task_kind_field_is_task() {
        let task = Task {
            id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Queued,
                message: None,
                timestamp: None,
            },
            history: None,
            artifacts: None,
            metadata: None,
            kind: "task".to_string(),
        };
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["kind"], "task");
    }

    #[test]
    fn test_agent_provider_v0_3_fields() {
        // v0.3 AgentProvider has organization and url
        let provider = AgentProvider {
            organization: "Acme Corp".to_string(),
            url: "https://acme.example.com".to_string(),
        };
        let json = serde_json::to_value(&provider).unwrap();
        assert_eq!(json["organization"], "Acme Corp");
        assert_eq!(json["url"], "https://acme.example.com");
        // NOT "id" or "name" (those are v1 fields)
        assert!(json.get("id").is_none());
    }

    #[test]
    fn test_codec_method_names() {
        use tower_a2a::v0_3::codec::{Codec, JsonRpcCodec};

        let codec = JsonRpcCodec;

        let op = Operation::SendMessage {
            message: Message::user("hi"),
            task_id: None,
            skill_id: None,
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "message/send");

        let op = Operation::StreamMessage {
            message: Message::user("hi"),
            task_id: None,
            skill_id: None,
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "message/stream");

        let op = Operation::GetTask {
            id: "t1".to_string(),
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "tasks/get");

        let op = Operation::CancelTask { id: "t1".to_string() };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "tasks/cancel");

        let op = Operation::SetPushConfig {
            task_id: "t1".to_string(),
            config_id: "cfg-1".to_string(),
            config: PushNotificationConfig {
                url: "https://example.com/hook".to_string(),
                authentication_info: None,
            },
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "tasks/pushNotificationConfig/set");
    }

    #[test]
    fn test_round_trip_message() {
        let msg = Message {
            role: Role::User,
            parts: vec![Part::text("Hello")],
        };
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_agent_capabilities_extensions() {
        let caps = AgentCapabilities {
            streaming: Some(true),
            push_notifications: None,
            state_transition_history: None,
            extensions: Some(vec![AgentExtension {
                uri: "https://example.com/ext".to_string(),
                description: Some("My extension".to_string()),
                required: Some(false),
                params: None,
            }]),
        };
        let json = serde_json::to_value(&caps).unwrap();
        assert!(json.get("extensions").is_some());
        assert!(json["extensions"].is_array());
        assert_eq!(json["extensions"][0]["uri"], "https://example.com/ext");
    }

    #[test]
    fn test_agent_skill_security() {
        let skill = AgentSkill {
            id: "skill-1".to_string(),
            name: "My Skill".to_string(),
            description: "Does things".to_string(),
            tags: vec![],
            examples: None,
            input_modes: None,
            output_modes: None,
            security: Some(vec![{
                let mut m = std::collections::HashMap::new();
                m.insert("oauth2".to_string(), vec!["read".to_string()]);
                m
            }]),
        };
        let json = serde_json::to_value(&skill).unwrap();
        assert!(json.get("security").is_some());
        assert!(json["security"].is_array());
    }

    #[test]
    fn test_push_notification_auth_headers() {
        let auth = PushNotificationAuthInfo {
            auth_type: "bearer".to_string(),
            token: Some("tok".to_string()),
            api_key: None,
            username: None,
            password: None,
            headers: Some({
                let mut m = std::collections::HashMap::new();
                m.insert("X-Custom".to_string(), "value".to_string());
                m
            }),
        };
        let json = serde_json::to_value(&auth).unwrap();
        assert!(json.get("headers").is_some());
        assert_eq!(json["headers"]["X-Custom"], "value");
    }

    #[test]
    fn test_task_push_notification_config() {
        use tower_a2a::v0_3::types::TaskPushNotificationConfig;

        let cfg = TaskPushNotificationConfig {
            id: "cfg-1".to_string(),
            task_id: "task-1".to_string(),
            config: PushNotificationConfig {
                url: "https://example.com/hook".to_string(),
                authentication_info: None,
            },
        };
        let json = serde_json::to_value(&cfg).unwrap();
        assert_eq!(json["id"], "cfg-1");
        assert_eq!(json["taskId"], "task-1");
        assert!(json.get("config").is_some());
        // No snake_case keys
        assert!(json.get("task_id").is_none());
    }

    #[test]
    fn test_send_message_has_task_id_param() {
        use tower_a2a::v0_3::codec::{Codec, JsonRpcCodec};

        let codec = JsonRpcCodec;
        let op = Operation::SendMessage {
            message: Message::user("continue"),
            task_id: Some("existing-task".to_string()),
            skill_id: None,
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["params"]["taskId"], "existing-task");
    }

    #[test]
    fn test_set_push_config_includes_id() {
        use tower_a2a::v0_3::codec::{Codec, JsonRpcCodec};

        let codec = JsonRpcCodec;
        let op = Operation::SetPushConfig {
            task_id: "task-1".to_string(),
            config_id: "my-config".to_string(),
            config: PushNotificationConfig {
                url: "https://example.com/hook".to_string(),
                authentication_info: None,
            },
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["params"]["id"], "my-config");
        assert_eq!(json["params"]["taskId"], "task-1");
    }

    #[test]
    fn test_sse_terminal_from_task_status_state() {
        use tower_a2a::v0_3::codec::SseEvent;
        use serde_json::json;

        // Terminal via taskStatus.state
        let event = SseEvent {
            kind: "TaskStatusUpdateEvent".to_string(),
            payload: json!({
                "kind": "TaskStatusUpdateEvent",
                "taskId": "t1",
                "taskStatus": { "state": "completed" }
            }),
            final_event: false,
        };
        assert!(event.is_terminal());

        // Non-terminal
        let event = SseEvent {
            kind: "TaskStatusUpdateEvent".to_string(),
            payload: json!({
                "kind": "TaskStatusUpdateEvent",
                "taskId": "t1",
                "taskStatus": { "state": "running" }
            }),
            final_event: false,
        };
        assert!(!event.is_terminal());

        // Terminal via final_event flag
        let event = SseEvent {
            kind: "TaskArtifactUpdateEvent".to_string(),
            payload: json!({ "final": true }),
            final_event: true,
        };
        assert!(event.is_terminal());
    }

    #[test]
    fn test_send_message_decodes_message_response() {
        use tower_a2a::v0_3::codec::{Codec, JsonRpcCodec};
        use serde_json::json;

        let codec = JsonRpcCodec;
        let op = Operation::SendMessage {
            message: Message::user("hello"),
            task_id: None,
            skill_id: None,
            configuration: None,
        };

        // Response is a Message (no kind: "task")
        let response_body = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": "req-1",
            "result": {
                "role": "agent",
                "parts": [{ "kind": "text", "text": "Hi there!" }]
            }
        }))
        .unwrap();

        let resp = codec.decode_response(&response_body, &op).unwrap();
        assert!(resp.into_message().is_some());

        // Response is a Task (kind: "task")
        let response_body = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": "req-2",
            "result": {
                "kind": "task",
                "id": "task-1",
                "contextId": "ctx-1",
                "status": { "state": "queued" }
            }
        }))
        .unwrap();
        let resp = codec.decode_response(&response_body, &op).unwrap();
        assert!(resp.into_task().is_some());
    }
}
