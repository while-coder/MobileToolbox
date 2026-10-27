import { createApp, type Plugin } from "vue"
import { createUiKit } from "@qingfeng346/ui-kit"
import "@mobiletoolbox/ui/styles.css"
import App from "./App.vue"

// ui-kit 的全局组件类型增强非常庞大，vue-tsc 在 use() 处会触发插件类型递归比较，
// 这里显式收窄成 Plugin 以绕开纯类型层面的栈溢出（运行行为不变）。
const app = createApp(App)
app.use(createUiKit() as unknown as Plugin)
app.mount("#app")
