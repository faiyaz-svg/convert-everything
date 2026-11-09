import { mount } from "svelte";
import "./styles/site.css";
import App from "./App.svelte";
import * as state from "./lib/state.svelte";

// In development, expose the app's state and actions so a script can drive the
// real window over the WebView2 debugging port.
if (import.meta.env.DEV) (window as unknown as { __ce: typeof state }).__ce = state;

export default mount(App, { target: document.getElementById("app")! });
