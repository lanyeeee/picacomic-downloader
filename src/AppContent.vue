<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { Comic, commands, Config, UserProfileDetailRespData } from './bindings.ts'
import { useMessage, useNotification } from 'naive-ui'
import LoginDialog from './components/LoginDialog.vue'
import SearchPane from './panes/SearchPane.vue'
import ChapterPane from './panes/ChapterPane.vue'
import DownloadingPane from './panes/DownloadingPane.vue'
import FavoritePane from './panes/FavoritePane.vue'
import SettingsDialog from './components/SettingsDialog.vue'
import { QuestionCircleOutlined, UserOutlined, SettingOutlined } from '@vicons/antd'
import AboutDialog from './components/AboutDialog.vue'
import { CurrentTabName } from './types.ts'
import DownloadedPane from './panes/DownloadedPane.vue'

const message = useMessage()
const notification = useNotification()

const config = ref<Config>()
const loginDialogShowing = ref<boolean>(false)
const settingsDialogShowing = ref<boolean>(false)
const aboutDialogShowing = ref<boolean>(false)
const userProfile = ref<UserProfileDetailRespData>()
const currentTabName = ref<CurrentTabName>('search')
const pickedComic = ref<Comic>()

watch(
  config,
  async () => {
    if (config.value === undefined) {
      return
    }
    await commands.saveConfig(config.value)
    message.success('保存配置成功')
  },
  { deep: true },
)
watch(
  () => config.value?.token,
  async () => {
    const result = await commands.getUserProfile()
    if (result.status === 'error') {
      notification.error({ title: '获取用户信息失败', description: result.error })
      userProfile.value = undefined
      return
    }
    userProfile.value = result.data
    message.success('获取用户信息成功')
  },
)

onMounted(async () => {
  // 屏蔽浏览器右键菜单
  document.oncontextmenu = (event) => {
    event.preventDefault()
  }
  // 获取配置
  config.value = await commands.getConfig()
})

async function searchById(comicId: string) {
  if (comicId === '') {
    message.warning('漫画ID不能为空')
    return
  }
  const result = await commands.getComic(comicId)
  if (result.status === 'error') {
    notification.error({ title: '获取章节详情失败', description: result.error })
    return
  }
  pickedComic.value = result.data
  currentTabName.value = 'chapter'
}
</script>

<template>
  <div v-if="config !== undefined" class="h-screen flex flex-col">
    <div class="flex gap-col-1 pt-2 px-2">
      <n-input-group>
        <n-input-group-label>Authorization</n-input-group-label>
        <n-input v-model:value="config.token" placeholder="手动输入或点击右侧的按钮登录" clearable />
        <n-button type="primary" @click="loginDialogShowing = true">
          <template #icon>
            <n-icon>
              <UserOutlined />
            </n-icon>
          </template>
          登录
        </n-button>
      </n-input-group>
      <n-button @click="settingsDialogShowing = true">
        <template #icon>
          <n-icon>
            <SettingOutlined />
          </n-icon>
        </template>
        配置
      </n-button>
      <n-button @click="aboutDialogShowing = true">
        <template #icon>
          <n-icon>
            <QuestionCircleOutlined />
          </n-icon>
        </template>
        关于
      </n-button>
      <div v-if="userProfile !== undefined" class="flex items-center">
        <n-avatar
          v-if="userProfile.avatar !== undefined"
          round
          :size="32"
          :src="`${userProfile.avatar.fileServer}/static/${userProfile.avatar.path}`"
          fallback-src="https://storage-b.picacomic.com/static/b3411e38-32f2-4ec4-a46c-2edee925dbbd.jpg" />
        <span class="whitespace-nowrap">{{ userProfile.name }}</span>
      </div>
    </div>

    <div class="flex overflow-hidden flex-1">
      <n-tabs class="h-full w-1/2" v-model:value="currentTabName" type="line" size="small" animated>
        <n-tab-pane class="h-full overflow-auto p-0!" name="search" tab="漫画搜索" display-directive="show">
          <search-pane :search-by-id="searchById" />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="favorite" tab="漫画收藏" display-directive="show">
          <favorite-pane :search-by-id="searchById" :current-tab-name="currentTabName" />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="downloaded" tab="本地库存" display-directive="show">
          <downloaded-pane v-model:picked-comic="pickedComic" v-model:current-tab-name="currentTabName" />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="chapter" tab="章节详情" display-directive="show">
          <chapter-pane v-model:picked-comic="pickedComic" />
        </n-tab-pane>
      </n-tabs>

      <div class="w-1/2 overflow-auto">
        <downloading-pane v-model:config="config"></downloading-pane>
      </div>
    </div>

    <login-dialog v-model:showing="loginDialogShowing" v-model:token="config.token" />
    <settings-dialog v-model:showing="settingsDialogShowing" v-model:config="config" />
    <about-dialog v-model:showing="aboutDialogShowing" />
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
