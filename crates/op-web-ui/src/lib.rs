//! OP-DBUS Web UI - Leptos WASM Application

mod components;
mod api;

use leptos::*;
use wasm_bindgen::prelude::wasm_bindgen;
use components::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Dashboard,
    SystemPrompt,
    Tools,
    Workflows,
    McpAdmin,
    Plugins,
    Logs,
}

#[component]
fn App() -> impl IntoView {
    // Navigation state
    let (current_view, set_current_view) = create_signal(View::Dashboard);
    
    // Global state
    let (services, set_services) = create_signal(Vec::<api::SystemService>::new());
    let (dbus_services, set_dbus_services) = create_signal(Vec::<api::DbusService>::new());
    let (diagnostics, set_diagnostics) = create_signal(None::<api::SystemDiagnostics>);
    let (tools, set_tools) = create_signal(Vec::<api::ToolDefinition>::new());
    let (tool_logs, set_tool_logs) = create_signal(Vec::<api::ToolCall>::new());
    let (messages, set_messages) = create_signal(vec![
        api::ChatMessage {
            role: "assistant".to_string(),
            content: "Connected to OP-DBUS. Try 'list tools' or 'system status'.".to_string(),
        }
    ]);
    let (is_processing, set_is_processing) = create_signal(false);
    let (connection_status, set_connection_status) = create_signal("connecting".to_string());

    // Initial data fetch
    spawn_local(async move {
        match api::fetch_all_data().await {
            Ok(data) => {
                set_services.set(data.services);
                set_dbus_services.set(data.dbus_services);
                set_diagnostics.set(Some(data.diagnostics));
                set_tools.set(data.tools);
                set_connection_status.set("connected".to_string());
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Init error: {}", e).into());
                set_connection_status.set("disconnected".to_string());
            }
        }
    });

    // Periodic diagnostics refresh
    let refresh_diagnostics = move || {
        spawn_local(async move {
            if let Ok(diag) = api::fetch_diagnostics().await {
                set_diagnostics.set(Some(diag));
            }
        });
    };
    set_interval(refresh_diagnostics, std::time::Duration::from_secs(10));

    // Chat handler
    let send_message = move |message: String| {
        set_is_processing.set(true);
        set_messages.update(|msgs| msgs.push(api::ChatMessage {
            role: "user".to_string(),
            content: message.clone(),
        }));

        spawn_local(async move {
            match api::send_chat(&message).await {
                Ok(response) => {
                    set_messages.update(|msgs| msgs.push(api::ChatMessage {
                        role: "assistant".to_string(),
                        content: response.message,
                    }));
                    
                    for action in response.actions_taken {
                        set_tool_logs.update(|logs| logs.insert(0, api::ToolCall {
                            id: action.execution_id.unwrap_or_else(uuid),
                            tool_name: action.tool,
                            args: action.params,
                            timestamp: current_time(),
                            status: if action.success { "success" } else { "failure" }.to_string(),
                            result: None,
                        }));
                    }

                    if let Ok(svcs) = api::fetch_services().await {
                        set_services.set(svcs);
                    }
                }
                Err(e) => {
                    set_messages.update(|msgs| msgs.push(api::ChatMessage {
                        role: "assistant".to_string(),
                        content: format!("Error: {}", e),
                    }));
                }
            }
            set_is_processing.set(false);
        });
    };

    view! {
        <div class="flex h-screen">
            // Sidebar Navigation
            <nav class="w-14 bg-zinc-900 border-r border-zinc-800 flex flex-col items-center py-4 gap-2">
                <NavButton 
                    view=View::Dashboard 
                    current=current_view 
                    set_view=set_current_view
                    icon=r#"<rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/>"#
                    label="Dashboard"
                />
                <NavButton 
                    view=View::SystemPrompt 
                    current=current_view 
                    set_view=set_current_view
                    icon=r#"<path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/>"#
                    label="System Prompt"
                />
                <NavButton 
                    view=View::Tools 
                    current=current_view 
                    set_view=set_current_view
                    icon=r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>"#
                    label="Tools"
                />
                <NavButton 
                    view=View::Workflows 
                    current=current_view 
                    set_view=set_current_view
                    icon=r#"<polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>"#
                    label="Workflows"
                />
                <NavButton 
                    view=View::McpAdmin 
                    current=current_view 
                    set_view=set_current_view
                    icon=r#"<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/>"#
                    label="MCP Groups"
                />
                <NavButton 
                    view=View::Plugins 
                    current=current_view 
                    set_view=set_current_view
                    icon=r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33"/>"#
                    label="Plugins"
                />
                <NavButton 
                    view=View::Logs 
                    current=current_view 
                    set_view=set_current_view
                    icon=r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>"#
                    label="Logs"
                />
                
                // Connection indicator at bottom
                <div class="mt-auto">
                    <div class={move || format!("w-3 h-3 rounded-full {}",
                        match connection_status.get().as_str() {
                            "connected" => "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]",
                            "connecting" => "bg-amber-500 animate-pulse",
                            _ => "bg-red-500"
                        }
                    )}></div>
                </div>
            </nav>

            // Main Content
            <div class="flex-1 flex flex-col overflow-hidden">
                {move || match current_view.get() {
                    View::Dashboard => view! {
                        <div class="flex h-full">
                            // Chat Panel
                            <div class="w-1/3 border-r border-zinc-800 bg-zinc-950">
                                <ChatInterface 
                                    messages=messages
                                    is_processing=is_processing
                                    on_send=send_message.clone()
                                />
                            </div>
                            // Right Panel: Monitor + Logs
                            <div class="flex-1 flex flex-col bg-zinc-950">
                                <div class="flex-1 p-4 overflow-y-auto border-b border-zinc-800">
                                    <SystemMonitor 
                                        services=services
                                        dbus_services=dbus_services
                                        diagnostics=diagnostics
                                        tools=tools
                                        connection_status=connection_status
                                    />
                                </div>
                                <div class="h-1/3">
                                    <ToolLog logs=tool_logs />
                                </div>
                            </div>
                        </div>
                    }.into_view(),
                    View::SystemPrompt => view! { <SystemPromptView /> }.into_view(),
                    View::Tools => view! { <ToolsView /> }.into_view(),
                    View::Workflows => view! { <WorkflowsView /> }.into_view(),
                    View::McpAdmin => view! { <McpAdminView /> }.into_view(),
                    View::Plugins => view! { <PluginConfigView /> }.into_view(),
                    View::Logs => view! { <SystemLogView /> }.into_view(),
                }}
            </div>
        </div>
    }
}

#[component]
fn NavButton(
    view: View,
    current: ReadSignal<View>,
    set_view: WriteSignal<View>,
    icon: &'static str,
    label: &'static str,
) -> impl IntoView {
    let is_active = move || current.get() == view;
    
    view! {
        <button
            class={move || format!(
                "w-10 h-10 rounded-lg flex items-center justify-center transition-colors group relative {}",
                if is_active() { 
                    "bg-zinc-800 text-white" 
                } else { 
                    "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50" 
                }
            )}
            on:click=move |_| set_view.set(view)
            title=label
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" inner_html=icon/>
            <span class="absolute left-12 bg-zinc-800 text-white text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 pointer-events-none whitespace-nowrap z-50">
                {label}
            </span>
        </button>
    }
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}", now.as_nanos())
}

fn current_time() -> String {
    js_sys::Date::new_0().to_locale_time_string("en-US").as_string().unwrap_or_default()
}

fn set_interval<F>(f: F, duration: std::time::Duration)
where
    F: Fn() + 'static,
{
    use gloo_timers::future::TimeoutFuture;
    spawn_local(async move {
        loop {
            TimeoutFuture::new(duration.as_millis() as u32).await;
            f();
        }
    });
}
