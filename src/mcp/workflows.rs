//! MCP Workflows using PocketFlow
//! Flow-based programming for complex MCP agent interactions

use anyhow::Result;
use pocketflow_rs::{Context, Flow, Node, ProcessState};
use serde_json::Value;
use std::sync::Arc;
use async_trait::async_trait;

/// Workflow states for MCP operations
#[derive(Debug, Clone, PartialEq)]
pub enum McpWorkflowState {
    /// Initial state
    Start,
    /// Code analysis completed
    CodeAnalyzed,
    /// Tests written/generated
    TestsGenerated,
    /// Documentation updated
    DocsUpdated,
    /// Deployment ready
    ReadyToDeploy,
    /// Operation completed successfully
    Success,
    /// Operation failed
    Failure,
    /// Awaiting user input
    AwaitingInput,
}

impl ProcessState for McpWorkflowState {
    fn is_default(&self) -> bool {
        matches!(self, McpWorkflowState::Start)
    }

    fn to_condition(&self) -> String {
        match self {
            McpWorkflowState::Start => "start",
            McpWorkflowState::CodeAnalyzed => "code_analyzed",
            McpWorkflowState::TestsGenerated => "tests_generated",
            McpWorkflowState::DocsUpdated => "docs_updated",
            McpWorkflowState::ReadyToDeploy => "ready_to_deploy",
            McpWorkflowState::Success => "success",
            McpWorkflowState::Failure => "failure",
            McpWorkflowState::AwaitingInput => "awaiting_input",
        }.to_string()
    }
}

/// MCP Code Review Workflow Node
pub struct CodeReviewNode {
    language: String,
}

impl CodeReviewNode {
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
        }
    }
}

#[async_trait]
impl Node for CodeReviewNode {
    type State = McpWorkflowState;

    async fn prepare(&self, context: &mut Context) -> Result<()> {
        log::info!("🔍 Preparing code review for {} code", self.language);
        context.insert("review_language".to_string(), Value::String(self.language.clone()));
        Ok(())
    }

    async fn execute(&self, context: &Context) -> Result<Self::State> {
        log::info!("⚡ Executing code review workflow");

        // Get code from context
        let code = context.get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Simulate calling MCP agents for code analysis
        log::info!("📝 Analyzing {} lines of {} code", code.lines().count(), self.language);

        // In real implementation, this would call actual MCP agents
        // like rust_pro, python_pro, etc.

        Ok(McpWorkflowState::CodeAnalyzed)
    }

    async fn post_process(&self, context: &mut Context, result: &Self::State) -> Result<()> {
        match result {
            McpWorkflowState::CodeAnalyzed => {
                context.insert("analysis_complete".to_string(), Value::Bool(true));
                log::info!("✅ Code analysis completed");
            }
            _ => {
                log::warn!("⚠️  Unexpected result from code review: {:?}", result);
            }
        }
        Ok(())
    }
}

/// Test Generation Node
pub struct TestGenerationNode;

#[async_trait]
impl Node for TestGenerationNode {
    type State = McpWorkflowState;

    async fn prepare(&self, context: &mut Context) -> Result<()> {
        log::info!("🧪 Preparing test generation");
        Ok(())
    }

    async fn execute(&self, context: &Context) -> Result<Self::State> {
        log::info!("⚡ Generating tests based on code analysis");

        // Check if code analysis was completed
        let analysis_done = context.get("analysis_complete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !analysis_done {
            log::warn!("⚠️  Cannot generate tests without code analysis");
            return Ok(McpWorkflowState::Failure);
        }

        // In real implementation, call test generation agents
        log::info!("📝 Generating comprehensive test suite");

        Ok(McpWorkflowState::TestsGenerated)
    }

    async fn post_process(&self, context: &mut Context, result: &Self::State) -> Result<()> {
        match result {
            McpWorkflowState::TestsGenerated => {
                context.insert("tests_generated".to_string(), Value::Bool(true));
                log::info!("✅ Tests generated successfully");
            }
            McpWorkflowState::Failure => {
                log::error!("❌ Test generation failed");
            }
            _ => {}
        }
        Ok(())
    }
}

/// Documentation Update Node
pub struct DocumentationNode;

#[async_trait]
impl Node for DocumentationNode {
    type State = McpWorkflowState;

    async fn prepare(&self, context: &mut Context) -> Result<()> {
        log::info!("📚 Preparing documentation update");
        Ok(())
    }

    async fn execute(&self, context: &Context) -> Result<Self::State> {
        log::info!("⚡ Updating documentation");

        let tests_done = context.get("tests_generated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !tests_done {
            log::info!("⏳ Awaiting test completion before updating docs");
            return Ok(McpWorkflowState::AwaitingInput);
        }

        // Update documentation
        log::info!("📝 Updating README, API docs, and inline documentation");

        Ok(McpWorkflowState::DocsUpdated)
    }

    async fn post_process(&self, context: &mut Context, result: &Self::State) -> Result<()> {
        match result {
            McpWorkflowState::DocsUpdated => {
                context.insert("docs_updated".to_string(), Value::Bool(true));
                log::info!("✅ Documentation updated");
            }
            McpWorkflowState::AwaitingInput => {
                log::info!("⏳ Documentation update paused - awaiting test completion");
            }
            _ => {}
        }
        Ok(())
    }
}

/// Deployment Preparation Node
pub struct DeploymentNode;

#[async_trait]
impl Node for DeploymentNode {
    type State = McpWorkflowState;

    async fn prepare(&self, context: &mut Context) -> Result<()> {
        log::info!("🚀 Preparing deployment");
        Ok(())
    }

    async fn execute(&self, context: &Context) -> Result<Self::State> {
        log::info!("⚡ Preparing deployment package");

        let docs_done = context.get("docs_updated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !docs_done {
            log::warn!("⚠️  Documentation not complete - proceeding anyway");
        }

        // Prepare deployment artifacts
        log::info!("📦 Creating deployment package with tests and docs");

        Ok(McpWorkflowState::ReadyToDeploy)
    }

    async fn post_process(&self, context: &mut Context, result: &Self::State) -> Result<()> {
        match result {
            McpWorkflowState::ReadyToDeploy => {
                context.insert("deployment_ready".to_string(), Value::Bool(true));
                log::info!("✅ Deployment package ready");
            }
            _ => {}
        }
        Ok(())
    }
}

/// MCP Development Workflow Manager
pub struct McpWorkflowManager {
    flows: std::collections::HashMap<String, Flow<McpWorkflowState>>,
}

impl McpWorkflowManager {
    pub fn new() -> Self {
        Self {
            flows: std::collections::HashMap::new(),
        }
    }

    /// Create a standard code review workflow
    pub fn create_code_review_workflow(&mut self, language: &str) -> Result<()> {
        // Create nodes
        let code_review = Arc::new(CodeReviewNode::new(language));
        let test_gen = Arc::new(TestGenerationNode);
        let docs = Arc::new(DocumentationNode);
        let deploy = Arc::new(DeploymentNode);

        // Create flow starting with code review
        let mut flow = Flow::new("code_review", code_review);
        flow.add_node("test_generation", test_gen);
        flow.add_node("documentation", docs);
        flow.add_node("deployment", deploy);

        // Define workflow transitions
        flow.add_edge("code_review", "test_generation", McpWorkflowState::CodeAnalyzed);
        flow.add_edge("test_generation", "documentation", McpWorkflowState::TestsGenerated);
        flow.add_edge("documentation", "deployment", McpWorkflowState::DocsUpdated);
        flow.add_edge("documentation", "documentation", McpWorkflowState::AwaitingInput); // Wait for tests
        flow.add_edge("deployment", "code_review", McpWorkflowState::ReadyToDeploy); // Loop back for next review

        self.flows.insert(format!("code_review_{}", language), flow);
        Ok(())
    }

    /// Run a workflow with the given context
    pub async fn run_workflow(&self, workflow_name: &str, context: Context) -> Result<Value> {
        if let Some(flow) = self.flows.get(workflow_name) {
            log::info!("🚀 Starting MCP workflow: {}", workflow_name);
            let result = flow.run(context).await?;
            log::info!("✅ MCP workflow completed: {}", workflow_name);
            Ok(result)
        } else {
            Err(anyhow::anyhow!("Workflow '{}' not found", workflow_name))
        }
    }

    /// List available workflows
    pub fn list_workflows(&self) -> Vec<String> {
        self.flows.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_review_workflow() {
        let mut manager = McpWorkflowManager::new();
        manager.create_code_review_workflow("rust").unwrap();

        let workflows = manager.list_workflows();
        assert!(workflows.contains(&"code_review_rust".to_string()));

        // Create test context
        let mut context = Context::new();
        context.insert("code".to_string(), Value::String("fn main() { println!(\"Hello\"); }".to_string()));

        // This would run the full workflow in a real test
        // let result = manager.run_workflow("code_review_rust", context).await;
        // assert!(result.is_ok());
    }
}