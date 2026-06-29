// Ambient globals shared by the webview gesture controllers. `dioxus` is the
// bridge object injected by document::eval. The `__...__` placeholders are bare
// identifiers (declared, never defined) so the bun build cannot constant-fold
// them; the Rust hooks replace each with a literal before eval.
declare const dioxus: { send(msg: string): void };
declare const __EDGE_PX__: number;
declare const __OPEN_AT__: number;
declare const __CLOSE_AT__: number;
declare const __THRESHOLD__: number;
declare const __GRAB_PX__: number;
declare const __DISMISS_AT__: number;
