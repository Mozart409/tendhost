# Quick Reference: kameo 0.19 API

## Actor Definition

```rust
use kameo::prelude::*;

// Simple actor with derive macro
#[derive(Actor, Default)]
struct SimpleActor {
    count: i64,
}

// Complex actor with custom lifecycle
struct ComplexActor {
    state: MyState,
}

impl Actor for ComplexActor {
    type Args = MyArgs;           // Passed to on_start
    type Error = MyError;         // Error type for lifecycle hooks

    async fn on_start(
        args: Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        // Initialize actor from args
        Ok(Self { state: args.into() })
    }

    async fn on_panic(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        err: PanicError,
    ) -> Result<ControlFlow<ActorStopReason>, Self::Error> {
        // Handle panic, return Continue to keep running or Break to stop
        Ok(ControlFlow::Continue(()))
    }

    async fn on_stop(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        // Cleanup
        Ok(())
    }
}
```

## Message Handler

```rust
use kameo::message::{Context, Message};

struct MyMessage {
    data: String,
}

impl Message<MyMessage> for MyActor {
    type Reply = Result<String, MyError>;

    async fn handle(
        &mut self,
        msg: MyMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Handle message, return reply
        Ok(format!("Processed: {}", msg.data))
    }
}
```

## Spawning Actors

```rust
// Spawn with default bounded mailbox (capacity 64)
let actor_ref = MyActor::spawn(args);

// Spawn with custom mailbox
let actor_ref = MyActor::spawn_with_mailbox(args, kameo::mailbox::bounded(1000));
let actor_ref = MyActor::spawn_with_mailbox(args, kameo::mailbox::unbounded());

// Spawn linked to another actor (supervision)
let child = ChildActor::spawn_link(&parent, child_args).await;
```

## Sending Messages

```rust
// Ask: send and wait for reply
let result = actor_ref.ask(MyMessage { data: "hello".into() }).await?;

// Tell: fire and forget
actor_ref.tell(MyMessage { data: "hello".into() }).send().await?;

// Try send (non-blocking, may fail if mailbox full)
actor_ref.tell(MyMessage { data: "hello".into() }).try_send()?;
```

## Actor Lifecycle Control

```rust
// Stop gracefully (processes remaining messages)
actor_ref.stop_gracefully().await?;

// Kill immediately
actor_ref.kill();

// Wait for startup to complete
actor_ref.wait_for_startup().await;

// Wait for shutdown
actor_ref.wait_for_shutdown().await;

// Check if alive
if actor_ref.is_alive() { /* ... */ }
```

## Error Handling

```rust
use kameo::error::SendError;

match actor_ref.ask(MyMessage { data: "test".into() }).await {
    Ok(result) => println!("Success: {result}"),
    Err(SendError::HandlerError(e)) => println!("Handler error: {e}"),
    Err(SendError::ActorNotRunning(_)) => println!("Actor stopped"),
    Err(SendError::MailboxFull(_)) => println!("Mailbox full"),
    Err(e) => println!("Other error: {e:?}"),
}
```

## Linking and Supervision

```rust
// Link two actors (bidirectional death notification)
parent.link(&child).await;

// Unlink
parent.unlink(&child).await;

// Handle link death in Actor impl
async fn on_link_died(
    &mut self,
    actor_ref: WeakActorRef<Self>,
    id: ActorID,
    reason: ActorStopReason,
) -> Result<ControlFlow<ActorStopReason>, Self::Error> {
    // Decide whether to stop or continue
    Ok(ControlFlow::Continue(()))
}
```

## Broadcast Channel (for WebSocket events)

```rust
use tokio::sync::broadcast;

// Create channel
let (tx, _rx) = broadcast::channel::<WsEvent>(1024);

// Subscribe (multiple consumers)
let rx = tx.subscribe();

// Send (ignores if no subscribers)
let _ = tx.send(WsEvent::HostStateChanged { /* ... */ });

// Receive
match rx.recv().await {
    Ok(event) => handle_event(event),
    Err(broadcast::error::RecvError::Lagged(n)) => {
        // Missed n messages
    }
    Err(broadcast::error::RecvError::Closed) => {
        // Channel closed
    }
}
```

## Common Imports

```rust
// Core kameo
use kameo::prelude::*;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::{ActorStopReason, SendError};
use kameo::message::{Context, Message};

// Mailbox configuration
use kameo::mailbox;

// For async traits
use async_trait::async_trait;

// For broadcast events
use tokio::sync::broadcast;
```

## Project-Specific Patterns

### HostActor Args Pattern
```rust
pub struct HostActorArgs {
    pub config: HostConfig,
    pub executor: Arc<dyn RemoteExecutor>,
    pub package_manager: Arc<dyn PackageManager>,
    pub event_tx: broadcast::Sender<WsEvent>,
}
```

### State Transition with Event Emission
```rust
fn transition_to(&mut self, new_state: HostState) -> Result<(), CoreError> {
    if !self.state.can_transition_to(new_state) {
        return Err(CoreError::InvalidTransition {
            from: self.state,
            to: new_state,
        });
    }

    let old_state = self.state;
    self.state = new_state;

    // Log transition
    info!(host = %self.config.name, from = %old_state, to = %new_state, "state transition");

    // Emit WebSocket event
    let _ = self.event_tx.send(WsEvent::HostStateChanged {
        host: self.config.name.clone(),
        from: old_state.to_string(),
        to: new_state.to_string(),
    });

    Ok(())
}
```

### Factory Pattern for Dependency Injection
```rust
#[async_trait]
pub trait HostActorFactory: Send + Sync {
    async fn create_executor(&self, config: &HostConfig) -> Arc<dyn RemoteExecutor>;
    async fn create_package_manager(
        &self,
        config: &HostConfig,
        executor: Arc<dyn RemoteExecutor>,
    ) -> Arc<dyn PackageManager>;
}
```
