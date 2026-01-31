//! Workflows View - Placeholder

use leptos::*;

#[component]
pub fn WorkflowsView() -> impl IntoView {
    view! {
        <div class="h-full flex flex-col bg-zinc-950">
            <div class="px-6 py-4 border-b border-zinc-800 bg-zinc-900/50">
                <h2 class="text-lg font-bold text-white flex items-center gap-3">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-amber-400">
                        <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
                    </svg>
                    "Workflows"
                </h2>
            </div>

            <div class="flex-1 flex items-center justify-center">
                <div class="text-center">
                    <div class="text-6xl mb-4">"🔧"</div>
                    <h3 class="text-xl text-zinc-400">"Workflow Builder"</h3>
                    <p class="text-sm text-zinc-600 mt-2">"Coming soon - compose multi-step tool workflows"</p>
                </div>
            </div>
        </div>
    }
}
