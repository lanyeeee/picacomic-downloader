<script setup lang="ts">
import {onMounted, ref, watch} from "vue";
import {commands, Config, Episode, UserProfile} from "./bindings.ts";
import {useMessage, useNotification} from "naive-ui";
import LoginDialog from "./components/LoginDialog.vue";
import SearchPane from "./components/SearchPane.vue";
import EpisodePane from "./components/EpisodePane.vue";
import DownloadingList from "./components/DownloadingList.vue";

const message = useMessage();
const notification = useNotification();

const config = ref<Config>();
const loginDialogShowing = ref<boolean>(false);
const userProfile = ref<UserProfile>();
const episodes = ref<Episode[]>();
const currentTabName = ref<"search" | "episode">("search");
const comicId = ref<string>();

watch(config, async () => {
  if (config.value === undefined) {
    return;
  }
  await commands.saveConfig(config.value);
  message.success("保存配置成功");
}, {deep: true});
watch(() => config.value?.token, async () => {
  const result = await commands.getUserProfile();
  if (result.status === "error") {
    notification.error({title: "获取用户信息失败", description: result.error});
    userProfile.value = undefined;
    return;
  }
  userProfile.value = result.data;
  message.success("获取用户信息成功");
});

onMounted(async () => {
  // 屏蔽浏览器右键菜单
  document.oncontextmenu = (event) => {
    event.preventDefault();
  };
  // 获取配置
  config.value = await commands.getConfig();
});

async function test() {
  console.log(userProfile.value);
}

</script>

<template>
  <div v-if="config!==undefined" class="h-screen flex flex-col overflow-auto">
    <div class="flex">
      <n-input v-model:value="config.token" placeholder="" clearable>
        <template #prefix>
          Authorization：
        </template>
      </n-input>
      <n-button type="primary" @click="loginDialogShowing=true">账号登录</n-button>
      <n-button @click="test">测试用</n-button>
    </div>
    <div class="flex overflow-hidden">
      <n-tabs class="basis-1/2 overflow-auto" v-model:value="currentTabName" type="line" size="small">
        <n-tab-pane class="h-full overflow-auto p-0!" name="search" tab="漫画搜索" display-directive="show:lazy">
          <search-pane v-model:episodes="episodes" v-model:current-tab-name="currentTabName"/>
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="episode" tab="章节详情" display-directive="show:lazy">
          <episode-pane v-model:comic-id="comicId" v-model:episodes="episodes"/>
        </n-tab-pane>
      </n-tabs>

      <div class="basis-1/2 overflow-auto">
        <div class="flex flex-justify-end" v-if="userProfile!==undefined">
          <n-avatar v-if="userProfile.avatar!==undefined"
                    round
                    :size="50"
                    :src="`${userProfile.avatar.fileServer}/static/${userProfile.avatar.path}`"
                    fallback-src="https://storage-b.picacomic.com/static/b3411e38-32f2-4ec4-a46c-2edee925dbbd.jpg"/>
          <div class="flex flex-col">
            <span>{{ userProfile.name }}</span>
            <span>Lv.{{ userProfile.level }} {{ userProfile.title }}</span>
          </div>
        </div>
        <downloading-list></downloading-list>
      </div>
    </div>
    <n-modal v-model:show="loginDialogShowing">
      <login-dialog v-model:showing="loginDialogShowing" v-model:token="config.token"/>
    </n-modal>
  </div>
</template>
