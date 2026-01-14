import { createApp } from "vue";
import App from "./App.vue";

import { defineCustomElements } from "@revolist/revogrid/loader";

import ElementPlus from "element-plus";
import "element-plus/dist/index.css";
import "./comm/styles/comm-theme.css";
import { createPinia } from "pinia";
import { router } from "./router";

defineCustomElements(window);

createApp(App).use(createPinia()).use(router).use(ElementPlus).mount("#app");
