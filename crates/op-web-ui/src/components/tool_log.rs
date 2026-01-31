//! Tool Log Component

use leptos::*;
use crate::api::ToolCall;

#[component]
pub fn ToolLog(logs: ReadSignal<Vec<ToolCall>>) -> impl IntoView {
    view! {
        <div class="flex flex-col h-full bg-black/20">
            <div class="px-4 py-2 border-b border-zinc-800 flex justify-between items-center bg-zinc-900/50">
                <h3 class="text-xs font-bold text-zinc-400 uppercase tracking-wider flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-emerald-500">
                        <polyline points="4 17 10 11 4 5"/>
                        <line x1="12" y1="19" x2="20" y2="19"/>
                    </svg>
                    "MCP Execution Stream"
                </h3>
                <span class="text-[10px] font-mono text-zinc-600">"JSON-RPC 2.0"</span>
            </div>
            
            <div class="flex-1 overflow-y-auto p-3 font-mono text-xs space-y-1">
                {move || if logs.get().is_empty() {
                    view! {
                        <div class="h-full flex items-center justify-center text-zinc-700 italic">
                            "Waiting for tool calls..."
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <For
                            each=move || logs.get()
                            key=|log| log.id.clone()
                            children=move |log| {
                                let border_color = match log.status.as_str() {
                                    "success" => "border-emerald-500",
                                    "pending" => "border-amber-500",
                                    "failure" => "border-red-500",
                                    _ => "border-zinc-700",
                                };
                                let badge_class = match log.status.as_str() {
                                    "success" => "bg-emerald-950 text-emerald-400 border-emerald-900",
                                    "pending" => "bg-amber-950 text-amber-400 border-amber-900 animate-pulse",
                                    "failure" => "bg-red-950 text-red-400 border-red-900",
                                    _ => "bg-zinc-800 text-zinc-400 border-zinc-700",
                                };
                                view! {
                                    <div class={format!("border-l-2 pl-3 py-2 hover:bg-zinc-900/50 {}", border_color)}>
                                        <div class="flex items-center gap-3 mb-1">
                                            <span class="text-zinc-500">"["{log.timestamp.clone()}"]"</span>
                                            <span class="font-bold text-emerald-400">{log.tool_name.clone()}</span>
                                            <span class={format!("ml-auto text-[10px] uppercase px-1.5 rounded border {}", badge_class)}>
                                                {log.status.clone()}
                                            </span>
                                        </div>
                                        <div class="text-zinc-400 pl-1">
                                            <span class="text-zinc-600 select-none">"$ "</span>
                                            <span class="text-indigo-300">"params: "</span>
                                            {log.args.to_string()}
                                        </div>
                                        {log.result.clone().map(|r| view! {
                                            <div class="text-zinc-400 pl-1 mt-1">
                                                <span class="text-zinc-600 select-none">"> "</span>
                                                <span class="text-emerald-300">"return: "</span>
                                                {r.to_string()}
                                            </div>
                                        })}
                                    </div>
                                }
                            }
                        />
                    }.into_view()
                }}
            </div>
        </div>
    }
}
