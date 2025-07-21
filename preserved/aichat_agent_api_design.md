# AIChat Agent API Design

## Overview

This document defines the public API for the aichat-agent library, designed to provide complete control over agent behavior while maintaining flexibility for various use cases. The API follows Rust best practices and provides both high-level convenience methods and low-level control.

## Core Design Principles

1. **Complete Context Control**: Every aspect of the agent's context window is controllable
2. **No Hidden State**: All state is explicit and visible to the library user
3. **Instance-Based**: No global state; multiple agents can run concurrently
4. **Async-First**: All I/O operations are async, letting users control the runtime
5. **Zero Overhead**: Direct access to internals without unnecessary abstractions
6. **Extensible by Default**: Every toggle, parameter, and behavior is exposed

## Primary API Components

### 1. Agent Builder

```rust
use aichat_agent::{AgentBuilder, AgentConfig, Model};

// High-level builder API
let agent = AgentBuilder::new()
    .model(Model::Claude3Opus)
    .temperature(0.7)
    .max_tokens(4096)
    .system_prompt("You are a helpful assistant")
    .api_key("sk-...")
    .build()
    .await?;

// Low-level control with custom config
let config = AgentConfig {
    model: Model::Custom("gpt-4-turbo".into()),
    temperature: Some(0.5),
    top_p: Some(0.9),
    max_tokens: Some(8192),
    stream: true,
    ..Default::default()
};

let agent = AgentBuilder::from_config(config)
    .build()
    .await?;
```

### 2. Context Management

```rust
use aichat_agent::{Context, Message, Role};

// Full control over context window
let mut context = Context::new();

// Add system message
context.add_message(Message {
    role: Role::System,
    content: "You have access to a knowledge graph...".into(),
    ..Default::default()
});

// Inject knowledge graph data
context.add_message(Message {
    role: Role::System,
    content: format!("Current graph context: {}", kg_data),
    name: Some("knowledge_graph".into()),
    ..Default::default()
});

// Add conversation history
context.add_message(Message {
    role: Role::User,
    content: "What do you know about Rust?".into(),
    ..Default::default()
});

// Manual context window management
context.truncate_to_tokens(3000); // Keep most recent messages
context.compress_with_summarization(&agent).await?; // Summarize old messages
```

### 3. Function/Tool Registration

```rust
use aichat_agent::{Function, FunctionRegistry, ToolCall, ToolResult};

// Define custom functions
let kg_search = Function {
    name: "search_knowledge_graph".into(),
    description: "Search the knowledge graph for concepts".into(),
    parameters: json!({
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "depth": {"type": "integer", "default": 2}
        },
        "required": ["query"]
    }),
    handler: Box::new(|args| async move {
        // Your KG search implementation
        let result = search_kg(args).await?;
        Ok(ToolResult::Success(result))
    }),
};

// Register functions
let mut registry = FunctionRegistry::new();
registry.register(kg_search);
registry.register_batch(vec![func1, func2, func3]);

// Attach to agent
agent.set_functions(registry);

// Or dynamically per request
let response = agent.chat_with_functions(&context, &registry).await?;
```

### 4. Streaming and Events

```rust
use aichat_agent::{StreamEvent, ChatStream};

// Stream responses with full event access
let mut stream = agent.stream_chat(&context).await?;

while let Some(event) = stream.next().await {
    match event {
        StreamEvent::Token(token) => print!("{}", token),
        StreamEvent::ToolCallStart(name, id) => {
            println!("Calling function: {}", name);
        },
        StreamEvent::ToolCallResult(id, result) => {
            // Handle tool results
        },
        StreamEvent::Error(e) => eprintln!("Error: {}", e),
        StreamEvent::Done => break,
    }
}

// Or use high-level stream
let response = agent.stream_chat_simple(&context)
    .await?
    .try_collect::<String>()
    .await?;
```

### 5. Agent Lifecycle Hooks

```rust
use aichat_agent::{AgentHooks, HookEvent};

// Define lifecycle hooks
struct CymbiontHooks {
    kg_client: KnowledgeGraphClient,
}

#[async_trait]
impl AgentHooks for CymbiontHooks {
    async fn before_message(&mut self, context: &mut Context) -> Result<()> {
        // Inject KG context before each message
        let kg_context = self.kg_client.get_relevant_context(&context).await?;
        context.inject_system_message(&kg_context);
        Ok(())
    }
    
    async fn after_tool_call(&mut self, call: &ToolCall, result: &ToolResult) -> Result<()> {
        // Update KG based on tool results
        if call.name == "create_note" {
            self.kg_client.index_new_note(&result).await?;
        }
        Ok(())
    }
    
    async fn on_context_overflow(&mut self, context: &mut Context) -> Result<()> {
        // Custom context management when hitting token limits
        let summary = self.kg_client.create_smart_summary(&context).await?;
        context.compress_with_summary(summary);
        Ok(())
    }
}

// Attach hooks to agent
agent.set_hooks(Box::new(CymbiontHooks { kg_client }));
```

### 6. Advanced Configuration

```rust
use aichat_agent::{ClientConfig, RetryPolicy, CacheConfig};

// Fine-grained client configuration
let client_config = ClientConfig {
    timeout: Duration::from_secs(60),
    retry_policy: RetryPolicy::exponential_backoff(3),
    proxy: Some("http://proxy:8080".into()),
    extra_headers: vec![
        ("X-Custom-Header", "value")
    ],
    ..Default::default()
};

// Response caching for development
let cache_config = CacheConfig {
    enabled: true,
    ttl: Duration::from_secs(3600),
    max_size: 1024 * 1024 * 100, // 100MB
    key_fn: Box::new(|req| {
        // Custom cache key generation
        format!("{:?}-{:?}", req.model, req.messages)
    }),
};

let agent = AgentBuilder::new()
    .client_config(client_config)
    .cache_config(cache_config)
    .build()
    .await?;
```

### 7. State Management

```rust
use aichat_agent::{AgentState, Session};

// Save agent state
let state = agent.save_state()?;
let json = serde_json::to_string(&state)?;

// Restore agent from state
let state: AgentState = serde_json::from_str(&json)?;
let agent = Agent::from_state(state).await?;

// Session management
let session = Session::new("user-123");
session.add_message(message);
session.set_variable("username", "Alice");

// Attach session to agent
agent.set_session(session);

// Or use session-scoped agents
let agent = agent.with_session(session);
```

### 8. Low-Level Access

```rust
// Direct access to underlying client
let client = agent.client();
let raw_response = client.complete_raw(request).await?;

// Manual message construction
let messages = agent.build_messages(&context)?;
let request = agent.build_request(messages)?;
let response = agent.execute_request(request).await?;

// Custom token counting
let token_count = agent.count_tokens(&context)?;
let remaining = agent.remaining_context_window()?;

// Direct model introspection
let model_info = agent.model_info();
println!("Max tokens: {}", model_info.context_window);
println!("Supports functions: {}", model_info.supports_functions);
println!("Supports vision: {}", model_info.supports_vision);
```

## Integration Example for Cymbiont

```rust
use aichat_agent::{Agent, AgentBuilder, Context, Message, Function};
use cymbiont::{KnowledgeGraph, KGHooks};

pub struct CymbiontAgent {
    agent: Agent,
    kg: KnowledgeGraph,
}

impl CymbiontAgent {
    pub async fn new(config: Config) -> Result<Self> {
        // Build the underlying agent
        let agent = AgentBuilder::new()
            .model(config.model)
            .temperature(config.temperature)
            .api_key(&config.api_key)
            .build()
            .await?;
        
        // Initialize knowledge graph
        let kg = KnowledgeGraph::load(&config.kg_path).await?;
        
        // Register KG-specific functions
        let mut functions = FunctionRegistry::new();
        functions.register(kg_search_function());
        functions.register(kg_update_function());
        functions.register(kg_traverse_function());
        agent.set_functions(functions);
        
        // Attach KG hooks for context injection
        agent.set_hooks(Box::new(KGHooks::new(kg.clone())));
        
        Ok(Self { agent, kg })
    }
    
    pub async fn chat(&mut self, message: &str) -> Result<String> {
        // Build context with KG data
        let mut context = Context::new();
        
        // Add KG-aware system prompt
        context.add_system_message(&self.build_kg_prompt().await?);
        
        // Add relevant graph context
        let graph_context = self.kg.get_relevant_nodes(message).await?;
        context.add_system_message(&format!(
            "Relevant knowledge graph nodes:\n{}",
            serde_json::to_string_pretty(&graph_context)?
        ));
        
        // Add user message
        context.add_user_message(message);
        
        // Get response with full KG integration
        let response = self.agent.chat(&context).await?;
        
        // Update KG based on conversation
        self.kg.add_conversation_node(message, &response).await?;
        
        Ok(response.content)
    }
}
```

## Migration Path from Current AIChat

1. **Phase 1**: Create lib.rs that exports minimal API
2. **Phase 2**: Refactor internals to remove global state
3. **Phase 3**: Implement builder pattern and instance-based config
4. **Phase 4**: Add hook system for extensibility
5. **Phase 5**: Expose low-level APIs for advanced use cases

## Benefits for Cymbiont

1. **Complete Context Control**: Can inject KG data at any point in the conversation
2. **Function Integration**: Easy to add KG-specific tools
3. **State Management**: Can maintain KG state alongside agent state
4. **Streaming Control**: Can intercept and modify responses in real-time
5. **Multi-Agent**: Can run multiple specialized agents concurrently
6. **No Overhead**: Direct function calls, no HTTP/serialization costs

This API design provides maximum flexibility while maintaining ease of use. Cymbiont gets full control over the agent behavior while the library remains generic enough for other use cases.