//! MCP Groups Admin View - Placeholder

use leptos::*;

#[component]
pub fn McpAdminView() -> impl IntoView {
    let (groups, set_groups) = create_signal(Vec::<crate::api::McpGroup>::new());

    spawn_local(async move {
        if let Ok(g) = crate::api::fetch_mcp_groups().await {
            set_groups.set(g);
        }
    });

    view! {
        <div class="h-full flex flex-col bg-zinc-950">
            <div class="px-6 py-4 border-b border-zinc-800 bg-zinc-900/50">
                <h2 class="text-lg font-bold text-white flex items-center gap-3">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-purple-400">
                        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
                        <circle cx="9" cy="7" r="4"/>
                        <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
                        <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
                    </svg>
                    "MCP Agent Groups"
                </h2>
            </div>

            <div class="flex-1 overflow-y-auto p-6 space-y-4">
                <For
                    each=move || groups.get()
                    key=|g| g.id.clone()
                    children=move |group| view! {
                        <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
                            <div class="flex items-center justify-between mb-3">
                                <h3 class="text-sm font-semibold text-zinc-100">{group.name.clone()}</h3>
                                <span class={format!("text-[10px] px-2 py-0.5 rounded {}",
                                    if group.active { "bg-emerald-900/30 text-emerald-400" } else { "bg-zinc-800 text-zinc-500" }
                                )}>
                                    {if group.active { "Active" } else { "Inactive" }}
                                </span>
                            </div>
                            <div class="flex flex-wrap gap-2">
                                {group.agents.iter().map(|a| view! {
                                    <span class="text-xs font-mono bg-purple-950/30 text-purple-300 px-2 py-1 rounded border border-purple-900/30">
                                        {a.clone()}
                                    </span>
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }
                />
            </div>
        </div>
    }
}
