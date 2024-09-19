<script setup lang="ts">
import {ref} from "vue";
import {commands} from "../bindings.ts";
import {useMessage, useNotification} from "naive-ui";

const message = useMessage();
const notification = useNotification();

const showing = defineModel<boolean>("showing", {required: true});
const token = defineModel<string>("token", {required: true});

const emailInput = ref<string>("");
const passwordInput = ref<string>("");

async function onLogin(email: string, password: string) {
  if (email === "") {
    message.error("请输入用户名");
    return;
  }
  if (password === "") {
    message.error("请输入密码");
    return;
  }
  const result = await commands.login(email, password);
  if (result.status === "error") {
    notification.error({title: "登录失败", description: result.error});
    return;
  }
  message.success("登录成功");
  token.value = result.data;
  showing.value = false;
}
</script>

<template>
  <n-dialog class="flex flex-col"
            :showIcon="false"
            title="账号登录"
            positive-text="登录"
            @positive-click="onLogin(emailInput, passwordInput)"
            @close="showing=false">
    <n-input v-model:value="emailInput" placeholder="">
      <template #prefix>
        用户名:
      </template>
    </n-input>
    <n-input v-model:value="passwordInput" type="password" placeholder="" show-password-on="mousedown">
      <template #prefix>
        密码:
      </template>
    </n-input>
  </n-dialog>
</template>
