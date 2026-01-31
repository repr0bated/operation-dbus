//! System Monitor Component

use leptos::*;
use crate::api::{SystemService, DbusService, SystemDiagnostics, ToolDefinition};

#[component]
pub fn SystemMonitor(
    services: ReadSignal<Vec<SystemService>>,
    dbus_services: ReadSignal<Vec<DbusService>>,
    diagnostics: ReadSignal<Option<SystemDiagnostics>>,
    tools: ReadSignal<Vec<ToolDefinition>>,
    connection_status: ReadSignal<String>,
) -> impl IntoView {
    let active_count = move || services.get().iter().filter(|s| s.status == "active").count();
    let error_count = move || services.get().iter().filter(|s| s.status == "error").count();
    
    let services_display = move || services.get().into_iter().take(15).collect::<Vec<_>>();
    let opdbus_display = move || dbus_services.get().into_iter().filter(|s| s.category == "op-dbus").collect::<Vec<_>>();
    let other_dbus_display = move || dbus_services.get().into_iter().filter(|s| s.category != "op-dbus").take(10).collect::<Vec<_>>();
    let tools_display = move || tools.get();

    view! {
        <div class="space-y-4">
            // Diagnostics Header
            {move || diagnostics.get().map(|diag| view! {
                <div class="bg-gradient-to-r from-zinc-900 to-zinc-950 border border-zinc-800 rounded-lg p-4">
                    <div class="flex items-center justify-between mb-3">
                        <div class="flex items-center gap-3">
                            <div class={move || format!("w-3 h-3 rounded-full {}",
                                if connection_status.get() == "connected" {
                                    "bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]"
                                } else {
                                    "bg-red-500 animate-pulse"
                                }
                            )}></div>
                            <h2 class="text-lg font-bold text-white">{diag.hostname.clone()}</h2>
                        </div>
                        <div class="text-xs font-mono text-zinc-500">"Uptime: "{diag.uptime.formatted.clone()}</div>
                    </div>
                    
                    <div class="grid grid-cols-4 gap-3">
                        <div class="bg-zinc-950/50 rounded p-2 border border-zinc-800/50">
                            <div class="text-[10px] text-zinc-500 uppercase">"Load"</div>
                            <div class="font-mono text-xs text-zinc-200">
                                {format!("{:.2} / {:.2} / {:.2}", diag.load.one_min, diag.load.five_min, diag.load.fifteen_min)}
                            </div>
                        </div>
                        <div class="bg-zinc-950/50 rounded p-2 border border-zinc-800/50">
                            <div class="text-[10px] text-zinc-500 uppercase">"Memory"</div>
                            <div class="font-mono text-xs text-zinc-200">{diag.memory.formatted.clone()}</div>
                            <div class="mt-1 h-1 bg-zinc-800 rounded overflow-hidden">
                                <div class="h-full bg-emerald-500" style={format!("width: {}%", diag.memory.percent_used)}></div>
                            </div>
                        </div>
                        <div class="bg-zinc-950/50 rounded p-2 border border-zinc-800/50">
                            <div class="text-[10px] text-zinc-500 uppercase">"CPU Cores"</div>
                            <div class="font-mono text-xl text-zinc-200">{diag.cpu_cores}</div>
                        </div>
                        <div class="bg-zinc-950/50 rounded p-2 border border-zinc-800/50">
                            <div class="text-[10px] text-zinc-500 uppercase">"Services"</div>
                            <div class="flex items-center gap-1">
                                <span class="font-mono text-emerald-400">{active_count()}</span>
                                <span class="text-zinc-600">"/"</span>
                                <span class="font-mono text-zinc-400">{move || services.get().len()}</span>
                                {move || (error_count() > 0).then(|| view! {
                                    <span class="ml-2 text-[10px] bg-red-900/30 text-red-400 px-1 rounded">{error_count()}" failed"</span>
                                })}
                            </div>
                        </div>
                    </div>
                </div>
            })}

            <div class="grid grid-cols-2 gap-4">
                // Services Panel
                <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                    <div class="px-3 py-2 border-b border-zinc-800 bg-zinc-950/50 flex justify-between items-center">
                        <h3 class="text-xs font-semibold text-zinc-100">"Systemd Services"</h3>
                        <span class="text-[10px] font-mono text-zinc-500">{move || services.get().len()}" units"</span>
                    </div>
                    <div class="p-2 space-y-1 max-h-40 overflow-y-auto">
                        <For
                            each=services_display
                            key=|s| s.id.clone()
                            let:svc
                        >
                            {
                                let status_color = match svc.status.as_str() {
                                    "active" => "bg-emerald-500",
                                    "error" => "bg-red-500",
                                    _ => "bg-zinc-600",
                                };
                                view! {
                                    <div class="flex items-center justify-between p-1.5 rounded bg-zinc-950/50 border border-zinc-800/50 hover:border-zinc-700">
                                        <div class="flex items-center gap-2">
                                            <div class={format!("w-1.5 h-1.5 rounded-full {}", status_color)}></div>
                                            <span class="text-[10px] text-zinc-300 truncate max-w-[120px]">{svc.name.clone()}</span>
                                        </div>
                                        <span class="text-[9px] text-zinc-600">{svc.sub_state.clone().unwrap_or_default()}</span>
                                    </div>
                                }
                            }
                        </For>
                    </div>
                </div>

                // D-Bus Panel
                <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                    <div class="px-3 py-2 border-b border-zinc-800 bg-zinc-950/50 flex justify-between items-center">
                        <h3 class="text-xs font-semibold text-zinc-100">"D-Bus Services"</h3>
                        <span class="text-[10px] font-mono text-zinc-500">{move || dbus_services.get().len()}" names"</span>
                    </div>
                    <div class="p-2 space-y-1 max-h-40 overflow-y-auto">
                        // OP-DBUS services first
                        <For
                            each=opdbus_display
                            key=|s| s.name.clone()
                            let:svc
                        >
                            <div class="flex items-center justify-between p-1.5 rounded bg-emerald-950/20 border border-emerald-900/30">
                                <div class="flex items-center gap-2">
                                    <div class="w-1.5 h-1.5 rounded-full bg-emerald-500"></div>
                                    <span class="text-[10px] font-mono text-emerald-300 truncate max-w-[150px]">{svc.name.clone()}</span>
                                </div>
                                <span class="text-[9px] text-emerald-600">"OP-DBUS"</span>
                            </div>
                        </For>
                        <For
                            each=other_dbus_display
                            key=|s| s.name.clone()
                            let:svc
                        >
                            <div class="flex items-center justify-between p-1.5 rounded bg-zinc-950/50 border border-zinc-800/50">
                                <span class="text-[10px] font-mono text-zinc-400 truncate max-w-[150px]">{svc.name.clone()}</span>
                                <span class="text-[9px] text-zinc-600">{svc.category.clone()}</span>
                            </div>
                        </For>
                    </div>
                </div>
            </div>

            // Tools
            <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                <div class="px-3 py-2 border-b border-zinc-800 bg-zinc-950/50 flex justify-between items-center">
                    <h3 class="text-xs font-semibold text-zinc-100">"Available Tools"</h3>
                    <span class="text-[10px] font-mono text-zinc-500">{move || tools.get().len()}" registered"</span>
                </div>
                <div class="p-2 flex flex-wrap gap-1 max-h-24 overflow-y-auto">
                    <For
                        each=tools_display
                        key=|t| t.name.clone()
                        let:tool
                    >
                        <span class="text-[10px] font-mono text-cyan-400 bg-cyan-950/30 px-1.5 py-0.5 rounded border border-cyan-900/30">
                            {tool.name.clone()}
                        </span>
                    </For>
                </div>
            </div>
        </div>
    }
}
