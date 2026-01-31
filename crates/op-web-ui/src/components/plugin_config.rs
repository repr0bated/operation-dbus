//! Plugin Config View - Placeholder

use leptos::*;

#[component]
pub fn PluginConfigView() -> impl IntoView {
    let (plugins, set_plugins) = create_signal(Vec::<crate::api::PluginConfig>::new());

    spawn_local(async move {
        if let Ok(p) = crate::api::fetch_plugins().await {
            set_plugins.set(p);
        }
    });

    view! {
        <div class="h-full flex flex-col bg-zinc-950">
            <div class="px-6 py-4 border-b border-zinc-800 bg-zinc-900/50">
                <h2 class="text-lg font-bold text-white flex items-center gap-3">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-orange-400">
                        <circle cx="12" cy="12" r="3"/>
                        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
                    </svg>
                    "Plugin Configuration"
                </h2>
            </div>

            <div class="flex-1 overflow-y-auto p-6 space-y-4">
                <For
                    each=move || plugins.get()
                    key=|p| p.name.clone()
                    children=move |plugin| view! {
                        <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
                            <div class="flex items-center justify-between mb-3">
                                <div>
                                    <h3 class="text-sm font-semibold text-zinc-100">{plugin.name.clone()}</h3>
                                    <span class="text-[10px] text-zinc-500">"v"{plugin.version.clone()}</span>
                                </div>
                                <span class={format!("text-[10px] px-2 py-0.5 rounded {}",
                                    if plugin.enabled { "bg-emerald-900/30 text-emerald-400" } else { "bg-zinc-800 text-zinc-500" }
                                )}>
                                    {if plugin.enabled { "Enabled" } else { "Disabled" }}
                                </span>
                            </div>
                            <div class="bg-zinc-950 border border-zinc-800 rounded p-3">
                                <div class="text-[10px] text-zinc-500 uppercase mb-1">"Tunables"</div>
                                <pre class="text-xs font-mono text-zinc-400">
                                    {simd_json::to_string_pretty(&plugin.tunables).unwrap_or_default()}
                                </pre>
                            </div>
                        </div>
                    }
                />
            </div>
        </div>
    }
}
