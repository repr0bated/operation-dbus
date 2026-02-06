use leptos::*;
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
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
}

#[component]
pub fn WorkflowBuilder(
    services: Vec<String>,
    plugins: Vec<String>,
) -> impl IntoView {
    let (nodes, set_nodes) = create_signal(Vec::<Node>::new());
    let (connections, set_connections) = create_signal(Vec::<Connection>::new());
    let (dragging, set_dragging) = create_signal(None::<String>);
    let (selected, set_selected) = create_signal(None::<String>);

    let on_drag_start = move |node_type: String| {
        set_dragging(Some(node_type));
    };

    let on_drop = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        if let Some(node_type) = dragging() {
            let x = ev.offset_x() as f64;
            let y = ev.offset_y() as f64;
            let id = format!("node_{}", nodes().len());
            
            let node = Node {
                id,
                node_type: if node_type.starts_with("service:") {
                    NodeType::Service(node_type.strip_prefix("service:").unwrap().to_string())
                } else {
                    NodeType::Plugin(node_type.strip_prefix("plugin:").unwrap().to_string())
                },
                x,
                y,
                config: HashMap::new(),
            };
            
            set_nodes.update(|n| n.push(node));
            set_dragging(None);
        }
    };

    let save_workflow = move |_| {
        let workflow = serde_json::json!({
            "nodes": nodes(),
            "connections": connections(),
        });
        log!("Workflow: {}", serde_json::to_string_pretty(&workflow).unwrap());
    };

    view! {
        <div class="workflow-builder">
            <div class="palette">
                <h3>"Services"</h3>
                <For
                    each=move || services.clone()
                    key=|s| s.clone()
                    children=move |service| {
                        let node_type = format!("service:{}", service);
                        view! {
                            <div
                                class="palette-item"
                                draggable="true"
                                on:dragstart=move |_| on_drag_start(node_type.clone())
                            >
                                {service}
                            </div>
                        }
                    }
                />
                
                <h3>"Plugins"</h3>
                <For
                    each=move || plugins.clone()
                    key=|p| p.clone()
                    children=move |plugin| {
                        let node_type = format!("plugin:{}", plugin);
                        view! {
                            <div
                                class="palette-item"
                                draggable="true"
                                on:dragstart=move |_| on_drag_start(node_type.clone())
                            >
                                {plugin}
                            </div>
                        }
                    }
                />
            </div>
            
            <div
                class="canvas"
                on:drop=on_drop
                on:dragover=|ev: web_sys::DragEvent| ev.prevent_default()
            >
                <svg width="100%" height="100%">
                    <For
                        each=move || connections()
                        key=|c| format!("{}-{}", c.from, c.to)
                        children=move |conn| {
                            let from_node = nodes().iter().find(|n| n.id == conn.from).cloned();
                            let to_node = nodes().iter().find(|n| n.id == conn.to).cloned();
                            
                            if let (Some(from), Some(to)) = (from_node, to_node) {
                                view! {
                                    <line
                                        x1=from.x
                                        y1=from.y
                                        x2=to.x
                                        y2=to.y
                                        stroke="black"
                                        stroke-width="2"
                                    />
                                }
                            } else {
                                view! { <g/> }
                            }
                        }
                    />
                </svg>
                
                <For
                    each=move || nodes()
                    key=|n| n.id.clone()
                    children=move |node| {
                        let is_selected = selected() == Some(node.id.clone());
                        let node_id = node.id.clone();
                        
                        let label = match &node.node_type {
                            NodeType::Service(s) => format!("Service: {}", s),
                            NodeType::Plugin(p) => format!("Plugin: {}", p),
                        };
                        
                        view! {
                            <div
                                class=move || if is_selected { "node selected" } else { "node" }
                                style=format!("left: {}px; top: {}px", node.x, node.y)
                                on:click=move |_| set_selected(Some(node_id.clone()))
                            >
                                <div class="node-header">{label}</div>
                                <div class="node-ports">
                                    <div class="port input"></div>
                                    <div class="port output"></div>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
            
            <div class="toolbar">
                <button on:click=save_workflow>"Save Workflow"</button>
            </div>
        </div>
    }
}
