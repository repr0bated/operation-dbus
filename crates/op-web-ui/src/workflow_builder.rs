use yew::prelude::*;
use web_sys::{DragEvent, HtmlElement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub x: f64,
    pub y: f64,
    pub config: HashMap<String, String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Service(String),
    Plugin(String),
    Condition,
    Parallel,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

#[derive(Properties, PartialEq)]
pub struct WorkflowBuilderProps {
    pub services: Vec<String>,
    pub plugins: Vec<String>,
}

pub struct WorkflowBuilder {
    workflow: Workflow,
    dragging: Option<String>,
    selected: Option<String>,
}

pub enum Msg {
    DragStart(String),
    Drop(f64, f64),
    SelectNode(String),
    Connect(String, String),
    DeleteNode(String),
    Save,
}

impl Component for WorkflowBuilder {
    type Message = Msg;
    type Properties = WorkflowBuilderProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            workflow: Workflow {
                name: "New Workflow".to_string(),
                nodes: vec![],
                connections: vec![],
            },
            dragging: None,
            selected: None,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::DragStart(node_type) => {
                self.dragging = Some(node_type);
                false
            }
            Msg::Drop(x, y) => {
                if let Some(node_type) = self.dragging.take() {
                    let id = format!("node_{}", self.workflow.nodes.len());
                    let node = Node {
                        id: id.clone(),
                        node_type: if node_type.starts_with("service:") {
                            NodeType::Service(node_type.strip_prefix("service:").unwrap().to_string())
                        } else {
                            NodeType::Plugin(node_type.strip_prefix("plugin:").unwrap().to_string())
                        },
                        x,
                        y,
                        config: HashMap::new(),
                    };
                    self.workflow.nodes.push(node);
                }
                true
            }
            Msg::SelectNode(id) => {
                self.selected = Some(id);
                true
            }
            Msg::Connect(from, to) => {
                self.workflow.connections.push(Connection { from, to });
                true
            }
            Msg::DeleteNode(id) => {
                self.workflow.nodes.retain(|n| n.id != id);
                self.workflow.connections.retain(|c| c.from != id && c.to != id);
                true
            }
            Msg::Save => {
                let json = serde_json::to_string_pretty(&self.workflow).unwrap();
                web_sys::console::log_1(&json.into());
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        
        html! {
            <div class="workflow-builder">
                <div class="palette">
                    <h3>{"Services"}</h3>
                    { for props.services.iter().map(|s| self.render_palette_item(ctx, format!("service:{}", s), s)) }
                    
                    <h3>{"Plugins"}</h3>
                    { for props.plugins.iter().map(|p| self.render_palette_item(ctx, format!("plugin:{}", p), p)) }
                </div>
                
                <div class="canvas"
                    ondrop={ctx.link().callback(|e: DragEvent| {
                        e.prevent_default();
                        let x = e.offset_x() as f64;
                        let y = e.offset_y() as f64;
                        Msg::Drop(x, y)
                    })}
                    ondragover={Callback::from(|e: DragEvent| e.prevent_default())}
                >
                    <svg width="100%" height="100%">
                        { for self.workflow.connections.iter().map(|c| self.render_connection(c)) }
                    </svg>
                    { for self.workflow.nodes.iter().map(|n| self.render_node(ctx, n)) }
                </div>
                
                <div class="toolbar">
                    <button onclick={ctx.link().callback(|_| Msg::Save)}>{"Save Workflow"}</button>
                </div>
            </div>
        }
    }
}

impl WorkflowBuilder {
    fn render_palette_item(&self, ctx: &Context<Self>, node_type: String, label: &str) -> Html {
        let node_type_clone = node_type.clone();
        html! {
            <div class="palette-item"
                draggable="true"
                ondragstart={ctx.link().callback(move |_| Msg::DragStart(node_type_clone.clone()))}
            >
                {label}
            </div>
        }
    }

    fn render_node(&self, ctx: &Context<Self>, node: &Node) -> Html {
        let id = node.id.clone();
        let selected = self.selected.as_ref() == Some(&node.id);
        
        let label = match &node.node_type {
            NodeType::Service(s) => format!("Service: {}", s),
            NodeType::Plugin(p) => format!("Plugin: {}", p),
            NodeType::Condition => "Condition".to_string(),
            NodeType::Parallel => "Parallel".to_string(),
        };
        
        html! {
            <div class={classes!("node", selected.then(|| "selected"))}
                style={format!("left: {}px; top: {}px", node.x, node.y)}
                onclick={ctx.link().callback(move |_| Msg::SelectNode(id.clone()))}
            >
                <div class="node-header">{label}</div>
                <div class="node-ports">
                    <div class="port input"></div>
                    <div class="port output"></div>
                </div>
            </div>
        }
    }

    fn render_connection(&self, conn: &Connection) -> Html {
        // Find node positions
        let from_node = self.workflow.nodes.iter().find(|n| n.id == conn.from);
        let to_node = self.workflow.nodes.iter().find(|n| n.id == conn.to);
        
        if let (Some(from), Some(to)) = (from_node, to_node) {
            html! {
                <line
                    x1={from.x.to_string()}
                    y1={from.y.to_string()}
                    x2={to.x.to_string()}
                    y2={to.y.to_string()}
                    stroke="black"
                    stroke-width="2"
                />
            }
        } else {
            html! {}
        }
    }
}
