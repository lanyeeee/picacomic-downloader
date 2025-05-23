<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { commands, events } from '../bindings.ts'
import { open } from '@tauri-apps/plugin-dialog'
import { FolderOpenOutlined } from '@vicons/antd'
import { useStore } from '../store.ts'
import UncompletedProgresses from '../components/UncompletedProgresses.vue'
import CompletedProgresses from '../components/CompletedProgresses.vue'
import { ProgressData } from '../types.ts'

const store = useStore()

const downloadSpeed = ref<string>('')

onMounted(async () => {
  await events.downloadSpeedEvent.listen(async ({ payload: { speed } }) => {
    downloadSpeed.value = speed
  })

  await events.downloadSleepingEvent.listen(async ({ payload: { id, remainingSec } }) => {
    const progressData = store.progresses.get(id)
    if (progressData !== undefined) {
      progressData.indicator = `将在${remainingSec}秒后继续下载`
    }
  })

  await events.downloadTaskEvent.listen(async ({ payload: { event, data } }) => {
    if (event === 'Create') {
      const { chapterInfo, downloadedImgCount, totalImgCount } = data

      store.progresses.set(chapterInfo.chapterId, {
        ...data,
        percentage: 0,
        indicator: `排队中 ${downloadedImgCount}/${totalImgCount}`,
      })
    } else if (event === 'Update') {
      const { chapterId, state, downloadedImgCount, totalImgCount } = data

      const progressData = store.progresses.get(chapterId)
      if (progressData === undefined) {
        return
      }

      progressData.state = state
      progressData.downloadedImgCount = downloadedImgCount
      progressData.totalImgCount = totalImgCount

      if (state === 'Completed') {
        progressData.chapterInfo.isDownloaded = true
        await syncPickedComic()
        await syncComicInSearch(progressData)
        await syncComicInFavorite(progressData)
      }

      progressData.percentage = (downloadedImgCount / totalImgCount) * 100

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

      progressData.indicator = indicator
    }
  })
})

async function syncPickedComic() {
  if (store.pickedComic === undefined) {
    return
  }
  const result = await commands.getSyncedComic(store.pickedComic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  store.pickedComic = result.data
}

async function syncComicInSearch(progressData: ProgressData) {
  if (store.searchResult === undefined) {
    return
  }
  const comic = store.searchResult.docs.find((comic) => comic.id === progressData.comic.id)
  if (comic === undefined) {
    return
  }
  const result = await commands.getSyncedComicInSearch(comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  Object.assign(comic, { ...result.data })
}

async function syncComicInFavorite(progressData: ProgressData) {
  if (store.getFavoriteResult === undefined) {
    return
  }
  const comic = store.getFavoriteResult.docs.find((comic) => comic.id === progressData.comic.id)
  if (comic === undefined) {
    return
  }
  const result = await commands.getSyncedComicInFavorite(comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  Object.assign(comic, { ...result.data })
}

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
  <div class="flex flex-col gap-row-2 flex-1 overflow-auto" v-if="store.config !== undefined">
    <n-input-group class="box-border px-2 pt-2">
      <n-input-group-label size="small">下载目录</n-input-group-label>
      <n-input
        v-model:value="store.config.downloadDir"
        :default-value="0"
        size="small"
        readonly
        @click="selectDownloadDir" />
      <n-button size="small" @click="showDownloadDirInFileManager">
        <template #icon>
          <n-icon>
            <FolderOpenOutlined />
          </n-icon>
        </template>
      </n-button>
    </n-input-group>

    <n-tabs class="h-full overflow-auto" type="line" size="small">
      <n-tab-pane class="h-full p-0! overflow-auto" name="uncompleted" tab="未完成">
        <uncompleted-progresses />
      </n-tab-pane>
      <n-tab-pane class="h-full p-0! overflow-auto" name="completed" tab="已完成">
        <completed-progresses />
      </n-tab-pane>
    </n-tabs>
    <span class="ml-auto mr-2 mb-2">下载速度：{{ downloadSpeed }}</span>
  </div>
</template>

<style scoped>
:deep(.n-tabs-tab) {
  @apply important-py-0.75;
}
</style>
