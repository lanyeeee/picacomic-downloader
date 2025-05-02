<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { commands } from './bindings.ts'
import { useMessage } from 'naive-ui'
import LoginDialog from './components/LoginDialog.vue'
import SearchPane from './panes/SearchPane.vue'
import ChapterPane from './panes/ChapterPane.vue'
import DownloadingPane from './panes/DownloadingPane.vue'
import FavoritePane from './panes/FavoritePane.vue'
import SettingsDialog from './components/SettingsDialog.vue'
import { QuestionCircleOutlined, UserOutlined, SettingOutlined, BarsOutlined } from '@vicons/antd'
import AboutDialog from './components/AboutDialog.vue'
import DownloadedPane from './panes/DownloadedPane.vue'
import { useStore } from './store.ts'
import LogViewer from './components/LogViewer.vue'

const store = useStore()

const message = useMessage()

const loginDialogShowing = ref<boolean>(false)
const settingsDialogShowing = ref<boolean>(false)
const aboutDialogShowing = ref<boolean>(false)
const logViewerShowing = ref<boolean>(false)

watch(
  () => store.config,
  async () => {
    if (store.config === undefined) {
      return
    }
    await commands.saveConfig(store.config)
    message.success('保存配置成功')
  },
  { deep: true },
)
watch(
  () => store.config?.token,
  async () => {
    const result = await commands.getUserProfile()
    if (result.status === 'error') {
      console.error(result.error)
      store.userProfile = undefined
      return
    }
    store.userProfile = result.data
    message.success('获取用户信息成功')
  },
)

onMounted(async () => {
  // 屏蔽浏览器右键菜单
  document.oncontextmenu = (event) => {
    event.preventDefault()
  }
  // 获取配置
  store.config = await commands.getConfig()
})
</script>

<template>
  <div v-if="store.config !== undefined" class="h-screen flex flex-col">
    <div class="flex gap-col-1 pt-2 px-2">
      <n-input-group>
        <n-input-group-label>Authorization</n-input-group-label>
        <n-input v-model:value="store.config.token" placeholder="手动输入或点击右侧的按钮登录" clearable />
        <n-button type="primary" @click="loginDialogShowing = true">
          <template #icon>
            <n-icon>
              <UserOutlined />
            </n-icon>
          </template>
          登录
        </n-button>
      </n-input-group>
      <div v-if="store.userProfile !== undefined" class="flex items-center">
        <n-avatar
          v-if="store.userProfile.avatar !== undefined"
          round
          :size="32"
          :src="`${store.userProfile.avatar.fileServer}/static/${store.userProfile.avatar.path}`"
          fallback-src="https://storage-b.picacomic.com/static/b3411e38-32f2-4ec4-a46c-2edee925dbbd.jpg" />
        <span class="whitespace-nowrap">{{ store.userProfile.name }}</span>
      </div>
    </div>

    <div class="flex overflow-hidden flex-1">
      <n-tabs class="h-full w-1/2" v-model:value="store.currentTabName" type="line" size="small" animated>
        <n-tab-pane class="h-full overflow-auto p-0!" name="search" tab="漫画搜索" display-directive="show">
          <search-pane />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="favorite" tab="漫画收藏" display-directive="show">
          <favorite-pane />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="downloaded" tab="本地库存" display-directive="show">
          <downloaded-pane />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="chapter" tab="章节详情" display-directive="show">
          <chapter-pane />
        </n-tab-pane>
      </n-tabs>

      <div class="w-1/2 overflow-auto flex flex-col">
        <div
          class="h-8.5 flex gap-col-1 mx-2 items-center border-solid border-0 border-b box-border border-[rgb(239,239,245)]">
          <div class="text-xl font-bold box-border">下载列表</div>
          <n-button class="ml-auto" size="small" @click="logViewerShowing = true">
            <template #icon>
              <n-icon>
                <BarsOutlined />
              </n-icon>
            </template>
            日志
          </n-button>
          <n-button size="small" @click="settingsDialogShowing = true">
            <template #icon>
              <n-icon>
                <SettingOutlined />
              </n-icon>
            </template>
            配置
          </n-button>
          <n-button size="small" @click="aboutDialogShowing = true">
            <template #icon>
              <n-icon>
                <QuestionCircleOutlined />
              </n-icon>
            </template>
            关于
          </n-button>
        </div>
        <downloading-pane />
      </div>
    </div>

    <login-dialog v-model:showing="loginDialogShowing" />
    <settings-dialog v-model:showing="settingsDialogShowing" />
    <about-dialog v-model:showing="aboutDialogShowing" />
    <log-viewer v-model:showing="logViewerShowing" />
  </div>
</template>

<style scoped>
:global(.n-notification-main__header) {
  @apply break-words;
}

:global(.n-tabs-pane-wrapper) {
  @apply h-full;
}

:deep(.n-tabs-nav) {
  @apply px-2;
}
</style>
