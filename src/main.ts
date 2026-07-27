import { mount } from "svelte";
import "@fontsource/inter/400.css";
import "@fontsource/inter/700.css";
import "@fontsource/inter/900.css";

import App from "./App.svelte";
import "./styles.css";

const target = document.getElementById("app");

if (!target) {
  throw new Error("Application mount point is missing");
}

mount(App, { target });
