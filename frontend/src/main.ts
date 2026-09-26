import { mount } from "svelte";
import "@fontsource-variable/manrope";
import "./styles.css";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("Missing application root");
mount(App, { target });
