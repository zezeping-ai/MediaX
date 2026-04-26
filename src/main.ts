import { createApp } from "vue";
import { createPinia } from "pinia";
import {
  Alert,
  Button,
  Card,
  ConfigProvider,
  Dropdown,
  Empty,
  Input,
  Layout,
  List,
  Menu,
  Modal,
  Radio,
  Segmented,
  Slider,
  Space,
  Spin,
  Switch,
  Typography,
} from "ant-design-vue";
import "ant-design-vue/dist/reset.css";
import App from "@/App.vue";
import { router } from "@/router";
import "@/styles/tailwind.css";
import "@/styles/global.scss";

const app = createApp(App);

app.use(createPinia());
app.use(router);

[
  Alert,
  Button,
  Card,
  ConfigProvider,
  Dropdown,
  Empty,
  Input,
  Layout,
  List,
  Menu,
  Modal,
  Radio,
  Segmented,
  Slider,
  Space,
  Spin,
  Switch,
  Typography,
  Typography.Text,
  Typography.Title,
  List.Item,
  List.Item.Meta,
  Layout.Content,
  Layout.Sider,
  Menu.Divider,
  Menu.Item,
  Menu.ItemGroup,
  Menu.SubMenu,
  Radio.Group,
].forEach((component) => {
  if (component?.name) {
    app.component(component.name, component);
  }
});

app.mount("#app");
