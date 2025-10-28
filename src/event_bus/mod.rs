//! Event bus for decoupled component communication
//! 
//! This module provides a publish-subscribe event bus that allows
//! components to communicate without direct dependencies.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use anyhow::{Result, Context};

/// Event trait that all events must implement
pub trait Event: Send + Sync + Any {
    /// Get event type identifier
    fn event_type(&self) -> &'static str;
    
    /// Convert to JSON value for serialization
    fn to_json(&self) -> Value;
    
    /// Clone the event as a boxed trait object
    fn clone_event(&self) -> Box<dyn Event>;
    
    /// Downcast to concrete type
    fn as_any(&self) -> &dyn Any;
}

/// Generic event wrapper for any serializable type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericEvent<T: Clone + Send + Sync + 'static> {
    pub event_type: String,
    pub payload: T,
}

impl<T: Clone + Send + Sync + Serialize + 'static> Event for GenericEvent<T> {
    fn event_type(&self) -> &'static str {
        Box::leak(self.event_type.clone().into_boxed_str())
    }
    
    fn to_json(&self) -> Value {
        serde_json::to_value(&self.payload).unwrap_or(Value::Null)
    }
    
    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Event handler trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: Arc<Box<dyn Event>>) -> Result<()>;
    
    /// Get the event types this handler is interested in
    fn event_types(&self) -> Vec<&'static str>;
}

/// Function-based event handler
pub struct FnHandler {
    handler: Arc<dyn Fn(Arc<Box<dyn Event>>) -> Result<()> + Send + Sync>,
    event_types: Vec<&'static str>,
}

impl FnHandler {
    pub fn new<F>(event_types: Vec<&'static str>, handler: F) -> Self
    where
        F: Fn(Arc<Box<dyn Event>>) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            handler: Arc::new(handler),
            event_types,
        }
    }
}

#[async_trait]
impl EventHandler for FnHandler {
    async fn handle(&self, event: Arc<Box<dyn Event>>) -> Result<()> {
        (self.handler)(event)
    }
    
    fn event_types(&self) -> Vec<&'static str> {
        self.event_types.clone()
    }
}

/// Event bus for publishing and subscribing to events
pub struct EventBus {
    /// Registered handlers by event type
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<Box<dyn EventHandler>>>>>>,
    
    /// Broadcast channels for real-time event streaming
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<Arc<Box<dyn Event>>>>>>,
    
    /// Event history for replay
    history: Arc<RwLock<Vec<Arc<Box<dyn Event>>>>>,
    
    /// Maximum history size
    max_history: usize,
    
    /// Global event interceptors (middleware)
    interceptors: Arc<RwLock<Vec<Box<dyn EventInterceptor>>>>,
}

/// Event interceptor for middleware functionality
#[async_trait]
pub trait EventInterceptor: Send + Sync {
    /// Called before event is published
    async fn before_publish(&self, event: &Box<dyn Event>) -> Result<()>;
    
    /// Called after event is published
    async fn after_publish(&self, event: &Box<dyn Event>);
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 1000,
            interceptors: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            max_history,
            ..Self::new()
        }
    }
    
    /// Register an event handler
    pub async fn register_handler(&self, handler: Box<dyn EventHandler>) -> Result<()> {
        let handler = Arc::new(handler);
        let event_types = handler.event_types();
        
        let mut handlers = self.handlers.write().await;
        for event_type in event_types {
            handlers
                .entry(event_type.to_string())
                .or_insert_with(Vec::new)
                .push(handler.clone());
        }
        
        Ok(())
    }
    
    /// Subscribe to an event type with a callback
    pub async fn subscribe<F>(&self, event_type: &'static str, callback: F) -> Result<()>
    where
        F: Fn(Arc<Box<dyn Event>>) -> Result<()> + Send + Sync + 'static,
    {
        let handler = FnHandler::new(vec![event_type], callback);
        self.register_handler(Box::new(handler)).await
    }
    
    /// Subscribe to an event stream
    pub async fn stream(&self, event_type: &str) -> broadcast::Receiver<Arc<Box<dyn Event>>> {
        let mut channels = self.channels.write().await;
        let sender = channels.entry(event_type.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(100);
                tx
            });
        sender.subscribe()
    }
    
    /// Add an event interceptor
    pub async fn add_interceptor(&self, interceptor: Box<dyn EventInterceptor>) {
        let mut interceptors = self.interceptors.write().await;
        interceptors.push(interceptor);
    }
    
    /// Publish an event
    pub async fn publish(&self, event: Box<dyn Event>) -> Result<()> {
        let event_arc = Arc::new(event);
        
        // Call interceptors (before)
        let interceptors = self.interceptors.read().await;
        for interceptor in interceptors.iter() {
            interceptor.before_publish(&event_arc).await?;
        }
        
        // Add to history
        {
            let mut history = self.history.write().await;
            history.push(event_arc.clone());
            
            // Trim history if needed
            if history.len() > self.max_history {
                let drain_to = history.len().saturating_sub(self.max_history);
                history.drain(0..drain_to);
            }
        }
        
        // Broadcast to stream subscribers
        {
            let channels = self.channels.read().await;
            if let Some(sender) = channels.get(event_arc.event_type()) {
                let _ = sender.send(event_arc.clone());
            }
        }
        
        // Call registered handlers
        {
            let handlers = self.handlers.read().await;
            if let Some(event_handlers) = handlers.get(event_arc.event_type()) {
                for handler in event_handlers {
                    if let Err(e) = handler.handle(event_arc.clone()).await {
                        log::error!("Event handler failed: {}", e);
                    }
                }
            }
        }
        
        // Call interceptors (after)
        for interceptor in interceptors.iter() {
            interceptor.after_publish(&event_arc).await;
        }
        
        Ok(())
    }
    
    /// Get event history
    pub async fn get_history(&self) -> Vec<Arc<Box<dyn Event>>> {
        let history = self.history.read().await;
        history.clone()
    }
    
    /// Get events of a specific type from history
    pub async fn get_history_by_type(&self, event_type: &str) -> Vec<Arc<Box<dyn Event>>> {
        let history = self.history.read().await;
        history.iter()
            .filter(|e| e.event_type() == event_type)
            .cloned()
            .collect()
    }
    
    /// Clear event history
    pub async fn clear_history(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }
}

/// Global event bus instance
static GLOBAL_EVENT_BUS: once_cell::sync::Lazy<EventBus> = 
    once_cell::sync::Lazy::new(|| EventBus::new());

/// Get the global event bus
pub fn global() -> &'static EventBus {
    &GLOBAL_EVENT_BUS
}

/// Macro to define events easily
#[macro_export]
macro_rules! define_event {
    ($name:ident { $($field:ident: $type:ty),* }) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $name {
            $(pub $field: $type,)*
        }
        
        impl Event for $name {
            fn event_type(&self) -> &'static str {
                stringify!($name)
            }
            
            fn to_json(&self) -> Value {
                serde_json::to_value(self).unwrap_or(Value::Null)
            }
            
            fn clone_event(&self) -> Box<dyn Event> {
                Box::new(self.clone())
            }
            
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

// Define common system events
define_event!(PluginRegistered { 
    plugin_name: String,
    version: String
});

define_event!(PluginUnregistered {
    plugin_name: String
});

define_event!(StateChanged {
    plugin: String,
    old_state: Value,
    new_state: Value
});

define_event!(AgentSpawned {
    agent_id: String,
    agent_type: String
});

define_event!(AgentDied {
    agent_id: String,
    reason: String
});

define_event!(TaskCompleted {
    task_id: String,
    agent_id: String,
    result: Value
});

define_event!(ToolExecuted {
    tool_name: String,
    params: Value,
    success: bool
});

define_event!(ErrorOccurred {
    context: String,
    error: String,
    severity: String
});

/// Example logging interceptor
pub struct LoggingInterceptor;

#[async_trait]
impl EventInterceptor for LoggingInterceptor {
    async fn before_publish(&self, event: &Box<dyn Event>) -> Result<()> {
        log::debug!("Publishing event: {} - {:?}", event.event_type(), event.to_json());
        Ok(())
    }
    
    async fn after_publish(&self, event: &Box<dyn Event>) {
        log::trace!("Event published: {}", event.event_type());
    }
}

/// Example metrics interceptor
pub struct MetricsInterceptor {
    event_counts: Arc<RwLock<HashMap<String, u64>>>,
}

impl MetricsInterceptor {
    pub fn new() -> Self {
        Self {
            event_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn get_metrics(&self) -> HashMap<String, u64> {
        let counts = self.event_counts.read().await;
        counts.clone()
    }
}

#[async_trait]
impl EventInterceptor for MetricsInterceptor {
    async fn before_publish(&self, _event: &Box<dyn Event>) -> Result<()> {
        Ok(())
    }
    
    async fn after_publish(&self, event: &Box<dyn Event>) {
        let mut counts = self.event_counts.write().await;
        *counts.entry(event.event_type().to_string()).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new();
        
        // Subscribe to events
        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();
        
        bus.subscribe("TestEvent", move |event| {
            let mut r = received_clone.blocking_write();
            r.push(event.event_type().to_string());
            Ok(())
        }).await.unwrap();
        
        // Define a test event
        define_event!(TestEvent { message: String });
        
        // Publish event
        let event = TestEvent {
            message: "Hello, World!".to_string(),
        };
        
        bus.publish(Box::new(event)).await.unwrap();
        
        // Check received
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let r = received.read().await;
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], "TestEvent");
    }
}