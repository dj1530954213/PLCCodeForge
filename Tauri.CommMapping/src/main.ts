import { createApp } from "vue";
import App from "./App.vue";

import { defineCustomElements } from "@revolist/revogrid/loader";

import "./comm/styles/comm-theme.css";
import "element-plus/es/components/message/style/css";
import "element-plus/es/components/message-box/style/css";
import { createPinia } from "pinia";
import { router } from "./router";

defineCustomElements(window);

createApp(App).use(createPinia()).use(router).mount("#app");
