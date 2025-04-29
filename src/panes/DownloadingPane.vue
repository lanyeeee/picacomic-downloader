<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { commands, events } from '../bindings.ts'
import { open } from '@tauri-apps/plugin-dialog'
import { NProgress, useNotification } from 'naive-ui'
import { FolderOpenOutlined } from '@vicons/antd'
import { useStore } from '../store.ts'

type ProgressData = {
  title: string
  downloadedCount: number
  total: number
  percentage: number
  indicator: string
}

const store = useStore()

const notification = useNotification()

const progresses = ref<Map<string, ProgressData>>(new Map())
const overallProgress = ref<ProgressData>({
  title: '总进度',
  downloadedCount: 0,
  total: 0,
  percentage: 0,
  indicator: '',
})

onMounted(async () => {
  await events.downloadEvent.listen(({ payload }) => {
    if (payload.event === 'ChapterPending') {
      let progressData: ProgressData = {
        title: `等待中 ${payload.data.title}`,
        downloadedCount: 0,
        total: 0,
        percentage: 0,
        indicator: '',
      }
      progresses.value.set(payload.data.chapterId, progressData)
    } else if (payload.event === 'ChapterStart') {
      const progressData = progresses.value.get(payload.data.chapterId) as ProgressData | undefined
      if (progressData === undefined) {
        return
      }
      progressData.total = payload.data.total
      progressData.title = payload.data.title
    } else if (payload.event === 'ImageSuccess') {
      const progressData = progresses.value.get(payload.data.chapterId) as ProgressData | undefined
      if (progressData === undefined) {
        return
      }
      progressData.downloadedCount = payload.data.downloadedCount
      progressData.percentage = Math.round((progressData.downloadedCount / progressData.total) * 100)
    } else if (payload.event === 'ImageError') {
      const progressData = progresses.value.get(payload.data.chapterId) as ProgressData | undefined
      if (progressData === undefined) {
        return
      }
      notification.warning({
        title: '下载图片失败',
        description: payload.data.url,
        content: payload.data.errMsg,
        meta: progressData.title,
      })
    } else if (payload.event === 'ChapterEnd') {
      const progressData = progresses.value.get(payload.data.chapterId) as ProgressData | undefined
      if (progressData === undefined) {
        return
      }
      if (payload.data.errMsg !== null) {
        notification.warning({ title: '下载章节失败', content: payload.data.errMsg, meta: progressData.title })
      }
      progresses.value.delete(payload.data.chapterId)
    } else if (payload.event === 'OverallUpdate') {
      overallProgress.value.percentage = payload.data.percentage
      overallProgress.value.downloadedCount = payload.data.downloadedImageCount
      overallProgress.value.total = payload.data.totalImageCount
    } else if (payload.event === 'Speed') {
      overallProgress.value.indicator = payload.data.speed
    }
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
    <div class="h-8.5 text-xl flex items-center font-bold px-2">下载列表</div>
    <n-input-group class="box-border px-2">
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

    <div class="grid grid-cols-[1fr_4fr_2fr] h-7 items-center px-2">
      <span class="text-ellipsis whitespace-nowrap overflow-hidden">{{ overallProgress.title }}</span>
      <n-progress :percentage="overallProgress.percentage" indicator-placement="inside" :height="21">
        {{ overallProgress.indicator }}
      </n-progress>
      <span>{{ overallProgress.downloadedCount }}/{{ overallProgress.total }}</span>
    </div>
    <div
      class="grid grid-cols-[1fr_4fr] px-2"
      v-for="[chapterId, { title, percentage, downloadedCount, total }] in progresses"
      :key="chapterId">
      <span class="mb-1! text-ellipsis whitespace-nowrap overflow-hidden">{{ title }}</span>
      <n-progress class="" :percentage="percentage">{{ downloadedCount }}/{{ total }}</n-progress>
    </div>
  </div>
</template>
