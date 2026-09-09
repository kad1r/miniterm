import { mount } from "svelte";
import "./fonts.css";
import "./styles.css";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
