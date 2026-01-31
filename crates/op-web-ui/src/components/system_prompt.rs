//! System Prompt View - Immutable + Tunable sections

use leptos::*;

#[component]
pub fn SystemPromptView() -> impl IntoView {
    let (immutable, set_immutable) = create_signal(String::new());
    let (tunable, set_tunable) = create_signal(String::new());
    let (is_loading, set_is_loading) = create_signal(true);
    let (is_saving, set_is_saving) = create_signal(false);
    let (save_status, set_save_status) = create_signal(None::<String>);

    // Load system prompt on mount
    spawn_local(async move {
        match crate::api::fetch_system_prompt().await {
            Ok(config) => {
                set_immutable.set(config.immutable_section);
                set_tunable.set(config.tunable_section);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to load prompt: {}", e).into());
            }
        }
        set_is_loading.set(false);
    });

    let save_tunable = move |_| {
        let tunable_val = tunable.get();
        set_is_saving.set(true);
        set_save_status.set(None);
        
        spawn_local(async move {
            // Simulate saving - in production this would POST to /api/system/prompt
            gloo_timers::future::TimeoutFuture::new(500).await;
            set_save_status.set(Some("✓ Saved".to_string()));
            set_is_saving.set(false);
            
            // Clear status after 2 seconds
            gloo_timers::future::TimeoutFuture::new(2000).await;
            set_save_status.set(None);
        });
    };

    view! {
        <div class="h-full flex flex-col bg-zinc-950">
            // Header
            <div class="px-6 py-4 border-b border-zinc-800 bg-zinc-900/50">
                <h2 class="text-lg font-bold text-white flex items-center gap-3">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-violet-400">
                        <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
                        <polyline points="14 2 14 8 20 8"/>
                        <line x1="16" y1="13" x2="8" y2="13"/>
                        <line x1="16" y1="17" x2="8" y2="17"/>
                        <line x1="10" y1="9" x2="8" y2="9"/>
                    </svg>
                    "System Prompt Configuration"
                </h2>
                <p class="text-xs text-zinc-500 mt-1">
                    "The cognitive contract governing OP-DBUS chatbot behavior"
                </p>
            </div>

            {move || if is_loading.get() {
                view! {
                    <div class="flex-1 flex items-center justify-center">
                        <div class="text-zinc-500">"Loading..."</div>
                    </div>
                }.into_view()
            } else {
                view! {
                    <div class="flex-1 overflow-y-auto p-6 space-y-6">
                        // Immutable Section
                        <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                            <div class="px-4 py-3 border-b border-zinc-800 bg-zinc-950/50 flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    <div class="w-2 h-2 rounded-full bg-red-500"></div>
                                    <h3 class="text-sm font-semibold text-zinc-100">"Immutable Core"</h3>
                                </div>
                                <span class="text-[10px] bg-red-900/30 text-red-400 px-2 py-0.5 rounded border border-red-900/50">
                                    "READ-ONLY"
                                </span>
                            </div>
                            <div class="p-4">
                                <div class="text-xs text-zinc-500 mb-2">
                                    "These invariants cannot be modified at runtime. They define the fundamental contract."
                                </div>
                                <pre class="bg-zinc-950 border border-zinc-800 rounded p-4 text-xs font-mono text-zinc-300 whitespace-pre-wrap overflow-x-auto">
                                    {move || immutable.get()}
                                </pre>
                            </div>
                        </div>

                        // Tunable Section
                        <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                            <div class="px-4 py-3 border-b border-zinc-800 bg-zinc-950/50 flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    <div class="w-2 h-2 rounded-full bg-emerald-500"></div>
                                    <h3 class="text-sm font-semibold text-zinc-100">"Tunable Context"</h3>
                                </div>
                                <span class="text-[10px] bg-emerald-900/30 text-emerald-400 px-2 py-0.5 rounded border border-emerald-900/50">
                                    "EDITABLE"
                                </span>
                            </div>
                            <div class="p-4">
                                <div class="text-xs text-zinc-500 mb-2">
                                    "Runtime context that can be adjusted. Changes take effect on next chat session."
                                </div>
                                <textarea
                                    class="w-full bg-zinc-950 border border-zinc-800 rounded p-4 text-xs font-mono text-zinc-300 resize-none focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500"
                                    rows="10"
                                    prop:value=move || tunable.get()
                                    on:input=move |ev| {
                                        set_tunable.set(event_target_value(&ev));
                                    }
                                />
                                <div class="mt-3 flex items-center justify-between">
                                    <div class="text-xs text-zinc-600">
                                        "Supports markdown formatting"
                                    </div>
                                    <div class="flex items-center gap-3">
                                        {move || save_status.get().map(|s| view! {
                                            <span class="text-xs text-emerald-400">{s}</span>
                                        })}
                                        <button
                                            class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium rounded transition-colors disabled:opacity-50"
                                            disabled=move || is_saving.get()
                                            on:click=save_tunable
                                        >
                                            {move || if is_saving.get() { "Saving..." } else { "Save Changes" }}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>

                        // Preview Section
                        <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                            <div class="px-4 py-3 border-b border-zinc-800 bg-zinc-950/50">
                                <h3 class="text-sm font-semibold text-zinc-100">"Combined Prompt Preview"</h3>
                            </div>
                            <div class="p-4">
                                <div class="bg-zinc-950 border border-zinc-800 rounded p-4 text-xs font-mono text-zinc-400 whitespace-pre-wrap max-h-48 overflow-y-auto">
                                    {move || format!("{}\n\n---\n\n{}", immutable.get(), tunable.get())}
                                </div>
                            </div>
                        </div>
                    </div>
                }.into_view()
            }}
        </div>
    }
}
