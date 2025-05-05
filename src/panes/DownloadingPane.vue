<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { commands, events } from '../bindings.ts'
import { open } from '@tauri-apps/plugin-dialog'
import { NProgress } from 'naive-ui'
import { FolderOpenOutlined } from '@vicons/antd'
import { useStore } from '../store.ts'

const store = useStore()

const downloadSpeed = ref<string>('')

onMounted(async () => {
  await events.downloadSpeedEvent.listen(async ({ payload: { speed } }) => {
    downloadSpeed.value = speed
  })

  await events.downloadTaskEvent.listen(({ payload: downloadTaskEvent }) => {
    const { state, chapterInfo, downloadedImgCount, totalImgCount } = downloadTaskEvent

    const percentage = (downloadedImgCount / totalImgCount) * 100

    let indicator = ''
    if (state === 'Pending') {
      indicator = `排队中`
    } else if (state === 'Downloading') {
      indicator = `下载中`
    } else if (state === 'Paused') {
      indicator = `已暂停`
    } else if (state === 'Cancelled') {
      indicator = `已取消`
    } else if (state === 'Completed') {
      indicator = `下载完成`
    } else if (state === 'Failed') {
      indicator = `下载失败`
    }
    if (totalImgCount !== 0) {
      indicator += ` ${downloadedImgCount}/${totalImgCount}`
    }

    const progressData = { ...downloadTaskEvent, percentage, indicator }
    store.progresses.set(chapterInfo.chapterId, progressData)
  })
})

async function showDownloadDirInFileManager() {
  if (store.config === undefined) {
    return
  }

  const result = await commands.showPathInFileManager(store.config.downloadDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}

async function selectDownloadDir() {
  if (store.config === undefined) {
    return
  }

  const selectedDirPath = await open({ directory: true })
  if (selectedDirPath === null) {
    return
  }

  store.config.downloadDir = selectedDirPath
}
</script>

<template>
  <div class="flex flex-col gap-row-2" v-if="store.config !== undefined">
    <n-input-group class="box-border px-2 pt-2">
      <n-input-group-label size="small">下载目录</n-input-group-label>
      <n-input v-model:value="store.config.downloadDir" :default-value="0" size="small" readonly @click="selectDownloadDir" />
      <n-button size="small" @click="showDownloadDirInFileManager">
        <template #icon>
          <n-icon>
            <FolderOpenOutlined />
          </n-icon>
        </template>
      </n-button>
    </n-input-group>

    <div
      class="grid grid-cols-[1fr_4fr] px-2"
      v-for="[chapterId, { chapterInfo, percentage, downloadedImgCount, totalImgCount }] in store.progresses"
      :key="chapterId">
      <span class="mb-1! text-ellipsis whitespace-nowrap overflow-hidden">{{ chapterInfo.comicTitle }}</span>
      <n-progress class="" :percentage="percentage">{{ downloadedImgCount }}/{{ totalImgCount }}</n-progress>
    </div>
  </div>
</template>
