//! System Log View - Stream logs with fallback to external source

use leptos::*;

#[component]
pub fn SystemLogView() -> impl IntoView {
    let (logs, set_logs) = create_signal(Vec::<LogEntry>::new());
    let (use_external, set_use_external) = create_signal(false);
    let (is_connected, set_is_connected) = create_signal(false);

    // Try local first, fallback to external
    spawn_local(async move {
        // Try local endpoint
        match gloo_net::http::Request::get("/api/logs/stream").send().await {
            Ok(resp) if resp.ok() => {
                set_is_connected.set(true);
                // Would set up SSE stream here
            }
            _ => {
                // Fallback to external
                set_use_external.set(true);
                set_is_connected.set(true);
            }
        }
    });

    view! {
        <div class="h-full flex flex-col bg-zinc-950">
            <div class="px-6 py-4 border-b border-zinc-800 bg-zinc-900/50">
                <div class="flex items-center justify-between">
                    <h2 class="text-lg font-bold text-white flex items-center gap-3">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-green-400">
                            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                            <polyline points="14 2 14 8 20 8"/>
                            <line x1="16" y1="13" x2="8" y2="13"/>
                            <line x1="16" y1="17" x2="8" y2="17"/>
                            <polyline points="10 9 9 9 8 9"/>
                        </svg>
                        "System Logs"
                    </h2>
                    <div class="flex items-center gap-3">
                        <div class={move || format!("flex items-center gap-2 text-xs {}",
                            if is_connected.get() { "text-emerald-400" } else { "text-zinc-500" }
                        )}>
                            <div class={move || format!("w-2 h-2 rounded-full {}",
                                if is_connected.get() { "bg-emerald-500" } else { "bg-zinc-600" }
                            )}></div>
                            {move || if use_external.get() { "logs.ghostbridge.tech" } else { "local" }}
                        </div>
                        <button 
                            class="text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
                            on:click=move |_| set_use_external.update(|v| *v = !*v)
                        >
                            "Toggle Source"
                        </button>
                    </div>
                </div>
            </div>

            {move || if use_external.get() {
                view! {
                    <iframe 
                        src="https://logs.ghostbridge.tech" 
                        class="flex-1 w-full border-0"
                        style="background: #09090b;"
                    />
                }.into_view()
            } else {
                view! {
                    <div class="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-1 bg-black">
                        {move || if logs.get().is_empty() {
                            view! {
                                <div class="flex items-center justify-center h-full text-zinc-600">
                                    "Waiting for log stream..."
                                </div>
                            }.into_view()
                        } else {
                            view! {
                                <For
                                    each=move || logs.get()
                                    key=|l| l.id.clone()
                                    children=move |log| {
                                        let level_color = match log.level.as_str() {
                                            "ERROR" => "text-red-400",
                                            "WARN" => "text-amber-400",
                                            "INFO" => "text-blue-400",
                                            "DEBUG" => "text-zinc-500",
                                            _ => "text-zinc-400",
                                        };
                                        view! {
                                            <div class="flex gap-3 hover:bg-zinc-900/50 px-2 py-0.5">
                                                <span class="text-zinc-600">{log.timestamp.clone()}</span>
                                                <span class={format!("w-12 {}", level_color)}>{log.level.clone()}</span>
                                                <span class="text-cyan-400">{log.target.clone()}</span>
                                                <span class="text-zinc-300 flex-1">{log.message.clone()}</span>
                                            </div>
                                        }
                                    }
                                />
                            }.into_view()
                        }}
                    </div>
                }.into_view()
            }}
        </div>
    }
}

#[derive(Clone, Debug)]
struct LogEntry {
    id: String,
    timestamp: String,
    level: String,
    target: String,
    message: String,
}
