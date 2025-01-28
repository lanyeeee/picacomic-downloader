<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { Comic, commands, Config, UserProfileDetailRespData } from './bindings.ts'
import { useMessage, useNotification } from 'naive-ui'
import LoginDialog from './components/LoginDialog.vue'
import SearchPane from './panes/SearchPane.vue'
import ChapterPane from './panes/ChapterPane.vue'
import DownloadingPane from './panes/DownloadingPane.vue'
import { appDataDir } from '@tauri-apps/api/path'
import { path } from '@tauri-apps/api'
import FavoritePane from './panes/FavoritePane.vue'

const message = useMessage()
const notification = useNotification()

const config = ref<Config>()
const loginDialogShowing = ref<boolean>(false)
const userProfile = ref<UserProfileDetailRespData>()
const currentTabName = ref<'search' | 'favorite' | 'chapter'>('search')
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
  { deep: true }
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
  }
)

onMounted(async () => {
  // 屏蔽浏览器右键菜单
  document.oncontextmenu = (event) => {
    event.preventDefault()
  }
  // 获取配置
  config.value = await commands.getConfig()
})

async function test() {
  console.log(userProfile.value)
}

async function showConfigInFileManager() {
  const configName = 'config.json'
  const configPath = await path.join(await appDataDir(), configName)
  const result = await commands.showPathInFileManager(configPath)
  if (result.status === 'error') {
    notification.error({ title: '打开配置文件失败', description: result.error })
  }
}

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
  <div v-if="config !== undefined" class="h-screen flex flex-col overflow-auto">
    <div class="flex gap-col-1">
      <n-input v-model:value="config.token" placeholder="" clearable>
        <template #prefix>Authorization：</template>
      </n-input>
      <n-button type="primary" @click="loginDialogShowing = true">账号登录</n-button>
      <n-button @click="showConfigInFileManager">打开配置目录</n-button>
      <n-button @click="test">测试用</n-button>
      <div v-if="userProfile !== undefined" class="flex flex-justify-end">
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
      <!-- TODO: 可以给n-tabs加animated -->
      <n-tabs class="basis-1/2 overflow-auto" v-model:value="currentTabName" type="line" size="small">
        <n-tab-pane class="h-full overflow-auto p-0!" name="search" tab="漫画搜索" display-directive="show:lazy">
          <search-pane :search-by-id="searchById" />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="favorite" tab="漫画收藏" display-directive="show:lazy">
          <favorite-pane :search-by-id="searchById" :current-tab-name="currentTabName" />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="chapter" tab="章节详情" display-directive="show:lazy">
          <chapter-pane v-model:picked-comic="pickedComic" />
        </n-tab-pane>
      </n-tabs>

      <div class="basis-1/2 overflow-auto">
        <downloading-pane v-model:config="config"></downloading-pane>
      </div>
    </div>
    <n-modal v-model:show="loginDialogShowing">
      <login-dialog v-model:showing="loginDialogShowing" v-model:token="config.token" />
    </n-modal>
  </div>
</template>
