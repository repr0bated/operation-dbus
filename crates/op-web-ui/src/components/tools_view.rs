//! Tools View - Browse and execute tools

use leptos::*;

#[component]
pub fn ToolsView() -> impl IntoView {
    let (tools, set_tools) = create_signal(Vec::<crate::api::ToolDefinition>::new());
    let (selected, set_selected) = create_signal(None::<String>);

    spawn_local(async move {
        if let Ok(t) = crate::api::fetch_tools().await {
            set_tools.set(t);
        }
    });

    view! {
        <div class="h-full flex flex-col bg-zinc-950">
            <div class="px-6 py-4 border-b border-zinc-800 bg-zinc-900/50">
                <h2 class="text-lg font-bold text-white flex items-center gap-3">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400">
                        <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>
                    </svg>
                    "Tool Registry"
                </h2>
            </div>

            <div class="flex-1 overflow-y-auto p-6">
                <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
                    <For
                        each=move || tools.get()
                        key=|t| t.name.clone()
                        children=move |tool| {
                            let name = tool.name.clone();
                            let name2 = tool.name.clone();
                            view! {
                                <div 
                                    class="p-3 rounded-lg bg-zinc-900 border border-zinc-800 hover:border-cyan-700 cursor-pointer transition-colors"
                                    on:click=move |_| set_selected.set(Some(name.clone()))
                                >
                                    <div class="text-sm font-mono text-cyan-400">{name2}</div>
                                    <div class="text-xs text-zinc-500 mt-1 truncate">{tool.description.clone()}</div>
                                    <div class="text-[10px] text-zinc-600 mt-2">{tool.plugin.clone()}</div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}
