<script setup lang="ts">
import {onMounted, ref, watch} from "vue";
import {commands, Config} from "./bindings.ts";
import {useMessage} from "naive-ui";
import LoginDialog from "./components/LoginDialog.vue";

const message = useMessage();

const config = ref<Config>();
const loginDialogShowing = ref<boolean>(false);

watch(config, async () => {
  if (config.value === undefined) {
    return;
  }
  await commands.saveConfig(config.value);
  message.success("保存配置成功");
}, {deep: true});

onMounted(async () => {
  config.value = await commands.getConfig();
});

async function test() {
  console.log("test");

}

</script>

<template>
  <div v-if="config!==undefined" class="h-full flex flex-col">
    <div class="flex">
      <n-input v-model:value="config.token" placeholder="" clearable>
        <template #prefix>
          Authorization：
        </template>
      </n-input>
      <n-button type="primary" @click="loginDialogShowing=true">账号登录</n-button>
      <n-button @click="test">测试用</n-button>
    </div>

    <n-modal v-model:show="loginDialogShowing">
      <login-dialog v-model:showing="loginDialogShowing" v-model:token="config.token"/>
    </n-modal>
  </div>
</template>
