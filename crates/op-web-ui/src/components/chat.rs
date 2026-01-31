//! Chat Interface Component

use leptos::*;
use crate::api::ChatMessage;

#[component]
pub fn ChatInterface(
    messages: ReadSignal<Vec<ChatMessage>>,
    is_processing: ReadSignal<bool>,
    on_send: impl Fn(String) + 'static + Clone,
) -> impl IntoView {
    let (input, set_input) = create_signal(String::new());
    let input_ref = create_node_ref::<html::Input>();
    
    let on_send_click = on_send.clone();
    let on_send_key = on_send.clone();

    let do_send = move || {
        let msg = input.get();
        if !msg.trim().is_empty() && !is_processing.get() {
            on_send_click(msg);
            set_input.set(String::new());
        }
    };

    let messages_display = move || messages.get();

    view! {
        <div class="flex flex-col h-full">
            // Chat History
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                <For
                    each=messages_display
                    key=|msg| msg.content.clone()
                    let:msg
                >
                    {
                        let is_user = msg.role == "user";
                        view! {
                            <div class={if is_user { "flex justify-end" } else { "flex justify-start" }}>
                                <div class={format!("max-w-[85%] rounded-lg p-3 text-sm leading-relaxed {}",
                                    if is_user {
                                        "bg-emerald-600 text-white rounded-br-none"
                                    } else {
                                        "bg-zinc-800 text-zinc-200 rounded-bl-none border border-zinc-700"
                                    }
                                )}>
                                    <div class="flex items-center gap-2 mb-1 opacity-50 text-xs font-mono uppercase">
                                        <span class={if is_user { "text-emerald-200" } else { "text-zinc-400" }}>
                                            {if is_user { "Operator" } else { "Agent" }}
                                        </span>
                                    </div>
                                    <div class="whitespace-pre-wrap">{msg.content.clone()}</div>
                                </div>
                            </div>
                        }
                    }
                </For>
                
                {move || is_processing.get().then(|| view! {
                    <div class="flex justify-start">
                        <div class="bg-zinc-800 border border-zinc-700 rounded-lg p-3 rounded-bl-none flex items-center gap-2">
                            <div class="w-2 h-2 bg-emerald-500 rounded-full animate-bounce"></div>
                            <div class="w-2 h-2 bg-emerald-500 rounded-full animate-bounce" style="animation-delay: 75ms"></div>
                            <div class="w-2 h-2 bg-emerald-500 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                            <span class="text-xs text-zinc-500 ml-2 font-mono">"Processing..."</span>
                        </div>
                    </div>
                })}
            </div>

            // Input Area
            <div class="p-4 border-t border-zinc-800 bg-zinc-900">
                <div class="relative">
                    <input
                        type="text"
                        node_ref=input_ref
                        prop:value=move || input.get()
                        on:input=move |ev| set_input.set(event_target_value(&ev))
                        on:keypress=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                let msg = input.get();
                                if !msg.trim().is_empty() && !is_processing.get() {
                                    on_send_key(msg);
                                    set_input.set(String::new());
                                }
                            }
                        }
                        disabled=move || is_processing.get()
                        placeholder="Type a command (e.g., 'list tools')..."
                        class="w-full bg-zinc-950 border border-zinc-700 text-white rounded-md pl-4 pr-12 py-3 text-sm focus:outline-none focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 disabled:opacity-50 font-mono transition-all"
                    />
                    <button
                        on:click=move |_| do_send()
                        disabled=move || input.get().trim().is_empty() || is_processing.get()
                        class="absolute right-2 top-2 p-1.5 text-zinc-400 hover:text-emerald-500 disabled:opacity-30 transition-colors"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="22" y1="2" x2="11" y2="13"/>
                            <polygon points="22 2 15 22 11 13 2 9 22 2"/>
                        </svg>
                    </button>
                </div>
                <div class="mt-2 text-[10px] text-zinc-500 font-mono text-center">
                    "MCP Enabled • Rust/WASM UI"
                </div>
            </div>
        </div>
    }
}
