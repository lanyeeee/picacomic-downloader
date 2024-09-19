<script setup lang="ts">
import {ref} from "vue";
import {commands} from "../bindings.ts";
import {useMessage, useNotification} from "naive-ui";

const message = useMessage();
const notification = useNotification();

const showing = defineModel<boolean>("showing", {required: true});
const token = defineModel<string>("token", {required: true});

const email = ref<string>();
const password = ref<string>();

async function onLogin() {
  if (email.value === undefined) {
    message.error("请输入邮箱");
    return;
  }
  if (password.value === undefined) {
    message.error("请输入密码");
    return;
  }
  const result = await commands.login(email.value, password.value);
  console.log("command result:", result);
  if (result.status === "error") {
    console.error(result.error);
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
            @positive-click="onLogin"
            @close="showing=false">
    <n-input v-model:value="email" placeholder="">
      <template #prefix>
        邮箱:
      </template>
    </n-input>
    <n-input v-model:value="password" type="password" placeholder="" show-password-on="mousedown">
      <template #prefix>
        密码:
      </template>
    </n-input>
  </n-dialog>
</template>
